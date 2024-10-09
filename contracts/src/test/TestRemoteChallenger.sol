// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.8.0;

import {IRemoteChallenger} from "../interfaces/IRemoteChallenger.sol";
import {TangleServiceManager} from "../TangleServiceManager.sol";

contract TestRemoteChallenger is IRemoteChallenger {
    TangleServiceManager internal immutable tsm;

    constructor(TangleServiceManager _tsm) {
        tsm = _tsm;
    }

    function challengeDelayBlocks() external pure returns (uint256) {
        return 50400; // one week of eth L1 blocks
    }

    function handleChallenge(address operator) external {
        tsm.freezeOperator(operator);
    }
}
