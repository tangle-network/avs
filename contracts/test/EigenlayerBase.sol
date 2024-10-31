// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.8.0;

import "forge-std/Test.sol";

import {ISlasher} from "../src/interfaces/vendored/ISlasher.sol";
import {TestAVSDirectory} from "../src/test/TestAVSDirectory.sol";
import {TestDelegationManager} from "../src/test/TestDelegationManager.sol";
import {TestSlasher} from "../src/test/TestSlasher.sol";

contract EigenlayerBase is Test {
    TestAVSDirectory internal avsDirectory;
    TestDelegationManager internal delegationManager;
    ISlasher internal slasher;

    function _deployMockEigenLayerAndAVS() internal {
        avsDirectory = new TestAVSDirectory();
        delegationManager = new TestDelegationManager();
        slasher = new TestSlasher();
    }
}
