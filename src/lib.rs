pub use crate::utils::eigenlayer::*;
use crate::utils::tangle::{generate_keys, register_operator_to_tangle};
pub use crate::utils::tangle::{run_tangle_validator, BalanceTransferContext};
use color_eyre::eyre::Result;
use gadget_sdk::event_listener::tangle::{TangleEvent, TangleEventListener};
use gadget_sdk::job;
use std::convert::Infallible;

pub mod utils;

// TODO: Replace params and result, we want to listen for balance on our account
/// Listens for a balance transfer into the specified account, after which it registers as
/// an operator with the provided user information.
#[job(
    id = 0,
    event_listener(
        listener = TangleEventListener<BalanceTransferContext>)
)]
// TODO: Switch from u64 to tangle_subxt::tangle_testnet_runtime::api::balances::events::Transfer. It can't currently due to lack of conversion from event to inputs
pub fn register_to_tangle(
    event: TangleEvent<BalanceTransferContext>,
    context: BalanceTransferContext,
) -> Result<u64, Infallible> {
    // Register, now that we have balance
    Ok(0)
}

pub async fn tangle_avs_registration(
    // env: &GadgetConfiguration<parking_lot::RawRwLock>,
    context: BalanceTransferContext,
) -> Result<(), gadget_sdk::Error> {
    // if env.test_mode {
    //     info!("Skipping registration in test mode");
    //     return Ok(());
    // }

    let node_key = generate_keys().await.map_err(|e| gadget_sdk::Error::Job {
        reason: "Failed to generate node key".to_string(),
    })?;

    // info!("Registering to EigenLayer");
    // register_to_eigenlayer(&env.clone()).await?;

    // info!("Registering to Tangle");
    // register_operator_to_tangle(&self.env.clone()).await?;

    Ok(())
}

