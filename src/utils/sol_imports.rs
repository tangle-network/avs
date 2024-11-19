alloy_sol_types::sol!(
    #[allow(missing_docs, clippy::too_many_arguments)]
    #[sol(rpc)]
    RegistryCoordinator,
    "./contracts/lib/eigenlayer-middleware/out/RegistryCoordinator.sol/RegistryCoordinator.json"
);

alloy_sol_types::sol!(
    #[allow(missing_docs, clippy::too_many_arguments)]
    #[sol(rpc)]
    TangleServiceManager,
    "./contracts/out/TangleServiceManager.sol/TangleServiceManager.json"
);

alloy_sol_types::sol!(
    #[allow(missing_docs, clippy::too_many_arguments)]
    #[sol(rpc)]
    ECDSAStakeRegistry,
    "./contracts/out/ECDSAStakeRegistry.sol/ECDSAStakeRegistry.json"
);
