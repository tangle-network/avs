use crate::error::Error;
// use crate::utils::sol_imports::ECDSAStakeRegistry::{
//     registerOperatorWithSignatureCall, ECDSAStakeRegistryCalls,
// };
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
use eigensdk::utils::binding::ECDSAStakeRegistry;
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
        return Err(Error::EigenLayerRegistration(
            "Missing EigenLayer contract addresses".into(),
        ));
    };

    let delegation_manager_address = contract_addresses.delegation_manager_address;
    let strategy_manager_address = contract_addresses.strategy_manager_address;
    let avs_directory_address = contract_addresses.avs_directory_address;

    let operator = env
        .keystore()
        .map_err(|e| Error::EigenLayerRegistration(e.to_string()))
        .unwrap()
        .ecdsa_key()
        .map_err(|e| Error::EigenLayerRegistration(e.to_string()))
        .unwrap();
    let operator_private_key = hex::encode(operator.signer().seed());
    let wallet = PrivateKeySigner::from_str(&operator_private_key)
        .map_err(|_| Error::EigenLayerRegistration("Invalid private key".into()))
        .unwrap();
    let operator_address = operator
        .alloy_key()
        .map_err(|e| Error::EigenLayerRegistration(e.to_string()))
        .unwrap()
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
        .map_err(|e| Error::EigenLayerRegistration(e.to_string()))
        .unwrap();

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
        .map_err(|e| Error::EigenLayerRegistration(e.to_string()))
        .unwrap();
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
        .unwrap();
    // .map_err(|_| Error::EigenLayerRegistration("Failed to calculate hash".to_string())).unwrap();

    let operator_signature = wallet
        .sign_hash(&msg_to_sign)
        .await
        .map_err(|_| Error::EigenLayerRegistration("Invalid Signature".to_string()))
        .unwrap();

    let operator_signature_with_salt_and_expiry = ECDSAStakeRegistry::SignatureWithSaltAndExpiry {
        signature: operator_signature.as_bytes().into(),
        salt: digest_hash_salt,
        expiry: sig_expiry,
    };

    // Register the operator to AVS
    let tangle_service_manager =
        TangleServiceManager::new(tangle_service_manager_addr, provider.clone());
    let stake_registry = tangle_service_manager
        .stakeRegistry()
        .call()
        .await
        .map_err(|e| Error::EigenLayerRegistration(e.to_string()))
        .unwrap()
        ._0;
    // let ecdsa_stake_registry = ECDSAStakeRegistry::new(stake_registry, provider.clone());
    // let register_call = ecdsa_stake_registry
    //     .registerOperatorWithSignature(operator_address, operator_signature_with_salt_and_expiry)
    //     .from(operator_address);

    // let register_call_data = ecdsa_stake_registry
    //     .registerOperatorWithSignature(operator_signature_with_salt_and_expiry, operator_address)
    //     .calldata();

    info!("Building Transaction");

    let tx = alloy_rpc_types::TransactionRequest::default()
        .with_call(&ECDSAStakeRegistry::registerOperatorWithSignatureCall {
            _operator: operator_address,
            _operatorSignature: operator_signature_with_salt_and_expiry,
        })
        .with_from(operator_address)
        .with_to(stake_registry)
        .with_nonce(
            provider
                .get_transaction_count(operator_address)
                .await
                .unwrap(),
        )
        .with_chain_id(provider.get_chain_id().await.unwrap())
        .with_gas_limit(21_000)
        .with_max_priority_fee_per_gas(1_000_000_000)
        .with_max_fee_per_gas(20_000_000_000);

    let signer = alloy_signer_local::PrivateKeySigner::from_str(&operator_private_key).unwrap();
    let wallet = EthereumWallet::from(signer);

    info!("Building Transaction Envelope");

    let tx_envelope = tx.build(&wallet).await.unwrap();

    info!("Sending Transaction Envelope");

    let result = provider
        .send_tx_envelope(tx_envelope)
        .await
        .unwrap()
        .register()
        .await
        .unwrap();
    // let receipt = provider.get_transaction_receipt(result).await.unwrap().unwrap();

    info!("Operator Registration to AVS Sent. Awaiting Receipt...");

    let tx_hash = result.await.unwrap();

    info!("Got Transaction Hash: {}", tx_hash);

    let receipt = provider
        .get_transaction_receipt(tx_hash)
        .await
        .unwrap()
        .unwrap();
    info!("Got Transaction Receipt: {:?}", receipt);

    if !receipt.status() {
        return Err(Error::EigenLayerRegistration(
            "Failed to register operator to AVS".to_string(),
        ));
    }
    info!("Operator Registration to AVS Succeeded");
    Ok(())
}
