use crate::utils::tangle::run_tangle_validator;
use color_eyre::eyre::{eyre, OptionExt, Result};
use gadget_sdk::{
    config::{GadgetConfiguration, StdGadgetConfiguration},
    events_watcher::InitializableEventHandler,
    run::GadgetRunner,
    subxt_core::tx::signer::Signer,
};
use gadget_sdk::{info, job};
use std::convert::Infallible;
pub use utils::eigenlayer::*;

pub mod utils;

// TODO: Replace params and result, we want to listen for balance on our account
/// Listens for a balance transfer into the specified account, after which it registers as
/// an operator with the provided user information.
#[job(id = 0, params(x), result(_), event_listener(TangleEventListener))]
pub fn register_to_tangle(x: u64) -> Result<u64, Infallible> {
    // Register, now that we have balance

    Ok(0)
}

pub struct TangleGadgetRunner {
    pub env: GadgetConfiguration<parking_lot::RawRwLock>,
}

#[async_trait::async_trait]
impl GadgetRunner for TangleGadgetRunner {
    type Error = color_eyre::eyre::Report;

    fn config(&self) -> &StdGadgetConfiguration {
        todo!()
    }

    async fn register(&mut self) -> Result<()> {
        if self.env.test_mode {
            info!("Skipping registration in test mode");
            return Ok(());
        }

        info!("Registering to EigenLayer");
        register_to_eigenlayer().await?;

        info!("Registering to Tangle");

        Ok(())
    }

    async fn benchmark(&self) -> std::result::Result<(), Self::Error> {
        todo!()
    }

    async fn run(&mut self) -> Result<()> {
        info!("Executing Run Function in Gadget Runner...");

        // Run Tangle Validator
        run_tangle_validator().await?; // We need to return necessary values

        // Run Tangle Event Listener, waiting for balance in our account so that we can register
        let client = self.env.client().await.map_err(|e| eyre!(e))?;
        let signer = self.env.first_sr25519_signer().map_err(|e| eyre!(e))?;

        info!("Starting the event watcher for {} ...", signer.account_id());

        let register_to_tangle = RegisterToTangleEventHandler {
            service_id: self.env.service_id.ok_or_eyre("No service id provided")?,
            signer,
            client,
        };

        let finished_rx = register_to_tangle
            .init_event_handler()
            .await
            .expect("Event Listener init already called");
        let res = finished_rx.await;
        gadget_sdk::error!("Event Listener finished with {res:?}");

        Ok(())
    }
}
