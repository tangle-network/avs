use alloy_primitives::Address;
use color_eyre::eyre::{eyre, Result};
use gadget_sdk::clients::tangle::runtime::TangleClient;
use gadget_sdk::config::GadgetConfiguration;
use gadget_sdk::executor::process::manager::GadgetProcessManager;
use gadget_sdk::tangle_subxt::subxt::tx::Signer;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::proxy::calls::types::add_proxy::{
    Delay, Delegate, ProxyType,
};
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::staking::calls::types;
use gadget_sdk::{info, trace, tx};
use std::os::unix::fs::PermissionsExt;
use gadget_sdk::tangle_subxt::parity_scale_codec::DecodeAll;
use gadget_sdk::tangle_subxt::subxt::backend::rpc::RpcClient;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_testnet_runtime::opaque::SessionKeys;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::session::calls::types::set_keys::Proof;
use url::Url;

#[derive(Clone)]
pub struct BalanceTransferContext {
    pub client: TangleClient,
    pub address: Address,
}

pub async fn proxy_and_stash(env: &GadgetConfiguration<parking_lot::RawRwLock>) -> Result<()> {
    let client = env.client().await.map_err(|e| eyre!(e))?;
    let _ecdsa_pair = env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
    let sr25519_pair = env.first_sr25519_signer().map_err(|e| eyre!(e))?;
    let account_id = sr25519_pair.account_id();

    // ---------- Add Proxy ----------
    let add_proxy_tx = api::tx().proxy().add_proxy(
        Delegate::from(account_id.clone()),
        ProxyType::NonTransfer,
        Delay::from(0u32),
    );
    let result = tx::tangle::send(&client, &sr25519_pair, &add_proxy_tx).await?;
    info!("Add Proxy Result: {:?}", result);

    // ---------- Stash Account Bonding ----------
    let bond_stash_tx = api::tx().staking().bond(
        types::bond::Value::from(1000u16),
        types::bond::Payee::Account(account_id), // TODO: Make this not hardcoded?
    );
    let result = tx::tangle::send(&client, &sr25519_pair, &bond_stash_tx).await?;
    info!("Stash Account Bonding Result: {:?}", result);

    Ok(())
}

pub async fn update_session_key(env: &GadgetConfiguration<parking_lot::RawRwLock>) -> Result<()> {
    let tangle_client = env.client().await.map_err(|e| eyre!(e))?;
    let _ecdsa_pair = env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
    let sr25519_pair = env.first_sr25519_signer().map_err(|e| eyre!(e))?;
    let url = Url::parse(&env.http_rpc_endpoint).map_err(|e| eyre!(e))?;

    // First, rotate keys
    let client = reqwest::Client::new();
    let body = r#"{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys", "params":[]}"#;

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?;

    let json: serde_json::Value = response.json().await?;

    // Extract the "result" value
    let result = json["result"]
        .as_str()
        .ok_or_else(|| eyre!("Failed to extract 'result' from JSON response"))?
        .to_string();

    info!("Result: {:?}", result);

    let session_keys =
        gadget_sdk::tangle_subxt::subxt::backend::legacy::rpc_methods::LegacyRpcMethods::<
            gadget_sdk::clients::tangle::runtime::TangleConfig,
        >::new(
            RpcClient::from_url(env.ws_rpc_endpoint.clone())
                .await
                .map_err(|e| eyre!(e))?,
        )
        .author_rotate_keys()
        .await
        .map_err(|e| eyre!(e))?;
    if session_keys.len() != 96 {
        return Err(eyre!("Invalid session key length"));
    }

    let mut babe = &session_keys[0..32];
    let mut grandpa = &session_keys[32..64];
    let mut imonline = &session_keys[64..96];

    // // Set Session Key
    let set_session_key_tx = api::tx().session().set_keys(
        SessionKeys {
            babe: runtime_types::sp_consensus_babe::app::Public::decode_all(&mut babe)?,
            grandpa: runtime_types::sp_consensus_grandpa::app::Public::decode_all(&mut grandpa)?,
            im_online: runtime_types::pallet_im_online::sr25519::app_sr25519::Public::decode_all(
                &mut imonline,
            )?,
        },
        Proof::from(Vec::new()),
    );

    // TODO: This currently fails with a `Metadata(IncompatibleCodegen)` error
    let _result = tx::tangle::send(&tangle_client, &sr25519_pair, &set_session_key_tx).await;

    Ok(())
}

pub async fn register_node_to_tangle() -> Result<()> {
    // TODO: Abstracted logic to handle registration of node to Tangle

    Ok(())
}

