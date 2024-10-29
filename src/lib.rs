pub use crate::utils::eigenlayer::*;
use crate::utils::tangle::update_session_key;
pub use crate::utils::tangle::{run_tangle_validator, BalanceTransferContext};
use color_eyre::eyre::Result;
use gadget_sdk::config::GadgetConfiguration;
use gadget_sdk::event_listener::tangle::{TangleEvent, TangleEventListener};
use gadget_sdk::{info, job};
use std::convert::Infallible;

pub mod utils;

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

        // return if event.stop() {
        //     info!("Successfully stopped job");
        //     Ok(0)
        // } else {
        //     info!("Failed to stop job");
        //     Ok(1)
        // }
    }
    Ok(0)
}

// pub async fn balance_transfer_pre_processor(
//     event: TangleEvent<BalanceTransferContext>,
// ) -> Result<Option<TangleEvent<BalanceTransferContext>>, Infallible> {
//     if let Some(balance_transfer) = event
//         .evt
//         .as_event::<gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::balances::events::Transfer>()
//         .ok()
//         .flatten()
//     {
//         info!("Balance Transfer Event Found: {:?} sent {:?} tTNT to {:?}", balance_transfer.from.to_string(), balance_transfer.amount, balance_transfer.to.to_string());
//         Ok(Some(event))
//     } else {
//         Ok(None)
//     }
// }

