use alloy_primitives::Address;
use async_trait::async_trait;
use color_eyre::eyre::{eyre, Result};
use futures::TryFutureExt;
use gadget_sdk::clients::tangle::runtime::{TangleClient, TangleConfig};
use gadget_sdk::config::GadgetConfiguration;
use gadget_sdk::event_listener::EventListener;
use gadget_sdk::events_watcher::substrate::EventHandlerFor;
use gadget_sdk::executor::process::manager::GadgetProcessManager;
use gadget_sdk::ext::subxt_signer::sr25519::{PublicKey, SecretKeyBytes};
use gadget_sdk::keystore::sp_core_subxt::Pair;
use gadget_sdk::keystore::Backend;
use gadget_sdk::subxt_core::utils::AccountId32;
use gadget_sdk::tangle_subxt::subxt::tx::Signer;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::balances;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::balances::events::Transfer;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::proxy::calls::types::add_proxy::{
    Delay, Delegate, ProxyType,
};
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::staking::calls::types;
use gadget_sdk::{info, tx, Error};
use std::os::unix::fs::PermissionsExt;
use tokio::sync::broadcast;
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;

#[derive(Clone)]
pub struct BalanceTransferContext {
    pub client: TangleClient,
    pub address: Address,
    pub handler: EventHandlerFor<TangleConfig, balances::events::Transfer>,
}

pub struct TangleBalanceTransferListener {
    client: TangleClient,
    address: Address,
    handler: EventHandlerFor<TangleConfig, balances::events::Transfer>,
}

#[async_trait]
impl EventListener<Vec<balances::events::Transfer>, BalanceTransferContext>
    for TangleBalanceTransferListener
{
    async fn new(context: &BalanceTransferContext) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Ok(Self {
            client: context.client.clone(),
            address: context.address,
            handler: context.handler.clone(),
        })
    }

    async fn next_event(&mut self) -> Option<Vec<balances::events::Transfer>> {
        loop {
            let events = self.client.events().at_latest().await.ok()?;
            let transfers = events
                .find::<balances::events::Transfer>()
                .flatten()
                .filter(|evt| evt.to.0.as_slice() == self.address.0.as_slice())
                .collect::<Vec<_>>();
            if !transfers.is_empty() {
                return Some(transfers);
            }
        }
    }

    async fn handle_event(&mut self, event: Vec<balances::events::Transfer>) -> Result<(), Error> {
        const MAX_RETRY_COUNT: usize = 5;
        info!("Processing {} Balance Transfer Event(s)", event.len(),);

        // We only care if we got at least one transfer event
        let transfer = event
            .first()
            .cloned()
            .ok_or(Error::Other("Failed to get transfer event".to_string()))?;
        let Transfer {
            from: transfer_from,
            to: transfer_to,
            amount: transfer_amount,
        } = transfer.clone();
        info!("Transfer Amount: {}", transfer_amount);
        info!("Transfer Source: {}", transfer_from);
        info!("Transfer Target: {}", transfer_to);

        let handler = &self.handler;

        // Create and await the task
        let task = async move {
            let backoff = ExponentialBackoff::from_millis(2)
                .factor(1000)
                .take(MAX_RETRY_COUNT);

            Retry::spawn(backoff, || async {
                let result = handler.handle(&transfer).await?;
                if result.is_empty() {
                    Err(Error::Other("Task handling failed".to_string()))
                } else {
                    Ok(())
                }
            })
            .await
        };
        let result = task.await;

        if let Err(e) = result {
            gadget_sdk::error!("Error while handling event: {e:?}");
        } else {
            info!("Event handled successfully");
        }

        Ok(())
    }
}

