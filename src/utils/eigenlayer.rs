use alloy_primitives::{address, Address, Bytes, FixedBytes, U256};
use color_eyre::eyre::{eyre, Result};
use eigensdk::client_avsregistry::writer::AvsRegistryChainWriter;
use eigensdk::client_elcontracts::reader::ELChainReader;
use eigensdk::client_elcontracts::writer::ELChainWriter;
use eigensdk::crypto_bls::BlsKeyPair;
use eigensdk::logging::get_logger;
use eigensdk::types::operator::Operator;
use gadget_sdk::info;
use gadget_sdk::random::rand::random;
use structopt::lazy_static::lazy_static;

pub const AVS_DIRECTORY_ADDR: Address = address!("0000000000000000000000000000000000000000");
pub const DELEGATION_MANAGER_ADDR: Address = address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
pub const ERC20_MOCK_ADDR: Address = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");
pub const MAILBOX_ADDR: Address = address!("0000000000000000000000000000000000000000");
pub const OPERATOR_STATE_RETRIEVER_ADDR: Address =
    address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
pub const REGISTRY_COORDINATOR_ADDR: Address = address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
pub const SERVICE_MANAGER_ADDR: Address = address!("67d269191c92caf3cd7723f116c85e6e9bf55933");
pub const STRATEGY_MANAGER_ADDR: Address = address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");

lazy_static! {
    /// 1 day
    static ref SIGNATURE_EXPIRY: U256 = U256::from(86400);
}

/// Registers operator to EigenLayer and the AVS.
///
/// # Environment Variables
/// - `EIGENLAYER_HTTP_ENDPOINT`: HTTP endpoint for EigenLayer.
/// - `EIGENLAYER_WS_ENDPOINT`: WebSocket endpoint for EigenLayer.
///
/// # Errors
/// May return errors if:
/// - Environment variables are not set.
/// - Contract interactions fail.
/// - Registration processes encounter issues.
pub async fn register_to_eigenlayer() -> Result<()> {
    eigensdk::logging::init_logger(eigensdk::logging::log_level::LogLevel::Trace);
    let eigen_logger = get_logger();

    // Collect all necessary information from Environment Variables
    let http_endpoint = std::env::var("EIGENLAYER_HTTP_ENDPOINT")
        .map_err(|_| eyre!("EIGENLAYER_HTTP_ENDPOINT must be set"))?;
    let _ws_endpoint = std::env::var("EIGENLAYER_WS_ENDPOINT")
        .map_err(|_| eyre!("EIGENLAYER_WS_ENDPOINT must be set"))?;

    let pvt_key = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    // Create a new provider using the retrieved HTTP Endpoint
    let provider = eigensdk::utils::get_provider(&http_endpoint);

    // TODO: Abstract Slasher retrieval away
    let delegation_manager =
        eigensdk::utils::binding::DelegationManager::new(DELEGATION_MANAGER_ADDR, provider.clone());
    let slasher_address = delegation_manager.slasher().call().await.map(|a| a._0)?;

    let avs_registry_writer = AvsRegistryChainWriter::build_avs_registry_chain_writer(
        eigen_logger.clone(),
        http_endpoint.to_string(),
        pvt_key.to_string(),
        REGISTRY_COORDINATOR_ADDR,
        OPERATOR_STATE_RETRIEVER_ADDR,
    )
    .await
    .expect("avs writer build fail ");

    // TODO: Retrieve BLS Secret Key from Keystore
    let bls_key_pair = BlsKeyPair::new(
        "1371012690269088913462269866874713266643928125698382731338806296762673180359922"
            .to_string(),
    )
    .map_err(|e| eyre!(e))?;

    // A new ElChainReader instance
    let el_chain_reader = ELChainReader::new(
        eigen_logger.clone(),
        slasher_address,
        DELEGATION_MANAGER_ADDR,
        AVS_DIRECTORY_ADDR,
        http_endpoint.to_string(),
    );
    // A new ElChainWriter instance
    let el_writer = ELChainWriter::new(
        DELEGATION_MANAGER_ADDR,
        STRATEGY_MANAGER_ADDR,
        el_chain_reader,
        http_endpoint.to_string(),
        pvt_key.to_string(),
    );

    let operator_addr = address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92266");
    let operator_details = Operator {
        address: operator_addr,
        earnings_receiver_address: operator_addr,
        delegation_approver_address: operator_addr,
        staker_opt_out_window_blocks: 50400u32,
        metadata_url: Some(
            "https://github.com/webb-tools/eigensdk-rs/blob/main/test-utils/metadata.json"
                .to_string(),
        ), // TODO: Metadata should be from Environment Variable
    };

    // Register the address as operator in delegation manager
    el_writer
        .register_as_operator(operator_details)
        .await
        .map_err(|e| eyre!(e))?;

    // Calculate the values necessary for registration
    // First, signature expiry
    let now = std::time::SystemTime::now();
    let sig_expiry = now
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| U256::from(d.as_secs()) + *SIGNATURE_EXPIRY)
        .unwrap_or_else(|| {
            info!("System time seems to be before the UNIX epoch.");
            *SIGNATURE_EXPIRY
        });
    // Quorum numbers and salt
    let quorum_nums = Bytes::from(vec![0]);
    let salt: FixedBytes<32> = FixedBytes::from(random::<[u8; 32]>());

    // Register the operator in registry coordinator
    avs_registry_writer
        .register_operator_in_quorum_with_avs_registry_coordinator(
            bls_key_pair,
            salt,
            sig_expiry,
            quorum_nums,
            http_endpoint.to_string(),
        )
        .await?;
    info!("Registered operator to EigenLayer and AVS");

    Ok(())
}
