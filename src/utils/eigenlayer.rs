use crate::error::Error;
use crate::utils::sol_imports::ECDSAStakeRegistry;
use crate::utils::sol_imports::TangleServiceManager;
use alloy_primitives::{Address, FixedBytes, U256};
use alloy_provider::network::{EthereumWallet, TransactionBuilder};
use alloy_provider::Provider;
use alloy_signer::Signer;
use alloy_signer_local::PrivateKeySigner;
use color_eyre::eyre::Result;
use eigensdk::client_elcontracts::reader::ELChainReader;
use eigensdk::client_elcontracts::writer::ELChainWriter;
use eigensdk::logging::get_test_logger;
use eigensdk::types::operator::Operator;
use gadget_sdk::alloy_rpc_types::BlockNumberOrTag;
use gadget_sdk::config::{GadgetConfiguration, ProtocolSpecificSettings};
use gadget_sdk::keystore::BackendExt;
use gadget_sdk::utils::evm::get_provider_http;
use gadget_sdk::{alloy_rpc_types, info};
use std::str::FromStr;

pub async fn register_to_eigenlayer_and_avs(
    env: &GadgetConfiguration<parking_lot::RawRwLock>,
    tangle_service_manager_addr: Address,
) -> Result<(), Error> {
    let ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &env.protocol_specific else {
        return Err(Error::EigenLayerRegistrationError(
            "Missing EigenLayer contract addresses".into(),
        ));
    };

    let delegation_manager_address = contract_addresses.delegation_manager_address;
    let strategy_manager_address = contract_addresses.strategy_manager_address;
    let avs_directory_address = contract_addresses.avs_directory_address;

    let operator = env
        .keystore()
        .map_err(|e| Error::EigenLayerRegistrationError(e.to_string()))?
        .ecdsa_key()
        .map_err(|e| Error::EigenLayerRegistrationError(e.to_string()))?;
    let operator_private_key = hex::encode(operator.signer().seed());
    let wallet = PrivateKeySigner::from_str(&operator_private_key)
        .map_err(|_| Error::EigenLayerRegistrationError("Invalid private key".into()))?;
    let operator_address = operator
        .alloy_key()
        .map_err(|e| Error::EigenLayerRegistrationError(e.to_string()))?
        .address();
    let provider = get_provider_http(&env.http_rpc_endpoint);

    let delegation_manager = eigensdk::utils::binding::DelegationManager::new(
        delegation_manager_address,
        provider.clone(),
    );
    let slasher_address = delegation_manager
        .slasher()
        .call()
        .await
        .map(|a| a._0)
        .map_err(|e| Error::EigenLayerRegistrationError(e.to_string()))?;

    let logger = get_test_logger();
    let el_chain_reader = ELChainReader::new(
        logger,
        slasher_address,
        delegation_manager_address,
        avs_directory_address,
        env.http_rpc_endpoint.clone(),
    );

    let el_writer = ELChainWriter::new(
        delegation_manager_address,
        strategy_manager_address,
        el_chain_reader.clone(),
        env.http_rpc_endpoint.clone(),
        operator_private_key.clone(),
    );

    let staker_opt_out_window_blocks = 50400u32;
    let operator_details = Operator {
        address: operator_address,
        earnings_receiver_address: operator_address,
        delegation_approver_address: operator_address,
        metadata_url: Some("https://github.com/tangle-network/gadget".to_string()),
        staker_opt_out_window_blocks,
    };

    let tx_hash = el_writer
        .register_as_operator(operator_details)
        .await
        .map_err(|e| Error::EigenLayerRegistrationError(e.to_string()))?;
    info!("Registered as operator for Eigenlayer {:?}", tx_hash);

    let digest_hash_salt: FixedBytes<32> = FixedBytes::from([0x02; 32]);
    let now = std::time::SystemTime::now();
    let sig_expiry = now
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| U256::from(duration.as_secs()) + U256::from(86400))
        .unwrap_or_else(|_| {
            info!("System time seems to be before the UNIX epoch.");
            U256::from(0)
        });

    let msg_to_sign = el_chain_reader
        .calculate_operator_avs_registration_digest_hash(
            operator_address,
            tangle_service_manager_addr,
            digest_hash_salt,
            sig_expiry,
        )
        .await
        .map_err(|_| Error::EigenLayerRegistrationError("Failed to calculate hash".to_string()))?;

    let operator_signature = wallet
        .sign_hash(&msg_to_sign)
        .await
        .map_err(|e| Error::SignerError(e.to_string()))?;

    let operator_signature_with_salt_and_expiry = ECDSAStakeRegistry::SignatureWithSaltAndExpiry {
        signature: operator_signature.as_bytes().into(),
        salt: digest_hash_salt,
        expiry: sig_expiry,
    };

    let signer = alloy_signer_local::PrivateKeySigner::from_str(&operator_private_key)
        .map_err(|e| Error::SignerError(e.to_string()))?;
    let wallet = EthereumWallet::from(signer);

    // --- Register the operator to AVS ---

    // Get the stake registry address to register the operator
    let tangle_service_manager =
        TangleServiceManager::new(tangle_service_manager_addr, provider.clone());
    let stake_registry = tangle_service_manager
        .stakeRegistry()
        .call()
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?
        ._0;

    info!("Building Transaction");

    let latest_block_number = provider
        .get_block_number()
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?;

    // Get the latest block to estimate gas price
    let latest_block = provider
        .get_block_by_number(BlockNumberOrTag::Number(latest_block_number), false)
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?
        .ok_or(Error::TransactionError(
            "Failed to get latest block".into(),
        ))?;

    // Get the base fee per gas from the latest block
    let base_fee_per_gas =
        latest_block
            .header
            .base_fee_per_gas
            .ok_or(Error::TransactionError(
                "Failed to get base fee per gas from latest block".into(),
            ))?;

    // Get the max priority fee per gas
    let max_priority_fee_per_gas = provider
        .get_max_priority_fee_per_gas()
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?;

    // Calculate max fee per gas
    let max_fee_per_gas = base_fee_per_gas + max_priority_fee_per_gas;

    // Build the transaction request
    let tx = alloy_rpc_types::TransactionRequest::default()
        .with_call(&ECDSAStakeRegistry::registerOperatorWithSignatureCall {
            _operatorSignature: operator_signature_with_salt_and_expiry,
            _signingKey: operator_address,
        })
        .with_from(operator_address)
        .with_to(stake_registry)
        .with_nonce(
            provider
                .get_transaction_count(operator_address)
                .await
                .map_err(|e| Error::TransactionError(e.to_string()))?,
        )
        .with_chain_id(
            provider
                .get_chain_id()
                .await
                .map_err(|e| Error::TransactionError(e.to_string()))?,
        )
        .with_max_priority_fee_per_gas(max_priority_fee_per_gas)
        .with_max_fee_per_gas(max_fee_per_gas);

    // Estimate gas limit
    let gas_estimate = provider
        .estimate_gas(&tx)
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?;
    info!("Gas Estimate: {}", gas_estimate);

    // Set gas limit
    let tx = tx.with_gas_limit(gas_estimate);

    info!("Building Transaction Envelope");

    let tx_envelope = tx
        .build(&wallet)
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?;

    info!("Sending Transaction Envelope");

    let result = provider
        .send_tx_envelope(tx_envelope)
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?
        .register()
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?;

    info!("Operator Registration to AVS Sent. Awaiting Receipt...");

    info!("Operator Address: {}", operator_address);
    info!("Stake Registry Address: {}", stake_registry);
    info!("RPC Endpoint: {}", env.http_rpc_endpoint);

    let tx_hash = result
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?;

    info!(
        "Command for testing: cast code {} --rpc-url {}",
        stake_registry, env.http_rpc_endpoint
    );

    let receipt = provider
        .get_transaction_receipt(tx_hash)
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?
        .ok_or(Error::TransactionError(
            "Failed to get receipt".into(),
        ))?;
    info!("Got Transaction Receipt: {:?}", receipt);

    if !receipt.status() {
        return Err(Error::EigenLayerRegistrationError(
            "Failed to register operator to AVS".to_string(),
        ));
    }
    info!("Operator Registration to AVS Succeeded");
    Ok(())
}
