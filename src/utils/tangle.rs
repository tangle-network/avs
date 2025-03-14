use color_eyre::eyre::Result;
use gadget_sdk::clients::tangle::runtime::TangleClient;
use gadget_sdk::config::GadgetConfiguration;
use gadget_sdk::executor::process::manager::GadgetProcessManager;
use gadget_sdk::ext::sp_core::hexdisplay::AsBytesRef;
use gadget_sdk::tangle_subxt::parity_scale_codec::DecodeAll;
use gadget_sdk::tangle_subxt::subxt::backend::rpc::RpcClient;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::session::calls::types::set_keys::{
    Keys, Proof,
};
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::staking::calls::types;
use gadget_sdk::{info, trace, tx};
use crate::error::Error;
use std::os::unix::fs::PermissionsExt;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_arithmetic::per_things::Perbill;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::staking::calls::types::validate::Prefs;
use tokio::process::Command;
use url::Url;

#[derive(Clone)]
pub struct BalanceTransferContext {
    pub client: TangleClient,
    pub env: GadgetConfiguration<parking_lot::RawRwLock>,
}

/// Bonds balance for the Operator specified in the [`GadgetConfiguration`].
///
/// # Note
/// This function does not currently utilize a proxy account.
pub async fn bond_balance(env: &GadgetConfiguration<parking_lot::RawRwLock>) -> Result<(), Error> {
    let client = env
        .client()
        .await
        .map_err(|e| Error::TangleRegistrationError(e.to_string()))?;
    let _ecdsa_pair = env
        .first_ecdsa_signer()
        .map_err(|e| Error::SignerError(e.to_string()))?;
    let sr25519_pair = env
        .first_sr25519_signer()
        .map_err(|e| Error::SignerError(e.to_string()))?;

    // // ---------- Add Proxy Account ----------
    // let add_proxy_tx = api::tx().proxy().add_proxy(
    //     Delegate::from(account_id.clone()),
    //     ProxyType::NonTransfer,
    //     Delay::from(0u32),
    // );
    // let result = tx::tangle::send(&client, &sr25519_pair, &add_proxy_tx).await?;
    // info!("Add Proxy Result: {:?}", result);

    // ---------- Bonding ----------
    info!("Bonding...");
    let bond_stash_tx = api::tx().staking().bond(
        types::bond::Value::from(100_000_000_000_000_000u128),
        types::bond::Payee::Stash,
    );
    let result = tx::tangle::send(&client, &sr25519_pair, &bond_stash_tx)
        .await
        .map_err(|e| Error::TangleRegistrationError(e.to_string()))?;
    info!("Stash Account Bonding Result: {:?}", result);

    Ok(())
}

