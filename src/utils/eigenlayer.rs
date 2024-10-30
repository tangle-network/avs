use alloy_primitives::{address, Address, Bytes};
use color_eyre::eyre::{eyre, Result};
use gadget_sdk::config::GadgetConfiguration;
use gadget_sdk::keystore::BackendExt;
use gadget_sdk::utils::evm::{register_as_operator, register_with_avs_registry_coordinator};

pub const AVS_DIRECTORY_ADDR: Address = address!("0000000000000000000000000000000000000000");
pub const DELEGATION_MANAGER_ADDR: Address = address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
pub const ERC20_MOCK_ADDR: Address = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");
pub const MAILBOX_ADDR: Address = address!("0000000000000000000000000000000000000000");
pub const OPERATOR_STATE_RETRIEVER_ADDR: Address =
    address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
pub const REGISTRY_COORDINATOR_ADDR: Address = address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
pub const SERVICE_MANAGER_ADDR: Address = address!("67d269191c92caf3cd7723f116c85e6e9bf55933");
pub const STRATEGY_MANAGER_ADDR: Address = address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");

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
pub async fn register_to_eigenlayer(
    env: &GadgetConfiguration<parking_lot::RawRwLock>,
) -> Result<()> {
    // Read keys from keystore
    let keystore = env.keystore()?;
    let bls_key_pair = keystore.bls_bn254_key().map_err(|e| eyre!(e))?;
    let ecdsa_pair = keystore.ecdsa_key().map_err(|e| eyre!(e))?;

    // Collect all necessary information from Environment Variables
    let http_endpoint = std::env::var("EIGENLAYER_HTTP_ENDPOINT")
        .map_err(|_| eyre!("EIGENLAYER_HTTP_ENDPOINT must be set"))?;
    let _ws_endpoint = std::env::var("EIGENLAYER_WS_ENDPOINT")
        .map_err(|_| eyre!("EIGENLAYER_WS_ENDPOINT must be set"))?;

    let _register_hash = register_as_operator(
        DELEGATION_MANAGER_ADDR,
        AVS_DIRECTORY_ADDR,
        STRATEGY_MANAGER_ADDR,
        &http_endpoint.clone(),
        ecdsa_pair.clone(),
    )
    .await
    .map_err(|e| eyre!(e))?;

    let _register_hash = register_with_avs_registry_coordinator(
        &http_endpoint.clone(),
        ecdsa_pair,
        bls_key_pair,
        Bytes::from(vec![0]),
        REGISTRY_COORDINATOR_ADDR,
        OPERATOR_STATE_RETRIEVER_ADDR,
    )
    .await?;

    Ok(())
}
