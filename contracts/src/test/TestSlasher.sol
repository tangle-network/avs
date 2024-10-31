// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.8.0;

import {ISlasher} from "../interfaces/vendored/ISlasher.sol";

contract TestSlasher is ISlasher {
    function freezeOperator(address toBeFrozen) external {}
}