pub async fn tangle_avs_registration(
    env: &GadgetConfiguration<parking_lot::RawRwLock>,
    _context: BalanceTransferContext,
) -> Result<(), gadget_sdk::Error> {
    info!("TANGLE AVS REGISTRATION HOOK");

    // info!("Registering to EigenLayer");
    // register_to_eigenlayer(&env.clone()).await?;

    // Run Tangle Validator
    run_tangle_validator().await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(4)).await; // Let Node start up

    // proxy_and_stash(&env.clone()).await.map_err(|e| gadget_sdk::Error::Job {
    //     reason: "Stash or Proxy failed".to_string(),
    // })?;

    // Rotate Keys and Update Session Key
    update_session_key(&env.clone()).await.unwrap();

    // Validate!

    // info!("Registering to Tangle");
    // register_operator_to_tangle(&self.env.clone()).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::sol_imports::*;
    use alloy_provider::Provider;
    use blueprint_test_utils::inject_test_keys;
    use blueprint_test_utils::test_ext::NAME_IDS;
    use gadget_sdk::config::protocol::TangleInstanceSettings;
    use gadget_sdk::config::{ContextConfig, GadgetCLICoreSettings, Protocol};
    use gadget_sdk::ext::sp_core;
    use gadget_sdk::ext::sp_core::Pair;
    use gadget_sdk::ext::subxt::tx::Signer;
    use gadget_sdk::keystore::backend::fs::FilesystemKeystore;
    use gadget_sdk::keystore::backend::GenericKeyStore;
    use gadget_sdk::keystore::{Backend, BackendExt};
    use gadget_sdk::runners::tangle::TangleConfig;
    use gadget_sdk::runners::BlueprintRunner;
    use gadget_sdk::{error, info};
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

        std::env::set_var(
            "REGISTRY_COORDINATOR_ADDR",
            REGISTRY_COORDINATOR_ADDR.to_string(),
        );
        std::env::set_var(
            "OPERATOR_STATE_RETRIEVER_ADDR",
            OPERATOR_STATE_RETRIEVER_ADDR.to_string(),
        );
        std::env::set_var(
            "DELEGATION_MANAGER_ADDR",
            DELEGATION_MANAGER_ADDR.to_string(),
        );
        std::env::set_var("STRATEGY_MANAGER_ADDR", STRATEGY_MANAGER_ADDR.to_string());

        // let url = Url::parse(ws_endpoint.clone().as_str()).ok().unwrap();
        let http_tangle_url = Url::parse("http://127.0.0.1:9948").unwrap();
        let ws_tangle_url = Url::parse("ws://127.0.0.1:9948").unwrap();
        let bind_port = ws_tangle_url.clone().port().unwrap();

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
        let operator_keystore_uri = keystore_paths[5].clone();
        let operator_keystore = gadget_sdk::keystore::backend::fs::FilesystemKeystore::open(
            operator_keystore_uri.clone(),
        )
        .unwrap();
        let transfer_destination = operator_keystore.sr25519_key().unwrap().account_id();

        let bob_keystore_uri = keystore_paths[1].clone();
        let bob_keystore =
            gadget_sdk::keystore::backend::fs::FilesystemKeystore::open(bob_keystore_uri).unwrap();
        let transfer_signer = bob_keystore.sr25519_key().unwrap();

        let config = ContextConfig {
            gadget_core_settings: GadgetCLICoreSettings::Run {
                bind_addr: IpAddr::from_str("127.0.0.1").unwrap(),
                bind_port,
                test_mode: false,
                log_id: None,
                http_rpc_url: http_tangle_url,
                bootnodes: None,
                keystore_uri: operator_keystore_uri,
                chain: gadget_io::SupportedChains::LocalTestnet,
                verbose: 3,
                pretty: true,
                keystore_password: None,
                blueprint_id: Some(0),
                service_id: Some(0),
                skip_registration: Some(true),
                protocol: Protocol::Tangle,
                registry_coordinator: Some(REGISTRY_COORDINATOR_ADDR),
                operator_state_retriever: Some(OPERATOR_STATE_RETRIEVER_ADDR),
                delegation_manager: Some(DELEGATION_MANAGER_ADDR),
                ws_rpc_url: ws_tangle_url,
                strategy_manager: Some(STRATEGY_MANAGER_ADDR),
                avs_directory: Some(AVS_DIRECTORY_ADDR),
                operator_registry: None,
                network_registry: None,
                base_delegator: None,
                network_opt_in_service: None,
                vault_opt_in_service: None,
                slasher: None,
                veto_slasher: None,
            },
        };
        let env = gadget_sdk::config::load(config).expect("Failed to load environment");

        let client = env.client().await.unwrap();
        let transfer_client = client.clone();
        let signer = env.first_sr25519_signer().unwrap();

        let transfer_task = async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            // Add Proxy
            let transfer_tx = gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::tx()
                .balances()
                .transfer_allow_death(transfer_destination.into(), 100000000000000000000);
            match gadget_sdk::tx::tangle::send(&transfer_client, &transfer_signer, &transfer_tx)
                .await
            {
                Ok(result) => {
                    info!("Transfer Result: {:?}", result);
                }
                Err(e) => {
                    error!("Balance Transfer Error: {:?}", e);
                }
            }
        };
        let _transfer_handle = tokio::task::spawn(transfer_task);

        info!("Starting the event watcher for {} ...", signer.account_id());

        let context = BalanceTransferContext {
            client: client.clone(),
            address: Default::default(),
        };

        tangle_avs_registration(&env.clone(), context.clone())
            .await
            .unwrap();

        let tangle_settings = env.protocol_specific.tangle().unwrap();
        let TangleInstanceSettings { service_id, .. } = tangle_settings;

        let tangle_avs = RegisterToTangleEventHandler {
            service_id: *service_id,
            context: context.clone(),
            client,
            signer,
        };

        info!("~~~ Executing the Tangle AVS ~~~");
        let tangle_config = TangleConfig {
            price_targets: Default::default(),
        };
        BlueprintRunner::new(tangle_config, env.clone())
            .job(tangle_avs)
            .run()
            .await
            .unwrap();

        info!("Exiting...");
    }

    async fn setup_tangle_avs_environment() -> Vec<String> {
        // Set some environment variables with some random seeds for testing
        std::env::set_var(
            "ACCO_SEED",
            "1af56add54dc7e62d68901c26a1323aa2460095c58de1e848a7cd77cc2276aa2",
        ); // SR25519
        std::env::set_var(
            "ACCO_SURI",
            "narrow copper napkin sail outside stadium fabric slice vessel cruel tragic trim",
        );

        std::env::set_var(
            "BABE_SEED",
            "305bd957e3b4483f44ceb51398f527aa5f1a862b02b782a7b5ddcaefdc55a263",
        ); // SR25519
        std::env::set_var(
            "BABE_SURI",
            "accuse dumb company early prison journey jaguar inmate great toy input walnut",
        );

        std::env::set_var(
            "IMON_SEED",
            "42b3e4f84e95f871355ece387416cc974f5dfac60ed6b87e7856e6bc934a967a",
        ); // SR25519
        std::env::set_var(
            "IMON_SURI",
            "decline ethics faculty coast invest two autumn insect arena burden tent cluster",
        );

        std::env::set_var(
            "GRAN_SEED",
            "0d124014884939f7e51379db995910d2603dfb6e36ea58103a45ef1674e866f0",
        ); // ED25519
        std::env::set_var(
            "GRAN_SURI",
            "tobacco indicate globe immense blind fitness home layer furnace luxury level leisure",
        );

        std::env::set_var(
            "ROLE_SEED",
            "eab9b42be4d5a821f0519bd116982da96cc51f2b8ad02cc62230b743a5db9199",
        ); // ECDSA
        std::env::set_var(
            "ROLE_SURI",
            "item near scene turn jelly hamster noise butter move require duty hat",
        );

        // Set up the Keys required for Tangle AVS
        let mut keystore_paths = Vec::new();

        // First we inject the premade Tangle Account keys
        for (item, name) in NAME_IDS.iter().enumerate() {
            let tmp_store = Uuid::new_v4().to_string();
            let keystore_uri = PathBuf::from(format!(
                "./target/keystores/{}/{tmp_store}/",
                name.to_lowercase()
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
            inject_test_keys(&keystore_uri, item)
                .await
                .expect("Failed to inject testing keys for Tangle AVS");
        }

        // Now we create a new Tangle Account for the Test
        let tmp_store = Uuid::new_v4().to_string();
        let keystore_uri = PathBuf::from(format!("./target/keystores/{}/{tmp_store}/", "testnode"));
        assert!(
            !keystore_uri.exists(),
            "Keystore URI cannot exist: {}",
            keystore_uri.display()
        );
        let keystore_uri_normalized =
            std::path::absolute(keystore_uri.clone()).expect("Failed to resolve keystore URI");
        let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());
        keystore_paths.push(keystore_uri_str.clone());

        tokio::fs::create_dir_all(keystore_uri.clone())
            .await
            .unwrap();
        let keystore = GenericKeyStore::<parking_lot::RawRwLock>::Fs(
            FilesystemKeystore::open(keystore_uri_str).unwrap(),
        );

        let acco_suri = std::env::var("ACCO_SURI").expect("ACCO_SURI not set");
        // let suri = format!("//{acco_suri}");
        let (_acco, acco_seed) = sp_core::sr25519::Pair::from_phrase(&acco_suri, None)
            .expect("Should be valid SR keypair");
        info!("Found SR_SEED: {:?}", acco_seed);

        let role_suri = std::env::var("ROLE_SURI").expect("ROLE_SURI not set");
        let (_role, role_seed) = sp_core::ecdsa::Pair::from_phrase(&role_suri, None)
            .expect("Should be valid ECDSA keypair");
        info!("Found ROLE_SEED: {:?}", role_seed);

        keystore
            .sr25519_generate_new(Some(acco_seed.as_ref()))
            .expect("Invalid SR25519 seed");
        keystore
            .ecdsa_generate_new(Some(role_seed.as_ref()))
            .expect("Invalid ECDSA seed");
        keystore
            .bls_bn254_generate_new(None)
            .expect("Random BLS Key Generation Failed");

        keystore_paths
    }
}