pub async fn register_operator_to_tangle(
    env: &GadgetConfiguration<parking_lot::RawRwLock>,
) -> Result<()> {
    // Register Session Key with the Network for the Node
    // curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys", "params":[]}' http://localhost:bind_port

    let client = env.client().await.map_err(|e| eyre!(e))?;
    let ecdsa_pair = env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
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

pub async fn register_node_to_tangle() -> Result<()> {
    // TODO: Abstracted logic to handle registration of node to Tangle

    Ok(())
}

pub async fn generate_keys() -> Result<String> {
    let mut manager = GadgetProcessManager::new();

    // Key Generation Commands
    let commands = vec![
        "key insert --base-path test --chain local --scheme Sr25519 --suri \"\" --key-type acco",
        "key insert --base-path test --chain local --scheme Sr25519 --suri \"\" --key-type babe",
        "key insert --base-path test --chain local --scheme Sr25519 --suri \"\" --key-type imon",
        "key insert --base-path test --chain local --scheme Ecdsa --suri \"\" --key-type role",
        "key insert --base-path test --chain local --scheme Ed25519 --suri \"\" --key-type gran",
    ];
    // Execute each command
    for (index, cmd) in commands.iter().enumerate() {
        info!("Running: {}", cmd);
        let service_name = format!("generate_key_{}", index);
        let full_command = format!("./tangle-default-linux-amd64 {}", cmd);

        let service = manager
            .run(service_name, &full_command)
            .await
            .map_err(|e| eyre!("Failed to start service: {}", e))?;

        manager
            .focus_service_to_completion(service)
            .await
            .map_err(|e| eyre!("Service failed: {}", e))?;
    }

    info!("Generating Node Key...");
    // ./tangle-default-linux-amd64 key generate-node-key --file test/node-key

    // Execute the node-key generation command and capture its output
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
pub async fn run_tangle_validator() -> Result<broadcast::Receiver<String>> {
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

    let base_path = "path/to/executable/";
    let chain = "local";
    let name = "TESTNODE";
    let validator = "--validator";
    let telemetry_url = "\"wss://telemetry.polkadot.io/submit/ 1\"";
    let rpc_port = "9944";

    // ./tangle-default-linux-amd64 key insert --base-path test --chain local --scheme Sr25519 --suri "" --key-type acco
    // ./tangle-default-linux-amd64 key insert --base-path test --chain local --scheme Sr25519 --suri "" --key-type babe
    // ./tangle-default-linux-amd64 key insert --base-path test --chain local --scheme Sr25519 --suri "" --key-type imon
    // ./tangle-default-linux-amd64 key insert --base-path test --chain local --scheme Ecdsa --suri "" --key-type role
    // ./tangle-default-linux-amd64 key insert --base-path test --chain local --scheme Ed25519 --suri "" --key-type gran
    // ./tangle-default-linux-amd64 key generate-node-key --file test/node-key                    -- outputs key
    //

    // let proxy_public_key = "0x0000000000000000000000000000000000000000000000000000000000000000";
    //
    // // Add Proxy
    // let xt = api::tx().proxy().add_proxy(
    //     Delegate::from(proxy_public_key),
    //     ProxyType::NonTransfer,
    //     Delay::from(0),
    // );
    //
    // // send the tx to the tangle and exit.
    // let result = tx::tangle::send(&client, &signer, &xt).await?;
    // info!("Registered operator with hash: {:?}", result);

    let start_node_command = format!(
        "./tangle-default-linux-amd64 \
    --base-path {base_path} \
    --chain {chain} \
    --name {name} \
    {validator} \
    --telemetry-url {telemetry_url}\
    --rpc-port {rpc_port} \
    "
    );

    // Start the validator
    let validator_stream = manager
        .start_process_and_get_output("tangle_validator".into(), start_node_command.as_str())
        .await
        .map_err(|e| eyre!(e.to_string()))?;
    Ok(validator_stream)
}

pub const TANGLE_AVS_ASCII: &str = r#"
 _____   _     _    _  _____ _      _____
|_   _| / \    | \ | |/ ____| |    |  ___|
  | |  / _ \   |  \| | |  __| |    | |__
  | | / ___ \  | . ` | | |_ | |    |  __|
  | |/ /   \ \ | |\  | |__| | |____| |___
  |_/_/     \_\|_| \_|\_____|______|_____|

              _   __     __ ____
             / \  \ \   / // ___|
            / _ \  \ \ / / \__ \
           / ___ \  \ V /  ___) |
          /_/   \_\  \_/  |____/
"#;
