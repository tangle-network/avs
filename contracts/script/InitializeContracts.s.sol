// SPDX-License-Identifier: MIT
pragma solidity ^0.8.12;

import {Script} from "forge-std/Script.sol";
import {TangleServiceManager} from "../src/TangleServiceManager.sol";
import {ECDSAStakeRegistry} from "../src/ECDSAStakeRegistry.sol";
import {StrategyParams, Quorum} from "../src/ECDSAStakeRegistryStorage.sol";
import {IStrategy} from "../src/interfaces/vendored/IStrategy.sol";

contract InitializeContracts is Script {
    function setUp() public {}

    function run() public {
        // Load private key from environment
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        address tangleServiceManagerAddr = 0x5aBc6138DD384a1b059f1fcBaD73E03c31170C14;
        address ecdsaStakeRegistryAddr = 0x131b803Bece581281A2E33d7E693DfA70aB85D06;

        // Initialize TangleServiceManager
        TangleServiceManager tangleServiceManager = TangleServiceManager(tangleServiceManagerAddr);
        tangleServiceManager.initialize(msg.sender);

        // Initialize ECDSAStakeRegistry
        ECDSAStakeRegistry ecdsaStakeRegistry = ECDSAStakeRegistry(ecdsaStakeRegistryAddr);

        // Create a quorum configuration with the WETH strategy
        IStrategy[] memory strategies = new IStrategy[](1);
        uint96[] memory weights = new uint96[](1);

        // WETH Strategy on Holesky
        strategies[0] = IStrategy(0x80528D6e9A2BAbFc766965E0E26d5aB08D9CFaF9);
        weights[0] = 10000; // 100% weight

        StrategyParams[] memory strategyParams = new StrategyParams[](1);
        strategyParams[0] = StrategyParams(strategies[0], weights[0]);

        Quorum memory quorum = Quorum(strategyParams);

        // Initialize with a 50% threshold (5000 basis points)
        ecdsaStakeRegistry.initialize(tangleServiceManagerAddr, 5000, quorum);

        vm.stopBroadcast();
    }
}