pub async fn generate_keys() -> Result<String> {
    let mut manager = GadgetProcessManager::new();

    let acco_seed = std::env::var("ACCO_SURI").expect("ACCO_SURI not set");
    let babe_seed = std::env::var("BABE_SURI").expect("BABE_SURI not set");
    let imon_seed = std::env::var("IMON_SURI").expect("IMON_SURI not set");
    let gran_seed = std::env::var("GRAN_SURI").expect("GRAN_SURI not set");
    let role_seed = std::env::var("ROLE_SURI").expect("ROLE_SURI not set");

    // Key Generation Commands
    // TODO: Update base-path and chain to be variables
    let commands = [
        &format!("key insert --base-path test --chain local --scheme Sr25519 --suri \"//{acco_seed}\" --key-type acco"),
        &format!("key insert --base-path test --chain local --scheme Sr25519 --suri \"//{babe_seed}\" --key-type babe"),
        &format!("key insert --base-path test --chain local --scheme Sr25519 --suri \"//{imon_seed}\" --key-type imon"),
        &format!("key insert --base-path test --chain local --scheme Ecdsa --suri \"//{role_seed}\" --key-type role"),
        &format!("key insert --base-path test --chain local --scheme Ed25519 --suri \"//{gran_seed}\" --key-type gran"),
    ];
    // Execute each command
    for (index, cmd) in commands.iter().enumerate() {
        trace!("Running: {}", cmd);
        let service_name = format!("generate_key_{}", index);
        let full_command = format!("./tangle-default-linux-amd64 {}", cmd);

        let service = manager
            .run(service_name, &full_command)
            .await
            .map_err(|e| eyre!("Failed to start service: {}", e))?;

        let _output = manager
            .focus_service_to_completion(service)
            .await
            .map_err(|e| eyre!("Service failed: {}", e))?;
    }

    // Execute the node-key generation command and capture its output
    trace!("Generating Node Key...");
    let node_key_command =
        "./tangle-default-linux-amd64 key generate-node-key --file test/node-key";
    let mut node_key_output = manager
        .start_process_and_get_output("generate_node_key".into(), node_key_command)
        .await
        .map_err(|e| eyre!("Failed to generate node key: {}", e))?;
    let node_key = node_key_output.recv().await?;
    let node_key = node_key.trim_start_matches("[stderr] ").to_string();
    info!("Node key: {}", node_key);

    Ok(node_key)
}

/// Fetches and runs the Tangle validator binary, initiating a validator node.
///
/// # Process
/// 1. Checks for the existence of the binary.
/// 2. If not found, downloads it from the official Tangle GitHub release page.
/// 3. Ensures the binary has executable permissions.
/// 4. Executes the binary to start the validator node.
///
/// # Errors
/// Returns an error if:
/// - The binary download fails
/// - Setting executable permissions fails
/// - The binary execution fails
pub async fn run_tangle_validator() -> Result<()> {
    let mut manager = GadgetProcessManager::new();

    // Check if the binary exists
    if !std::path::Path::new("tangle-default-linux-amd64").exists() {
        let install = manager
            .run("binary_install".to_string(), "wget https://github.com/webb-tools/tangle/releases/download/v1.0.0/tangle-default-linux-amd64")
            .await
            .map_err(|e| eyre!(e.to_string()))?;
        manager
            .focus_service_to_completion(install)
            .await
            .map_err(|e| eyre!(e.to_string()))?;
    }

    // Check if the binary is executable
    let metadata = std::fs::metadata("tangle-default-linux-amd64")?;
    let permissions = metadata.permissions();
    if !permissions.mode() & 0o111 != 0 {
        let make_executable = manager
            .run(
                "make_executable".to_string(),
                "chmod +x tangle-default-linux-amd64",
            )
            .await
            .map_err(|e| eyre!(e.to_string()))?;
        manager
            .focus_service_to_completion(make_executable)
            .await
            .map_err(|e| eyre!(e.to_string()))?;
    }

    let _node_key = generate_keys().await.map_err(|e| gadget_sdk::Error::Job {
        reason: e.to_string(),
    })?;

    let base_path = "test";
    let chain = "local";
    let name = "TESTNODE";
    let validator = "--validator";
    let telemetry_url = "\"wss://telemetry.polkadot.io/submit/ 1\"";
    let _rpc_port = "9948";

    let start_node_command = format!(
        "./tangle-default-linux-amd64 \
    --base-path {base_path} \
    --chain {chain} \
    --name {name} \
    {validator} \
    --telemetry-url {telemetry_url}\
    "
    );

    // Start the validator
    // TODO: Node is dying or getting stuck for some reason
    let _validator_task = tokio::spawn(async move {
        let _validator_stream = manager
            .run("tangle_validator".into(), start_node_command.as_str())
            .await
            .map_err(|e| eyre!(e.to_string()))
            .unwrap();
        manager
            .focus_service_to_completion("tangle_validator".into())
            .await
            .map_err(|e| eyre!(e.to_string()))
            .unwrap();
    });

    Ok(())
}
