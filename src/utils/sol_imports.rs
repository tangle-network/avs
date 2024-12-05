pub mod registry_coordinator {
    alloy_sol_types::sol!(
        #[allow(missing_docs, clippy::too_many_arguments)]
        #[sol(rpc)]
        RegistryCoordinator,
        "./contracts/lib/eigenlayer-middleware/out/RegistryCoordinator.sol/RegistryCoordinator.json"
    );
}

pub mod tangle_service_manager {
    alloy_sol_types::sol!(
        #[allow(missing_docs, clippy::too_many_arguments)]
        #[sol(rpc)]
        TangleServiceManager,
        "./contracts/out/TangleServiceManager.sol/TangleServiceManager.json"
    );
}

pub mod ecdsa_stake_registry {
    alloy_sol_types::sol!(
        #[allow(missing_docs, clippy::too_many_arguments)]
        #[sol(rpc)]
        ECDSAStakeRegistry,
        "./contracts/out/ECDSAStakeRegistry.sol/ECDSAStakeRegistry.json"
    );
}
