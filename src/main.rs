use color_eyre::Result;
use gadget_sdk::{config::ContextConfig, info, run::GadgetRunner};
use structopt::StructOpt;
use tangle_avs::TangleGadgetRunner;

#[tokio::main]
async fn main() -> Result<()> {
    gadget_sdk::logging::setup_log();
    println!("{}", tangle_avs::utils::tangle::TANGLE_AVS_ASCII);
    // Load the environment and create the gadget runner
    let config = ContextConfig::from_args();
    let env = gadget_sdk::config::load(config).expect("Failed to load environment");
    let mut runner = Box::new(TangleGadgetRunner { env: env.clone() });

    info!("~~~ Executing the Tangle AVS ~~~");

    info!("Registering...");
    // Register the operator if needed
    if env.should_run_registration() {
        runner.register().await?;
    }

    info!("Running...");
    // Run the gadget / AVS
    runner.run().await?;

    info!("Exiting...");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_provider::Provider;
    use std::time::Duration;
    use tangle_avs::utils::sol_imports::*;
    use tangle_avs::*;

    const ANVIL_STATE_PATH: &str = "./saved_testnet_state.json";

    #[tokio::test]
    async fn test_tangle_avs() {
        gadget_sdk::logging::setup_log();

        // Begin the Anvil Testnet
        let (_container, http_endpoint, ws_endpoint) =
            blueprint_test_utils::anvil::start_anvil_container(ANVIL_STATE_PATH, true).await;
        std::env::set_var("EIGENLAYER_HTTP_ENDPOINT", http_endpoint.clone());
        std::env::set_var("EIGENLAYER_WS_ENDPOINT", ws_endpoint);

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
            REGISTRY_COORDINATOR_ADDR, // TODO: Needs to be updated to PaymentCoordinator
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

        let config = ContextConfig::from_args();
        let env = gadget_sdk::config::load(config).expect("Failed to load environment");
        let mut runner = Box::new(TangleGadgetRunner { env: env.clone() });

        info!("~~~ Executing the incredible squaring blueprint ~~~");

        info!("Registering...");
        // Register the operator if needed
        if env.should_run_registration() {
            runner.register().await.unwrap();
        }

        info!("Running...");
        // Run the gadget / AVS
        runner.run().await.unwrap();

        info!("Exiting...");
    }
}
