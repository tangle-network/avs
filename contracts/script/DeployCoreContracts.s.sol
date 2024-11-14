// SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.12;

import {Script} from "forge-std/Script.sol";
import {RegistryCoordinator} from "../lib/eigenlayer-middleware/src/RegistryCoordinator.sol";
import {BLSApkRegistry} from "../lib/eigenlayer-middleware/src/BLSApkRegistry.sol";
import {StakeRegistry} from "../lib/eigenlayer-middleware/src/StakeRegistry.sol";
import {IndexRegistry} from "../lib/eigenlayer-middleware/src/IndexRegistry.sol";
import {OperatorStateRetriever} from "../lib/eigenlayer-middleware/src/OperatorStateRetriever.sol";
import {TangleServiceManager} from "../src/TangleServiceManager.sol";
import {PauserRegistry} from "../lib/eigenlayer-middleware/lib/eigenlayer-contracts/src/contracts/permissions/PauserRegistry.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {EmptyContract} from "../lib/eigenlayer-middleware/lib/eigenlayer-contracts/src/test/mocks/EmptyContract.sol";

contract DeployMiddleware is Script {
    // Existing deployed contracts
    address public constant TANGLE_SERVICE_MANAGER = 0x5aBc6138DD384a1b059f1fcBaD73E03c31170C14;
    address public constant ECDSA_STAKE_REGISTRY = 0x131b803Bece581281A2E33d7E693DfA70aB85D06;

    // Proxy admin for upgradeable contracts
    ProxyAdmin public tangleProxyAdmin;

    function run() external {
        IStrategy[1] memory deployedStrategyArray = [IStrategy(0x80528D6e9A2BAbFc766965E0E26d5aB08D9CFaF9)];
        uint numStrategies = deployedStrategyArray.length;

        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        // Deploy proxy admin for upgradeability
        tangleProxyAdmin = new ProxyAdmin();
        
        // Deploy empty contract for initial proxy implementation
        EmptyContract emptyContract = new EmptyContract();

        // Deploy PauserRegistry first (required by RegistryCoordinator)
        address[] memory pausers = new address[](1);
        pausers[0] = vm.addr(deployerPrivateKey); // deployer is the pauser
        address unpauser = vm.addr(deployerPrivateKey); // deployer is the unpauser
        PauserRegistry pauserRegistry = new PauserRegistry(pausers, unpauser);

        // Deploy proxies pointing to empty implementation initially
        RegistryCoordinator registryCoordinator = RegistryCoordinator(
            address(
                new TransparentUpgradeableProxy(
                    address(emptyContract),
                    address(tangleProxyAdmin),
                    ""
                )
            )
        );

        IndexRegistry indexRegistry = IndexRegistry(
            address(
                new TransparentUpgradeableProxy(
                    address(emptyContract),
                    address(tangleProxyAdmin),
                    ""
                )
            )
        );

        BLSApkRegistry blsApkRegistry = BLSApkRegistry(
            address(
                new TransparentUpgradeableProxy(
                    address(emptyContract),
                    address(tangleProxyAdmin),
                    ""
                )
            )
        );

        OperatorStateRetriever operatorStateRetriever = new OperatorStateRetriever();

        // Deploy implementation contracts
        BLSApkRegistry blsApkRegistryImplementation = new BLSApkRegistry(
            registryCoordinator
        );
        tangleProxyAdmin.upgrade(
            TransparentUpgradeableProxy(payable(address(blsApkRegistry))),
            address(blsApkRegistryImplementation)
        );

        IndexRegistry indexRegistryImplementation = new IndexRegistry(
            registryCoordinator
        );
        tangleProxyAdmin.upgrade(
            TransparentUpgradeableProxy(payable(address(indexRegistry))),
            address(indexRegistryImplementation)
        );

        RegistryCoordinator registryCoordinatorImplementation = new RegistryCoordinator(
            IServiceManager(TANGLE_SERVICE_MANAGER),
            IStakeRegistry(ECDSA_STAKE_REGISTRY),
            IBLSApkRegistry(blsApkRegistry),
            IIndexRegistry(indexRegistry)
        );

        {
            uint numQuorums = 1;
            // Define the following for each quorum
            // QuorumOperatorSetParam, minimumStakeForQuorum, and strategyParams
            IRegistryCoordinator.OperatorSetParam[]
            memory quorumsOperatorSetParams = new IRegistryCoordinator.OperatorSetParam[](
                numQuorums
            );
            for (uint i = 0; i < numQuorums; i++) {
                quorumsOperatorSetParams[i] = IRegistryCoordinator
                    .OperatorSetParam({
                    maxOperatorCount: 10000,
                    kickBIPsOfOperatorStake: 15000,
                    kickBIPsOfTotalStake: 100
                });
            }
            // set to 0 for every quorum
            uint96[] memory quorumsMinimumStake = new uint96[](numQuorums);
            IStakeRegistry.StrategyParams[][]
            memory quorumsStrategyParams = new IStakeRegistry.StrategyParams[][](
                numQuorums
            );
            for (uint i = 0; i < numQuorums; i++) {
                quorumsStrategyParams[i] = new IStakeRegistry.StrategyParams[](
                    numStrategies
                );
                for (uint j = 0; j < numStrategies; j++) {
                    quorumsStrategyParams[i][j] = IStakeRegistry
                        .StrategyParams({
                        strategy: deployedStrategyArray[j],
                        multiplier: 1 ether
                    });
                }
            }
            tangleProxyAdmin.upgradeAndCall(
                TransparentUpgradeableProxy(
                    payable(address(registryCoordinator))
                ),
                address(registryCoordinatorImplementation),
                abi.encodeWithSelector(
                    RegistryCoordinator.initialize.selector,
                    unpauser,
                    unpauser,
                    unpauser,
                    pauserRegistry,
                    0, // 0 initialPausedStatus means everything unpaused
                    quorumsOperatorSetParams,
                    quorumsMinimumStake,
                    quorumsStrategyParams
                )
            );
        }

        vm.stopBroadcast();

        // Log deployed addresses
        console.log("Deployed contracts:");
        console.log("PauserRegistry:", address(pauserRegistry));
        console.log("IndexRegistry:", address(indexRegistry));
        console.log("BLSApkRegistry:", address(blsApkRegistry));
        console.log("RegistryCoordinator (Proxy):", address(registryCoordinator));
        console.log("RegistryCoordinator (Implementation):", address(registryCoordinatorImplementation));
        console.log("OperatorStateRetriever:", address(operatorStateRetriever));
        console.log("ProxyAdmin:", address(tangleProxyAdmin));
    }
}