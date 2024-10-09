// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.8.0;

import {Enrollment, EnrollmentStatus, EnumerableMapEnrollment} from "../libs/EnumerableMapEnrollment.sol";
import {TangleServiceManager} from "../TangleServiceManager.sol";

contract TestTangleServiceManager is TangleServiceManager {
    using EnumerableMapEnrollment for EnumerableMapEnrollment.AddressToEnrollmentMap;

    constructor(address _avsDirectory, address _stakeRegistry, address _paymentCoordinator, address _delegationManager)
        TangleServiceManager(_avsDirectory, _stakeRegistry, _paymentCoordinator, _delegationManager)
    {}
}
