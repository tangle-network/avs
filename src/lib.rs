use crate::utils::tangle::{bond_balance, update_session_key};
pub use crate::utils::tangle::{run_tangle_validator, BalanceTransferContext};
use color_eyre::eyre::Result;
use gadget_sdk::event_listener::tangle::{TangleEvent, TangleEventListener};
use gadget_sdk::{info, job};
use std::convert::Infallible;

pub mod utils;
pub mod error;

#[cfg(test)]
mod tests;

/// Listens for a balance transfer into the specified account, after which it registers as
/// an operator with the provided user information.
#[job(
    id = 0,
    event_listener(
        listener = TangleEventListener<BalanceTransferContext>,
        // pre_processor = balance_transfer_pre_processor,
    )
)]
pub async fn register_to_tangle(
    event: TangleEvent<BalanceTransferContext>,
    context: BalanceTransferContext,
) -> Result<u64, Infallible> {
    if let Some(balance_transfer) = event
        .evt
        .as_event::<gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::balances::events::Transfer>()
        .ok()
        .flatten()
    {
        info!("Balance Transfer Event Found: {:?} sent {:?} tTNT to {:?}", balance_transfer.from.to_string(), balance_transfer.amount, balance_transfer.to.to_string());
        match tangle_avs_registration(context.clone()).await {
            Ok(_) => {
                info!("Successfully registered Tangle Validator");
            }
            Err(err) => {
                gadget_sdk::error!("Failed to register Tangle Validator: {}", err);
                return Ok(1);
            }
        }

        return if event.stop() {
            info!("Successfully stopped job");
            Ok(1)
        } else {
            info!("Failed to stop job");
            Ok(2)
        }
    }
    Ok(0)
}

/// Registers the Tangle AVS Operator to Tangle.
/// - Runs the Tangle Node
/// - Bonds Balance
/// - Rotates keys
/// - Updates Session Key
pub async fn tangle_avs_registration(
    context: BalanceTransferContext,
) -> Result<(), gadget_sdk::Error> {
    info!("TANGLE AVS REGISTRATION");
    let env = context.env.clone();

    // Run Tangle Validator
    run_tangle_validator(context.env.keystore_uri.as_str())
        .await
        .map_err(|e| gadget_sdk::Error::Job {
            reason: e.to_string(),
        })?;

    bond_balance(&env.clone())
        .await
        .map_err(|e| gadget_sdk::Error::Job {
            reason: e.to_string(),
        })?;

    // Rotate Keys and Update Session Key
    update_session_key(&env.clone())
        .await
        .map_err(|e| gadget_sdk::Error::Job {
            reason: e.to_string(),
        })?;

    // Validate

    Ok(())
}