// pub struct TangleGadgetRunner {
//     pub env: GadgetConfiguration<parking_lot::RawRwLock>,
// }
//
// #[async_trait::async_trait]
// impl GadgetRunner for TangleGadgetRunner {
//     type Error = color_eyre::eyre::Report;
//
//     fn config(&self) -> &StdGadgetConfiguration {
//         todo!()
//     }
//
//     async fn register(&mut self) -> Result<()> {
//         if self.env.test_mode {
//             info!("Skipping registration in test mode");
//             return Ok(());
//         }
//
//         let node_key = generate_keys().await?;
//
//         info!("Registering to EigenLayer");
//         register_to_eigenlayer(&self.env.clone()).await?;
//
//         // info!("Registering to Tangle");
//         // register_operator_to_tangle(&self.env.clone()).await?;
//
//         Ok(())
//     }
//
//     async fn benchmark(&self) -> std::result::Result<(), Self::Error> {
//         todo!()
//     }
//
//     async fn run(&mut self) -> Result<()> {
//         info!("Executing Run Function in Gadget Runner...");
//
//         // Run Tangle Validator
//         // let _tangle_stream = run_tangle_validator().await?; // We need to return necessary values
//         // tokio::time::sleep(std::time::Duration::from_secs(4)).await;
//
//         // Run Tangle Event Listener, waiting for balance in our account so that we can register
//         let client = self.env.client().await.map_err(|e| eyre!(e))?;
//         let signer = self.env.first_sr25519_signer().map_err(|e| eyre!(e))?;
//
//         info!("Starting the event watcher for {} ...", signer.account_id());
//
//         // let register_to_tangle = RegisterToTangleEventHandler {
//         //     context: BalanceTransferContext {
//         //         client: client.clone(),
//         //         address: Default::default(),
//         //         handler: Arc::new(()),
//         //     },
//         //     service_id: self.env.service_id.ok_or_eyre("No service id provided")?,
//         //     signer,
//         //     client,
//         // };
//         //
//         // let finished_rx = register_to_tangle
//         //     .init_event_handler()
//         //     .await
//         //     .expect("Event Listener init already called");
//         // let res = finished_rx.await;
//         // gadget_sdk::error!("Event Listener finished with {res:?}");
//
//         Ok(())
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::sol_imports::*;
    use crate::utils::tangle;
    use alloy_provider::Provider;
    use blueprint_test_utils::inject_test_keys;
    use blueprint_test_utils::test_ext::NAME_IDS;
    use gadget_sdk::config::{ContextConfig, GadgetCLICoreSettings, Protocol};
    use gadget_sdk::ext::subxt::tx::Signer;
    use gadget_sdk::info;
    use gadget_sdk::job_runner::{JobBuilder, MultiJobRunner};
    use std::net::IpAddr;
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::time::Duration;
    use url::Url;
    use uuid::Uuid;

    const ANVIL_STATE_PATH: &str = "./saved_testnet_state.json";

    #[tokio::test]
    async fn test_full_tangle_avs() {
        gadget_sdk::logging::setup_log();

        // Begin the Anvil Testnet
        let (_container, http_endpoint, ws_endpoint) =
            blueprint_test_utils::anvil::start_anvil_container(ANVIL_STATE_PATH, false).await;
        std::env::set_var("EIGENLAYER_HTTP_ENDPOINT", http_endpoint.clone());
        std::env::set_var("EIGENLAYER_WS_ENDPOINT", ws_endpoint.clone());
        std::env::set_var("REGISTRATION_MODE_ON", "true");

        // let url = Url::parse(ws_endpoint.clone().as_str()).ok().unwrap();
        let url = Url::parse("ws://127.0.0.1:9948").unwrap();
        let bind_port = url.clone().port().unwrap();

        // Sleep to give the testnet time to spin up
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Create a provider using the transport for the Anvil Testnet
        let provider = alloy_provider::ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(http_endpoint.parse().unwrap())
            .root()
            .clone()
            .boxed();

        // Get the accounts
        let accounts = provider.get_accounts().await.unwrap();
        info!("Accounts: {:?}", accounts);

        // Create a Registry Coordinator instance and then use it to create a quorum
        let registry_coordinator =
            RegistryCoordinator::new(REGISTRY_COORDINATOR_ADDR, provider.clone());
        let operator_set_params = RegistryCoordinator::OperatorSetParam {
            maxOperatorCount: 10,
            kickBIPsOfOperatorStake: 100,
            kickBIPsOfTotalStake: 1000,
        };
        let strategy_params = RegistryCoordinator::StrategyParams {
            strategy: ERC20_MOCK_ADDR,
            multiplier: 1,
        };
        let _ = registry_coordinator
            .createQuorum(operator_set_params, 0, vec![strategy_params])
            .send()
            .await
            .unwrap();

        // Retrieve the stake registry address from the registry coordinator
        let stake_registry_addr = registry_coordinator
            .stakeRegistry()
            .call()
            .await
            .unwrap()
            ._0;
        info!("Stake Registry Address: {:?}", stake_registry_addr);

        // Deploy the Tangle Service Manager to the running Anvil Testnet
        let tangle_service_manager_addr = TangleServiceManager::deploy_builder(
            provider.clone(),
            AVS_DIRECTORY_ADDR,
            stake_registry_addr,
            REGISTRY_COORDINATOR_ADDR, // TODO: Needs to be updated to PaymentCoordinator?
            DELEGATION_MANAGER_ADDR,
            MAILBOX_ADDR,
        )
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap()
        .contract_address
        .unwrap();

        // Make a Tangle Service Manager instance
        let _tangle_service_manager =
            TangleServiceManager::new(tangle_service_manager_addr, provider.clone());
        info!(
            "Tangle Service Manager Address: {:?}",
            tangle_service_manager_addr
        );

        let keystore_paths = setup_tangle_avs_environment().await;
        let alice_keystore = keystore_paths[0].clone();

        let config = ContextConfig {
            gadget_core_settings: GadgetCLICoreSettings::Run {
                bind_addr: IpAddr::from_str("127.0.0.1").unwrap(),
                bind_port,
                test_mode: false,
                log_id: None,
                url,
                bootnodes: None,
                keystore_uri: alice_keystore,
                chain: gadget_io::SupportedChains::LocalTestnet,
                verbose: 3,
                pretty: true,
                keystore_password: None,
                blueprint_id: 0,
                service_id: Some(0),
                protocol: Protocol::Tangle,
            },
        };
        let env = gadget_sdk::config::load(config).expect("Failed to load environment");
        // let mut runner = Box::new(TangleGadgetRunner { env: env.clone() });
        //
        // info!("~~~ Executing the Tangle AVS ~~~");
        //
        // info!("Registering...");
        // // Register the operator if needed
        // if env.should_run_registration() {
        //     runner.register().await.unwrap();
        // }
        //
        // info!("Running...");
        // // Run the gadget / AVS
        // runner.run().await.unwrap();
        //
        // info!("Exiting...");

        let client = env.client().await.unwrap();
        let signer = env.first_sr25519_signer().unwrap();

        info!("Starting the event watcher for {} ...", signer.account_id());

        let context = BalanceTransferContext {
            client: client.clone(),
            address: Default::default(),
        };

        let tangle_avs = RegisterToTangleEventHandler {
            service_id: env.service_id.unwrap(),
            context: context.clone(),
            client,
            signer,
        };

        info!("~~~ Executing the Tangle AVS ~~~");
        MultiJobRunner::new(env)
            .job(JobBuilder::new(tangle_avs).registration(context, tangle_avs_registration))
            .run()
            .await
            .unwrap();

        info!("Exiting...");
    }

    async fn setup_tangle_avs_environment() -> Vec<String> {
        // Set up the Keys required for Tangle AVS
        let mut keystore_paths = Vec::new();
        for (name, item) in NAME_IDS.iter().enumerate() {
            let tmp_store = Uuid::new_v4().to_string();
            let keystore_uri = PathBuf::from(format!(
                "./target/keystores/{}/{tmp_store}/",
                item.to_lowercase()
            ));
            assert!(
                !keystore_uri.exists(),
                "Keystore URI cannot exist: {}",
                keystore_uri.display()
            );
            let keystore_uri_normalized =
                std::path::absolute(keystore_uri.clone()).expect("Failed to resolve keystore URI");
            let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());
            keystore_paths.push(keystore_uri_str);
            inject_test_keys(&keystore_uri, name)
                .await
                .expect("Failed to inject testing keys for Tangle AVS");
        }
        keystore_paths
    }
}
