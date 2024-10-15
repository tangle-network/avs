use alloy_primitives::Address;
use async_trait::async_trait;
use color_eyre::eyre::{eyre, Result};
use gadget_sdk::clients::tangle::runtime::{TangleClient, TangleConfig};
use gadget_sdk::event_listener::EventListener;
use gadget_sdk::events_watcher::substrate::EventHandlerFor;
use gadget_sdk::executor::process::manager::GadgetProcessManager;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::balances;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::balances::events::Transfer;
use gadget_sdk::{info, Error};
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

pub async fn register_node_to_tangle() -> Result<()> {
    // TODO: Abstracted logic to handle registration of node to Tangle

    Ok(())
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
    let chain = "tangle-testnet";
    let name = "TESTNODE";
    let validator = "--validator";
    let telemetry_url = "\"wss://telemetry.polkadot.io/submit/ 1\"";

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