/// Update the session key for the Operator specified in the [`GadgetConfiguration`]
pub async fn update_session_key(
    env: &GadgetConfiguration<parking_lot::RawRwLock>,
) -> Result<(), Error> {
    let tangle_client = env
        .client()
        .await
        .map_err(|e| Error::TangleRegistrationError(e.to_string()))?;
    let _ecdsa_pair = env
        .first_ecdsa_signer()
        .map_err(|e| Error::SignerError(e.to_string()))?;
    let sr25519_pair = env
        .first_sr25519_signer()
        .map_err(|e| Error::SignerError(e.to_string()))?;
    let http_endpoint =
        Url::parse(&env.target_endpoint_http()).map_err(|e| Error::InvalidUrl(e.to_string()))?;
    let ws_endpoint =
        Url::parse(&env.target_endpoint_ws()).map_err(|e| Error::InvalidUrl(e.to_string()))?;

    // First, rotate keys
    let client = reqwest::Client::new();
    let body = r#"{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys", "params":[]}"#;

    let response = client
        .post(http_endpoint)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| Error::HttpRequestError(e.to_string()))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| Error::JsonError(e.to_string()))?;

    // Extract the "result" value
    let result = json["result"]
        .as_str()
        .ok_or_else(|| {
            Error::JsonError("Failed to extract 'result' from JSON response".to_string())
        })?
        .to_string();

    info!("Result: {:?}", result);

    let session_keys =
        gadget_sdk::tangle_subxt::subxt::backend::legacy::rpc_methods::LegacyRpcMethods::<
            gadget_sdk::clients::tangle::runtime::TangleConfig,
        >::new(
            RpcClient::from_url(ws_endpoint)
                .await
                .map_err(|e| Error::TangleRegistrationError(e.to_string()))?,
        )
        .author_rotate_keys()
        .await
        .map_err(|e| Error::TangleRegistrationError(e.to_string()))?;
    if session_keys.len() != 96 {
        return Err(Error::SessionKeyError);
    }

    // Split the session_keys into individual keys
    let babe_bytes = &session_keys[0..32];
    let grandpa_bytes = &session_keys[32..64];
    let im_online_bytes = &session_keys[64..96];

    // Log the keys for verification
    info!("BABE key: 0x{}", hex::encode(babe_bytes));
    info!("GRANDPA key: 0x{}", hex::encode(grandpa_bytes));
    info!("IMONLINE key: 0x{}", hex::encode(im_online_bytes));

    // Construct the keys as a tuple of encoded bytes
    let keys = Keys {
        babe: runtime_types::sp_consensus_babe::app::Public::decode_all(
            &mut babe_bytes.to_vec().as_bytes_ref(),
        )
        .map_err(|e| Error::TangleRegistrationError(e.to_string()))?,
        grandpa: runtime_types::sp_consensus_grandpa::app::Public::decode_all(
            &mut grandpa_bytes.to_vec().as_bytes_ref(),
        )
        .map_err(|e| Error::TangleRegistrationError(e.to_string()))?,
        im_online: runtime_types::pallet_im_online::sr25519::app_sr25519::Public::decode_all(
            &mut im_online_bytes.to_vec().as_bytes_ref(),
        )
        .map_err(|e| Error::TangleRegistrationError(e.to_string()))?,
    };

    // Create the set_keys call
    let set_session_key_tx = api::tx().session().set_keys(keys, Proof::from(Vec::new()));

    // Send the transaction
    let result = tx::tangle::send(&tangle_client, &sr25519_pair, &set_session_key_tx)
        .await
        .map_err(|e| Error::TangleRegistrationError(e.to_string()))?;

    info!("Session keys set successfully. Result: {:?}", result);

    Ok(())
}

/// Declares the desire to validate for the Operator specified in the [`GadgetConfiguration`].
///
/// Effects are not felt until the beginning of the next era.
pub async fn validate(env: &GadgetConfiguration<parking_lot::RawRwLock>) -> Result<(), Error> {
    let client = env
        .client()
        .await
        .map_err(|e| Error::TangleRegistrationError(e.to_string()))?;
    let sr25519_pair = env
        .first_sr25519_signer()
        .map_err(|e| Error::SignerError(e.to_string()))?;

    let start_validation = api::tx().staking().validate(Prefs {
        commission: Perbill(5),
        blocked: false,
    });
    let result = tx::tangle::send(&client, &sr25519_pair, &start_validation)
        .await
        .map_err(|e| Error::TangleRegistrationError(e.to_string()))?;
    info!("Start Validation Result: {:?}", result);

    Ok(())
}

