// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.13;

import "@eigenlayer/contracts/libraries/BytesLib.sol";
import "@eigenlayer-middleware/src/ServiceManagerBase.sol";
import "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import "@openzeppelin-upgrades/contracts/proxy/utils/UUPSUpgradeable.sol";

/**
 * @title Primary entrypoint for Eigenlayer operators to manage Tangle services
 * @author Tangle Network.
 * @dev This contract is upgradeable using the UUPS proxy pattern.
 *      To upgrade:
 *      1. Deploy a new implementation contract
 *      2. Call `upgradeTo(address)` or `upgradeToAndCall(address, bytes)` on the proxy contract
 */
contract TangleServiceManager is OwnableUpgradeable, ServiceManagerBase, UUPSUpgradeable {
    using BytesLib for bytes;

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor(IAVSDirectory _avsDirectory, IRegistryCoordinator _registryCoordinator, IStakeRegistry _stakeRegistry)
        ServiceManagerBase(_avsDirectory, _registryCoordinator, _stakeRegistry)
        initializer
    {}

    /**
     * @dev Initializer for TangleServiceManager, replacing the constructor for upgradeable contracts.
     * @param initialOwner The address that will be set as the initial owner of the contract.
     */
    function initialize(address initialOwner) public initializer {
        __Ownable_init();
        __ServiceManagerBase_init(initialOwner);
        __UUPSUpgradeable_init();
    }

    /**
     * @dev Function to authorize upgrade, overridden to only allow the owner to upgrade the contract.
     */
    function _authorizeUpgrade(address) internal override onlyOwner {}
}
