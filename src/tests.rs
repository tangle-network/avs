pub use crate::utils::constants;
use crate::utils::sol_imports::*;
use crate::BalanceTransferContext;
use crate::RegisterToTangleEventHandler;
use alloy_primitives::U256;
use alloy_provider::network::{EthereumWallet, TransactionBuilder};
use alloy_provider::Provider;
use blueprint_test_utils::test_ext::NAME_IDS;
use blueprint_test_utils::{inject_test_keys, KeyGenType};
use eigensdk::utils::binding::ECDSAStakeRegistry;
use eigensdk::utils::binding::ECDSAStakeRegistry::{Quorum, StrategyParams};
use gadget_sdk::config::{ContextConfig, GadgetCLICoreSettings, Protocol};
use gadget_sdk::ext::sp_core;
use gadget_sdk::ext::sp_core::Pair;
use gadget_sdk::ext::subxt::tx::Signer;
use gadget_sdk::keystore::backend::fs::FilesystemKeystore;
use gadget_sdk::keystore::backend::GenericKeyStore;
use gadget_sdk::keystore::{Backend, BackendExt};
use gadget_sdk::runners::eigenlayer::EigenlayerConfig;
use gadget_sdk::runners::BlueprintRunner;
use gadget_sdk::utils::evm::get_provider_http;
use crate::utils::eigenlayer::register_to_eigenlayer_and_avs;
use gadget_sdk::{alloy_rpc_types, error, info};
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use url::Url;
use uuid::Uuid;

const ANVIL_STATE_PATH: &str = "./saved_testnet_state.json";