/// Generates keys for a Tangle node
///
/// # Returns
/// - The generated Node Key as a [`String`]
///
/// # Arguments
/// - `base_path`: The base path of the location for the keys to be stored at
/// - `chain`: The type of chain (local, testnet, mainnet)
///
/// # Errors
/// - Fails if any of the required environment variables are not set
/// - If any key generation commands fail
///
pub async fn generate_keys(base_path: &str) -> Result<String, Error> {
    let mut manager = GadgetProcessManager::new();

    let acco_seed =
        std::env::var("ACCO_SURI").map_err(|e| Error::EnvironmentVariableError(e.to_string()))?;
    let babe_seed =
        std::env::var("BABE_SURI").map_err(|e| Error::EnvironmentVariableError(e.to_string()))?;
    let imon_seed =
        std::env::var("IMON_SURI").map_err(|e| Error::EnvironmentVariableError(e.to_string()))?;
    let gran_seed =
        std::env::var("GRAN_SURI").map_err(|e| Error::EnvironmentVariableError(e.to_string()))?;
    let role_seed =
        std::env::var("ROLE_SURI").map_err(|e| Error::EnvironmentVariableError(e.to_string()))?;

    // Key Generation Commands
    // TODO: Update base-path and chain to be variables
    let commands = [
        &format!("key insert --base-path {base_path} --chain local --scheme Sr25519 --suri \"//{acco_seed}\" --key-type acco"),
        &format!("key insert --base-path {base_path} --chain local --scheme Sr25519 --suri \"//{babe_seed}\" --key-type babe"),
        &format!("key insert --base-path {base_path} --chain local --scheme Sr25519 --suri \"//{imon_seed}\" --key-type imon"),
        &format!("key insert --base-path {base_path} --chain local --scheme Ecdsa --suri \"//{role_seed}\" --key-type role"),
        &format!("key insert --base-path {base_path} --chain local --scheme Ed25519 --suri \"//{gran_seed}\" --key-type gran"),
    ];
    // Execute each command
    for (index, cmd) in commands.iter().enumerate() {
        trace!("Running: {}", cmd);
        let service_name = format!("generate_key_{}", index);
        let full_command = format!("./tangle-default-linux-amd64 {}", cmd);
        let service = manager
            .run(service_name, &full_command)
            .await
            .map_err(|e| Error::JobError(e.to_string()))?;
        let _output = manager
            .focus_service_to_completion(service)
            .await
            .map_err(|e| Error::JobError(e.to_string()))?;
    }

    // Execute the node-key generation command and capture its output
    trace!("Generating Node Key...");
    let node_path = format!("{base_path}/node-key");
    info!("Node key path: {}", node_path);
    let output = Command::new("./tangle-default-linux-amd64")
        .args(["key", "generate-node-key", "--file", &node_path])
        .output()
        .await
        .map_err(|e| Error::CommandError(e.to_string()))?;
    if !output.status.success() {
        return Err(Error::CommandError(format!(
            "Command failed with code: {:?}",
            output.status.code()
        )));
    }
    let node_key = String::from_utf8(output.stderr)
        .map_err(|e| Error::Utf8Error(e.to_string()))?
        .trim()
        .to_string();
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
pub async fn run_tangle_validator(keystore_base_path: &str) -> Result<(), Error> {
    let keystore_base_path = keystore_base_path.trim_start_matches("file:");

    let mut manager = GadgetProcessManager::new();

    info!("Keystore Base Path: {}", keystore_base_path);

    // Check if the binary exists
    if !std::path::Path::new("tangle-default-linux-amd64").exists() {
        let install = manager
            .run("binary_install".to_string(), "wget https://github.com/tangle-network/tangle/releases/download/v1.0.0/tangle-default-linux-amd64")
            .await
            .map_err(|e| Error::JobError(e.to_string()))?;
        manager
            .focus_service_to_completion(install)
            .await
            .map_err(|e| Error::JobError(e.to_string()))?;
    }

    // Check if the binary is executable
    let metadata = std::fs::metadata("tangle-default-linux-amd64")
        .map_err(|e| Error::IoError(e.to_string()))?;
    let permissions = metadata.permissions();
    if !permissions.mode() & 0o111 != 0 {
        let make_executable = manager
            .run(
                "make_executable".to_string(),
                "chmod +x tangle-default-linux-amd64",
            )
            .await
            .map_err(|e| Error::JobError(e.to_string()))?;
        manager
            .focus_service_to_completion(make_executable)
            .await
            .map_err(|e| Error::JobError(e.to_string()))?;
    }

    let _node_key = generate_keys(keystore_base_path)
        .await
        .map_err(|e| Error::JobError(e.to_string()))?;

    let chain = "local";
    let name = "TESTNODE";
    let validator = "--validator";
    let telemetry_url = "\"wss://telemetry.polkadot.io/submit/ 1\"";
    let _rpc_port = "9948";

    let start_node_command = format!(
        "./tangle-default-linux-amd64 \
    --base-path {keystore_base_path} \
    --chain {chain} \
    --name {name} \
    {validator} \
    --telemetry-url {telemetry_url}\
    "
    );

    // Start the validator
    let _validator_task = tokio::spawn(async move {
        let _validator_stream = manager
            .run("tangle_validator".into(), start_node_command.as_str())
            .await
            .map_err(|e| Error::JobError(e.to_string()))
            .unwrap();
        manager
            .focus_service_to_completion("tangle_validator".into())
            .await
            .map_err(|e| Error::JobError(e.to_string()))
            .unwrap();
    });

    Ok(())
}
