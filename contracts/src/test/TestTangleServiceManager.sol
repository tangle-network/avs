// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.8.0;

import {Enrollment, EnrollmentStatus, EnumerableMapEnrollment} from "../libs/EnumerableMapEnrollment.sol";
import {TangleServiceManager} from "../TangleServiceManager.sol";
import {IRegistryCoordinator} from "../../lib/eigenlayer-middleware/src/interfaces/IRegistryCoordinator.sol";
import {IAVSDirectory} from "../../lib/eigenlayer-middleware/lib/eigenlayer-contracts/src/contracts/interfaces/IAVSDirectory.sol";
import {ISlasher} from "../../lib/eigenlayer-middleware/lib/eigenlayer-contracts/src/contracts/interfaces/ISlasher.sol";
import {IStakeRegistry} from "../../lib/eigenlayer-middleware/src/interfaces/IStakeRegistry.sol";

contract TestTangleServiceManager is TangleServiceManager {
    using EnumerableMapEnrollment for EnumerableMapEnrollment.AddressToEnrollmentMap;

    constructor(
        IAVSDirectory _avsDirectory,
        IRegistryCoordinator _registryCoordinator,
        IStakeRegistry _stakeRegistry,
        ISlasher _slasher
    ) TangleServiceManager(_avsDirectory, _registryCoordinator, _stakeRegistry, _slasher) {}

    function mockSetUnenrolled(address operator, address challenger) external {
        enrolledChallengers[operator].set(address(challenger), Enrollment(EnrollmentStatus.UNENROLLED, 0));
    }
}
