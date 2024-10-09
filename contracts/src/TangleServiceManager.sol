// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.13;

import "@eigenlayer/contracts/libraries/BytesLib.sol";
import "@eigenlayer-middleware/src/ServiceManagerBase.sol";
import "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

/**
 * @title Primary entrypoint for Eigenlayer operators to manage Tangle services
 * @author Tangle Network.
 * @dev This contract is upgradeable using the UUPS proxy pattern.
 *      To upgrade:
 *      1. Deploy a new implementation contract
 *      2. Call `upgradeTo(address)` or `upgradeToAndCall(address, bytes)` on the proxy contract
 */
contract TangleServiceManager is OwnableUpgradeable, ServiceManagerBase {
    using BytesLib for bytes;

    /**
     * @dev Constructor for ECDSAServiceManagerBase, initializing immutable contract addresses and disabling initializers.
     * @param _avsDirectory The address of the AVS directory contract, managing AVS-related data for registered operators.
     * @param _stakeRegistry The address of the stake registry contract, managing registration and stake recording.
     * @param _paymentCoordinator The address of the payment coordinator contract, handling payment distributions.
     * @param _delegationManager The address of the delegation manager contract, managing staker delegations to operators.
     */
    constructor(
        address _avsDirectory,
        address _stakeRegistry,
        address _paymentCoordinator,
        address _delegationManager
    ) {
        avsDirectory = _avsDirectory;
        stakeRegistry = _stakeRegistry;
        paymentCoordinator = _paymentCoordinator;
        delegationManager = _delegationManager;
    }
}