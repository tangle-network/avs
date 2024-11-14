// SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.12;

import {Script} from "forge-std/Script.sol";
import {RegistryCoordinator} from "../lib/eigenlayer-middleware/src/RegistryCoordinator.sol";
import {BLSApkRegistry} from "../lib/eigenlayer-middleware/src/BLSApkRegistry.sol";
import {StakeRegistry} from "../lib/eigenlayer-middleware/src/StakeRegistry.sol";
import {IndexRegistry} from "../lib/eigenlayer-middleware/src/IndexRegistry.sol";
import {TangleServiceManager} from "../src/TangleServiceManager.sol";

contract InitializeMiddleware is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        // Load deployed contract addresses from environment variables
        address registryCoordinatorAddr = vm.envAddress("REGISTRY_COORDINATOR_ADDRESS");
        address serviceManagerAddr = vm.envAddress("SERVICE_MANAGER_ADDRESS");
        address stakeRegistryAddr = vm.envAddress("STAKE_REGISTRY_ADDRESS");
        address indexRegistryAddr = vm.envAddress("INDEX_REGISTRY_ADDRESS");
        address blsApkRegistryAddr = vm.envAddress("BLS_APK_REGISTRY_ADDRESS");

        // Initialize contracts
        RegistryCoordinator registryCoordinator = RegistryCoordinator(registryCoordinatorAddr);
        TangleServiceManager serviceManager = TangleServiceManager(serviceManagerAddr);
        StakeRegistry stakeRegistry = StakeRegistry(stakeRegistryAddr);
        IndexRegistry indexRegistry = IndexRegistry(indexRegistryAddr);
        BLSApkRegistry blsApkRegistry = BLSApkRegistry(blsApkRegistryAddr);

        // Initialize ServiceManager
        serviceManager.initialize(
            registryCoordinator,
            vm.addr(deployerPrivateKey) // owner
        );

        // Initialize quorum parameters for the registries
        uint8[] memory quorumNumbers = new uint8[](1);
        quorumNumbers[0] = 0; // First quorum

        // Stake Registry parameters
        uint96[] memory minimumStakes = new uint96[](1);
        minimumStakes[0] = 32 ether; // Minimum stake for quorum 0
        
        uint96[] memory strategyWeights = new uint96[](1);
        strategyWeights[0] = 1000; // Weight for quorum 0 (in basis points)

        address[] memory strategies = new address[](1);
        strategies[0] = address(0); // Replace with actual strategy address

        // Initialize the first quorum
        registryCoordinator.initialize(
            quorumNumbers,
            minimumStakes,
            strategies,
            strategyWeights
        );

        vm.stopBroadcast();

        console.log("Middleware contracts initialized successfully");
    }
}