#[tokio::test]
async fn test_full_tangle_avs() {
    use constants::local::*;
    gadget_sdk::logging::setup_log();

    // Begin the Anvil Testnet
    let (_container, http_endpoint, ws_endpoint) =
        blueprint_test_utils::anvil::start_anvil_container(ANVIL_STATE_PATH, true).await;

    // Sleep to give the testnet time to spin up
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Create a provider using the transport for the Anvil Testnet
    let provider = alloy_provider::ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(http_endpoint.parse().unwrap())
        .root()
        .clone()
        .boxed();

    // Get the anvil accounts
    let accounts = provider.get_accounts().await.unwrap();
    info!("Accounts: {:?}", accounts);

    let ecdsa_stake_registry_addr =
        ECDSAStakeRegistry::deploy_builder(provider.clone(), DELEGATION_MANAGER_ADDR)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap()
            .contract_address
            .unwrap();
    let ecdsa_stake_registry = ECDSAStakeRegistry::new(ecdsa_stake_registry_addr, provider.clone());
    info!(
        "Ecdsa Stake Registry Address: {:?}",
        ecdsa_stake_registry_addr
    );

    // Deploy the Tangle Service Manager to the running Anvil Testnet
    let tangle_service_manager_addr = TangleServiceManager::deploy_builder(
        provider.clone(),
        AVS_DIRECTORY_ADDR,
        ecdsa_stake_registry_addr,
        DELEGATION_MANAGER_ADDR,
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
    let tangle_service_manager =
        TangleServiceManager::new(tangle_service_manager_addr, provider.clone());
    info!(
        "Tangle Service Manager Address: {:?}",
        tangle_service_manager_addr
    );

    // Initialize the Tangle Service Manager
    let init_call = tangle_service_manager.initialize(accounts[0]);
    let result = init_call.send().await.unwrap();
    let receipt = result.get_receipt().await.unwrap();
    assert!(receipt.status());
    info!("Tangle Service Manager Initialization Succeeded");

    // Initialize the ECDSA Stake Registry
    let init_quorum = Quorum {
        strategies: vec![StrategyParams {
            strategy: ERC20_MOCK_ADDR,
            multiplier: 10_000,
        }],
    };
    let init_call =
        ecdsa_stake_registry.initialize(tangle_service_manager_addr, U256::from(1000), init_quorum);
    let result = init_call.send().await.unwrap();
    let receipt = result.get_receipt().await.unwrap();
    info!("ECDSA Stake Registry Initialization Receipt: {:?}", receipt);
    assert!(receipt.status());
    info!("ECDSA Stake Registry Initialization Succeeded");

    // Setup Keystores for test
    set_tangle_env_vars();
    let tmp_dir = tempfile::TempDir::new().unwrap(); // Create a temporary directory for the keystores
    let keystore_paths = generate_tangle_avs_keys(tmp_dir.path()).await;

    // Get the operator's keys
    let operator_keystore_uri = keystore_paths[5].clone();
    let operator_keystore =
        gadget_sdk::keystore::backend::fs::FilesystemKeystore::open(operator_keystore_uri.clone())
            .unwrap();
    let operator_ecdsa_signer = operator_keystore.ecdsa_key().unwrap();
    let operator_signer = operator_keystore.sr25519_key().unwrap();
    let transfer_destination = operator_signer.account_id();

    let private_key_bytes = operator_ecdsa_signer.signer().seed();
    let private_key_hex = hex::encode(private_key_bytes);
    info!("ECDSA Private Key: {}", private_key_hex);
    info!("ECDSA Public Key: {:?}", operator_ecdsa_signer.public());

    // Get Bob's keys, who will transfer money to the operator
    let bob_keystore_uri = keystore_paths[1].clone();
    let bob_keystore =
        gadget_sdk::keystore::backend::fs::FilesystemKeystore::open(bob_keystore_uri).unwrap();
    let transfer_signer = bob_keystore.sr25519_key().unwrap();

    // Transfer balance into operator's account on Anvil for registration
    let provider = get_provider_http(&http_endpoint);
    let alloy_sender = accounts[0];

    let before_transfer = provider.get_balance(alloy_sender).await.unwrap();
    info!("Balance before transfer: {}", before_transfer);

    let anvil_tx_amount = 300_000_000_000_000u64; // Enough to cover registration
    let tx = alloy_rpc_types::TransactionRequest::default()
        .with_from(alloy_sender)
        .with_to(operator_ecdsa_signer.alloy_address().unwrap())
        .with_value(U256::from(anvil_tx_amount))
        .with_nonce(provider.get_transaction_count(alloy_sender).await.unwrap())
        .with_chain_id(provider.get_chain_id().await.unwrap())
        .with_gas_limit(21_000)
        .with_max_priority_fee_per_gas(1_000_000_000)
        .with_max_fee_per_gas(20_000_000_000);

    let funder_signer =
        alloy_signer_local::PrivateKeySigner::from_str("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80").unwrap();
    let funder_wallet = EthereumWallet::from(funder_signer);

    let tx_envelope = tx.build(&funder_wallet).await.unwrap();

    let tx_hash = provider
        .send_tx_envelope(tx_envelope)
        .await
        .unwrap()
        .watch()
        .await
        .unwrap();
    info!(
        "Transferred {anvil_tx_amount} from {:?} to {:?}\n\tHash: {:?}",
        alloy_sender,
        operator_ecdsa_signer.alloy_address(),
        tx_hash
    );

    let after_transfer = provider.get_balance(alloy_sender).await.unwrap();
    info!("Balance after transfer: {}", after_transfer);

    let operator_balance = provider.get_balance(operator_ecdsa_signer.alloy_address().unwrap()).await.unwrap();
    info!("Operator balance: {}", operator_balance);

    // Tangle node url/port
    let ws_tangle_url = Url::parse("ws://127.0.0.1:9948").unwrap();
    let target_port = ws_tangle_url.clone().port().unwrap();

    // Create the GadgetConfiguration
    let config = ContextConfig {
        gadget_core_settings: GadgetCLICoreSettings::Run {
            target_addr: IpAddr::from_str("127.0.0.1").unwrap(),
            target_port,
            use_secure_url: false,
            test_mode: false,
            log_id: None,
            http_rpc_url: Url::parse(&http_endpoint).unwrap(),
            bootnodes: None,
            keystore_uri: operator_keystore_uri,
            chain: gadget_io::SupportedChains::LocalTestnet,
            verbose: 3,
            pretty: true,
            keystore_password: None,
            blueprint_id: Some(0),
            service_id: Some(0),
            skip_registration: true,
            protocol: Protocol::Eigenlayer,
            registry_coordinator: Some(ZERO_ADDRESS), // We don't need this for Tangle AVS
            operator_state_retriever: Some(ZERO_ADDRESS), // We don't need this for Tangle AVS
            delegation_manager: Some(DELEGATION_MANAGER_ADDR),
            ws_rpc_url: Url::parse(&ws_endpoint).unwrap(),
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
    let signer_id = signer.clone().account_id();

    // Spawn task to transfer balance into Operator's account on Tangle
    let transfer_task = async move {
        tokio::time::sleep(Duration::from_secs(4)).await;
        info!(
            "Transferring balance from {:?} to {:?}",
            signer_id, transfer_destination
        );
        let transfer_tx = gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::tx()
            .balances()
            .transfer_allow_death(transfer_destination.into(), 1_000_000_000_000_000_000); // TODO: Adjust this amount as needed
        match gadget_sdk::tx::tangle::send(&transfer_client, &transfer_signer, &transfer_tx).await {
            Ok(result) => {
                info!("Transfer Result: {:?}", result);
            }
            Err(e) => {
                error!("Balance Transfer Error: {:?}", e);
            }
        }
    };
    let _transfer_handle = tokio::task::spawn(transfer_task);

    // Create Instance of the Event Handler
    let context = BalanceTransferContext {
        client: client.clone(),
        env: env.clone(),
    };
    let tangle_avs = RegisterToTangleEventHandler {
        service_id: 0,
        context: context.clone(),
        client,
        signer,
    };

    // Register to Eigenlayer and AVS
    register_to_eigenlayer_and_avs(&env, tangle_service_manager_addr)
        .await
        .unwrap();

    // Start the Runner
    info!("~~~ Executing the Tangle AVS ~~~");
    let eigen_config = EigenlayerConfig {};
    BlueprintRunner::new(eigen_config, env.clone())
        .job(tangle_avs)
        .run()
        .await
        .unwrap();

    info!("Exiting...");
}

#[tokio::test]
async fn test_holesky_tangle_avs() {
    use constants::holesky::*;
    gadget_sdk::logging::setup_log();

    let eth_http_endpoint = "https://ethereum-holesky.publicnode.com".to_string();
    let eth_ws_endpoint = "wss://ethereum-holesky.publicnode.com".to_string();
    let tangle_ws_endpoint = "ws://127.0.0.1:9948".to_string();

    // Private key of the account that will send ETH to the new test operator
    let funder_private_key = std::env::var("FUNDER_PRIVATE_KEY").expect("FUNDER_PRIVATE_KEY must be set");

    // Create a provider for the ETH network
    let provider = alloy_provider::ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(eth_http_endpoint.parse().unwrap())
        .root()
        .clone()
        .boxed();

    // Setup keystores for test
    set_tangle_env_vars();
    let tmp_dir = tempfile::TempDir::new().unwrap();
    let keystore_paths = generate_tangle_avs_keys(tmp_dir.path()).await;

    // Get the operator's keys
    let operator_keystore_uri = keystore_paths[5].clone();
    let operator_keystore =
        gadget_sdk::keystore::backend::fs::FilesystemKeystore::open(operator_keystore_uri.clone())
            .unwrap();
    let operator_ecdsa_signer = operator_keystore.ecdsa_key().unwrap();
    let operator_signer = operator_keystore.sr25519_key().unwrap();
    let transfer_destination = operator_signer.account_id();

    let private_key_bytes = operator_ecdsa_signer.signer().seed();
    let private_key_hex = hex::encode(private_key_bytes);
    info!("ECDSA Private Key: {}", private_key_hex);
    info!("ECDSA Public Key: {:?}", operator_ecdsa_signer.public());

    // // // Create signer from funder's private key
    // let funder_signer =
    //     alloy_signer_local::PrivateKeySigner::from_str(&funder_private_key).unwrap();
    //
    // // Transfer ETH to operator's account
    // let anvil_tx_amount = 300_000_000_000_000u64; // Enough to cover registration
    // let tx = alloy_rpc_types::TransactionRequest::default()
    //     .with_from(funder_signer.address())
    //     .with_to(operator_ecdsa_signer.alloy_address().unwrap())
    //     .with_value(U256::from(anvil_tx_amount))
    //     .with_nonce(provider.get_transaction_count(funder_signer.address()).await.unwrap())
    //     .with_chain_id(provider.get_chain_id().await.unwrap())
    //     .with_gas_limit(21_000)
    //     .with_max_priority_fee_per_gas(1_000_000_000)
    //     .with_max_fee_per_gas(20_000_000_000);
    //
    // let funder_wallet = EthereumWallet::from(funder_signer.clone());
    // let tx_envelope = tx.build(&funder_wallet).await.unwrap();
    //
    // let tx_hash = provider
    //     .send_tx_envelope(tx_envelope)
    //     .await
    //     .unwrap()
    //     .watch()
    //     .await
    //     .unwrap();
    // info!(
    //     "Transferred {anvil_tx_amount} from {:?} to {:?}\n\tHash: {:?}",
    //     funder_signer.address(),
    //     operator_ecdsa_signer.alloy_address(),
    //     tx_hash
    // );

    // Parse Tangle node URL
    let ws_tangle_url = Url::parse(&tangle_ws_endpoint).unwrap();
    let target_port = ws_tangle_url.clone().port().unwrap();

    // Create the GadgetConfiguration
    let config = ContextConfig {
        gadget_core_settings: GadgetCLICoreSettings::Run {
            target_addr: IpAddr::from_str("127.0.0.1").unwrap(),
            target_port,
            use_secure_url: false,
            test_mode: false,
            log_id: None,
            http_rpc_url: Url::parse(&eth_http_endpoint).unwrap(),
            bootnodes: None,
            keystore_uri: operator_keystore_uri,
            chain: gadget_io::SupportedChains::Testnet,
            verbose: 3,
            pretty: true,
            keystore_password: None,
            blueprint_id: Some(0),
            service_id: Some(0),
            skip_registration: false,
            protocol: Protocol::Eigenlayer,
            registry_coordinator: Some(ZERO_ADDRESS), // We don't need this for the Tangle AVS
            operator_state_retriever: Some(ZERO_ADDRESS), // We don't need this for the Tangle AVS
            delegation_manager: Some(DELEGATION_MANAGER_ADDR),
            ws_rpc_url: Url::parse(&eth_ws_endpoint).unwrap(),
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
    let signer_id = signer.clone().account_id();

    // Get Bob's keys for Tangle transfer
    let bob_keystore_uri = keystore_paths[1].clone();
    let bob_keystore =
        gadget_sdk::keystore::backend::fs::FilesystemKeystore::open(bob_keystore_uri).unwrap();
    let transfer_signer = bob_keystore.sr25519_key().unwrap();

    // Spawn task to transfer balance into Operator's account on Tangle
    let transfer_task = async move {
        tokio::time::sleep(Duration::from_secs(4)).await;
        info!(
            "Transferring balance from {:?} to {:?}",
            signer_id, transfer_destination
        );
        let transfer_tx = gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::tx()
            .balances()
            .transfer_allow_death(transfer_destination.into(), 1_000_000_000_000_000_000);
        match gadget_sdk::tx::tangle::send(&transfer_client, &transfer_signer, &transfer_tx).await {
            Ok(result) => {
                info!("Transfer Result: {:?}", result);
            }
            Err(e) => {
                error!("Balance Transfer Error: {:?}", e);
            }
        }
    };
    let _transfer_handle = tokio::task::spawn(transfer_task);

    // Create Instance of the Event Handler
    let context = BalanceTransferContext {
        client: client.clone(),
        env: env.clone(),
    };
    let tangle_avs = RegisterToTangleEventHandler {
        service_id: 0,
        context: context.clone(),
        client,
        signer,
    };

    // Register to Eigenlayer and AVS
    register_to_eigenlayer_and_avs(&env, TANGLE_SERVICE_MANAGER_ADDR)
        .await
        .unwrap();

    // Start the Runner
    info!("~~~ Executing the Tangle AVS ~~~");
    let eigen_config = EigenlayerConfig {};
    BlueprintRunner::new(eigen_config, env.clone())
        .job(tangle_avs)
        .run()
        .await
        .unwrap();

    info!("Exiting...");
}

/// Sets some environment variables with some random seeds for testing
///
/// # Warning
/// This function is for internal testing purposes. It uses keys that are visible to the public.
///
pub(crate) fn set_tangle_env_vars() {
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
}

/// Generate the keys required for Tangle AVS
///
/// # Warning
/// This function is specifically for testing. It will panic upon any errors and utilizes keys that are publicly visible.
pub(crate) async fn generate_tangle_avs_keys(keystore_base_path: &Path) -> Vec<String> {
    // Set up the Keys required for Tangle AVS
    let mut keystore_paths = Vec::new();

    // First we inject the premade Tangle Account keys
    for (item, name) in NAME_IDS.iter().enumerate() {
        let tmp_store = Uuid::new_v4().to_string();
        let keystore_uri =
            keystore_base_path.join(format!("keystores/{}/{tmp_store}/", name.to_lowercase()));
        assert!(
            !keystore_uri.exists(),
            "Keystore URI cannot exist: {}",
            keystore_uri.display()
        );
        let keystore_uri_normalized =
            std::path::absolute(keystore_uri.clone()).expect("Failed to resolve keystore URI");
        let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());
        keystore_paths.push(keystore_uri_str);
        inject_test_keys(&keystore_uri, KeyGenType::Tangle(item))
            .await
            .expect("Failed to inject testing keys for Tangle AVS");
    }

    // Now we create a new Tangle Account for the Test
    let tmp_store = Uuid::new_v4().to_string();
    let keystore_uri = keystore_base_path.join(format!("keystores/testnode/{tmp_store}/"));
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
    let (_acco, acco_seed) =
        sp_core::sr25519::Pair::from_phrase(&acco_suri, None).expect("Should be valid SR keypair");
    info!("Found SR_SEED: {:?}", acco_seed);

    let role_suri = std::env::var("ROLE_SURI").expect("ROLE_SURI not set");
    let (_role, role_seed) =
        sp_core::ecdsa::Pair::from_phrase(&role_suri, None).expect("Should be valid ECDSA keypair");
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
