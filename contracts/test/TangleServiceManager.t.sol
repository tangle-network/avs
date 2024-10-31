// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.8.0;

import {IDelegationManager} from "../src/interfaces/vendored/IDelegationManager.sol";
import {ISlasher} from "../src/interfaces/vendored/ISlasher.sol";

import {IAVSDirectory} from "../src/interfaces/vendored/IAVSDirectory.sol";
import {Quorum, StrategyParams} from "../src/interfaces/vendored/IECDSAStakeRegistryEventsAndErrors.sol";
import {TestDelegationManager} from "../src/test/TestDelegationManager.sol";
import {ECDSAStakeRegistry} from "../src/ECDSAStakeRegistry.sol";
import {TestPaymentCoordinator} from "../src/test/TestPaymentCoordinator.sol";

import {IStrategy} from "../src/interfaces/vendored/IStrategy.sol";
import {ISignatureUtils} from "../src/interfaces/vendored/ISignatureUtils.sol";
import {Enrollment, EnrollmentStatus} from "../src/libs/EnumerableMapEnrollment.sol";
import {IRemoteChallenger} from "../src/interfaces/IRemoteChallenger.sol";

import {TangleServiceManager} from "../src/TangleServiceManager.sol";
import {TestTangleServiceManager} from "../src/test/TestTangleServiceManager.sol";
import {TestRemoteChallenger} from "../src/test/TestRemoteChallenger.sol";

import {EigenlayerBase} from "./EigenlayerBase.sol";

contract TangleServiceManagerTest is EigenlayerBase {
    TestTangleServiceManager internal _tsm;
    ECDSAStakeRegistry internal _ecdsaStakeRegistry;
    TestPaymentCoordinator internal _paymentCoordinator;

    // Operator info
    uint256 operatorPrivateKey = 0xdeadbeef;
    address operator;
    address avsSigningKey = address(0xc0ffee);

    bytes32 emptySalt;
    uint256 maxExpiry = type(uint256).max;
    uint256 challengeDelayBlocks = 50400; // one week of eth L1 blocks
    address invalidServiceManager = address(0x1234);

    function setUp() public {
        _deployMockEigenLayerAndAVS();

        _ecdsaStakeRegistry = new ECDSAStakeRegistry(delegationManager);
        _paymentCoordinator = new TestPaymentCoordinator();

        // TODO: Investigate Hyperlane testing infra
        address _mailbox = address(0);

        _tsm = new TestTangleServiceManager(
            address(avsDirectory),
            address(_ecdsaStakeRegistry),
            address(_paymentCoordinator),
            address(delegationManager),
            address(_mailbox)
        );
        _tsm.initialize(address(this));
        _tsm.setSlasher(slasher);

        IStrategy mockStrategy = IStrategy(address(0x1234));
        Quorum memory quorum = Quorum({strategies: new StrategyParams[](1)});
        quorum.strategies[0] = StrategyParams({strategy: mockStrategy, multiplier: 10000});
        _ecdsaStakeRegistry.initialize(address(_tsm), 6667, quorum);

        // register operator to eigenlayer
        operator = vm.addr(operatorPrivateKey);
        vm.prank(operator);
        delegationManager.registerAsOperator(
            IDelegationManager.OperatorDetails({
                earningsReceiver: operator,
                delegationApprover: address(0),
                stakerOptOutWindowBlocks: 0
            }),
            ""
        );
        // set operator as registered in Eigenlayer
        delegationManager.setIsOperator(operator, true);
    }

    event AVSMetadataURIUpdated(address indexed avs, string metadataURI);

    function test_updateAVSMetadataURI() public {
        vm.expectEmit(true, true, true, true, address(avsDirectory));
        emit AVSMetadataURIUpdated(address(_tsm), "hyperlaneAVS");
        _tsm.updateAVSMetadataURI("hyperlaneAVS");
    }

    function test_updateAVSMetadataURI_revert_notOwnable() public {
        vm.prank(address(0x1234));
        vm.expectRevert("Ownable: caller is not the owner");
        _tsm.updateAVSMetadataURI("hyperlaneAVS");
    }

    function test_registerOperator() public {
        // act
        ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature =
            _getOperatorSignature(operatorPrivateKey, operator, address(_tsm), emptySalt, maxExpiry);

        vm.prank(operator);
        _ecdsaStakeRegistry.registerOperatorWithSignature(operatorSignature, avsSigningKey);

        // assert
        IAVSDirectory.OperatorAVSRegistrationStatus operatorStatus =
            avsDirectory.avsOperatorStatus(address(_tsm), operator);
        assertEq(uint8(operatorStatus), uint8(IAVSDirectory.OperatorAVSRegistrationStatus.REGISTERED));
    }

    function test_registerOperator_revert_invalidSignature() public {
        // act
        ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature =
            _getOperatorSignature(operatorPrivateKey, operator, address(0x1), emptySalt, maxExpiry);

        vm.prank(operator);
        vm.expectRevert("EIP1271SignatureUtils.checkSignature_EIP1271: signature not from signer");
        _ecdsaStakeRegistry.registerOperatorWithSignature(operatorSignature, avsSigningKey);

        // assert
        IAVSDirectory.OperatorAVSRegistrationStatus operatorStatus =
            avsDirectory.avsOperatorStatus(address(_tsm), operator);
        assertEq(uint8(operatorStatus), uint8(IAVSDirectory.OperatorAVSRegistrationStatus.UNREGISTERED));
    }

    function test_deregisterOperator() public {
        // act
        _registerOperator();
        vm.prank(operator);
        _ecdsaStakeRegistry.deregisterOperator();

        // assert
        IAVSDirectory.OperatorAVSRegistrationStatus operatorStatus =
            avsDirectory.avsOperatorStatus(address(_tsm), operator);
        assertEq(uint8(operatorStatus), uint8(IAVSDirectory.OperatorAVSRegistrationStatus.UNREGISTERED));
    }

    function testFuzz_enrollIntoChallengers(uint8 numOfChallengers) public {
        _registerOperator();
        IRemoteChallenger[] memory challengers = _deployChallengers(numOfChallengers);

        vm.prank(operator);
        _tsm.enrollIntoChallengers(challengers);

        _assertChallengers(challengers, EnrollmentStatus.ENROLLED, 0);
    }

    // to check if the exists in tryGet is working for when we set the enrollment to UNENROLLED
    function test_checkUnenrolled() public {
        _registerOperator();
        IRemoteChallenger[] memory challengers = _deployChallengers(1);

        vm.prank(operator);
        _tsm.enrollIntoChallengers(challengers);
        _assertChallengers(challengers, EnrollmentStatus.ENROLLED, 0);

        _tsm.mockSetUnenrolled(operator, address(challengers[0]));
        _assertChallengers(challengers, EnrollmentStatus.UNENROLLED, 0);
    }

    function testFuzz_startUnenrollment_revert(uint8 numOfChallengers) public {
        vm.assume(numOfChallengers > 0);

        _registerOperator();
        IRemoteChallenger[] memory challengers = _deployChallengers(numOfChallengers);

        vm.startPrank(operator);

        vm.expectRevert("TangleServiceManager: challenger isn't enrolled");
        _tsm.startUnenrollment(challengers);

        _tsm.enrollIntoChallengers(challengers);
        _tsm.startUnenrollment(challengers);
        _assertChallengers(challengers, EnrollmentStatus.PENDING_UNENROLLMENT, block.number);

        vm.expectRevert("TangleServiceManager: challenger isn't enrolled");
        _tsm.startUnenrollment(challengers);
        _assertChallengers(challengers, EnrollmentStatus.PENDING_UNENROLLMENT, block.number);

        vm.stopPrank();
    }

    function testFuzz_startUnenrollment(uint8 numOfChallengers, uint8 numQueued) public {
        vm.assume(numQueued <= numOfChallengers);

        _registerOperator();
        IRemoteChallenger[] memory challengers = _deployChallengers(numOfChallengers);
        IRemoteChallenger[] memory queuedChallengers = new IRemoteChallenger[](numQueued);
        for (uint8 i = 0; i < numQueued; i++) {
            queuedChallengers[i] = challengers[i];
        }
        IRemoteChallenger[] memory unqueuedChallengers = new IRemoteChallenger[](numOfChallengers - numQueued);
        for (uint8 i = numQueued; i < numOfChallengers; i++) {
            unqueuedChallengers[i - numQueued] = challengers[i];
        }

        vm.startPrank(operator);
        _tsm.enrollIntoChallengers(challengers);
        _assertChallengers(challengers, EnrollmentStatus.ENROLLED, 0);

        _tsm.startUnenrollment(queuedChallengers);
        _assertChallengers(queuedChallengers, EnrollmentStatus.PENDING_UNENROLLMENT, block.number);
        _assertChallengers(unqueuedChallengers, EnrollmentStatus.ENROLLED, 0);

        vm.stopPrank();
    }

    function testFuzz_completeQueuedUnenrollmentFromChallenger(uint8 numOfChallengers, uint8 numUnenrollable) public {
        vm.assume(numUnenrollable > 0 && numUnenrollable <= numOfChallengers);

        _registerOperator();
        IRemoteChallenger[] memory challengers = _deployChallengers(numOfChallengers);
        address[] memory unenrollableChallengers = new address[](numUnenrollable);
        for (uint8 i = 0; i < numUnenrollable; i++) {
            unenrollableChallengers[i] = address(challengers[i]);
        }

        vm.startPrank(operator);
        _tsm.enrollIntoChallengers(challengers);
        _tsm.startUnenrollment(challengers);

        _assertChallengers(challengers, EnrollmentStatus.PENDING_UNENROLLMENT, block.number);

        vm.expectRevert();
        _tsm.completeUnenrollment(unenrollableChallengers);

        vm.roll(block.number + challengeDelayBlocks);

        _tsm.completeUnenrollment(unenrollableChallengers);

        assertEq(_tsm.getOperatorChallengers(operator).length, numOfChallengers - numUnenrollable);

        vm.stopPrank();
    }

    function testFuzz_freezeOperator(uint8 numOfChallengers) public {
        _registerOperator();

        IRemoteChallenger[] memory challengers = _deployChallengers(numOfChallengers);

        vm.prank(operator);
        _tsm.enrollIntoChallengers(challengers);

        for (uint256 i = 0; i < challengers.length; i++) {
            vm.expectCall(address(slasher), abi.encodeCall(ISlasher.freezeOperator, (operator)));
            challengers[i].handleChallenge(operator);
        }
    }

    function testFuzz_freezeOperator_duringEnrollment(uint8 numOfChallengers, uint8 numUnenrollable) public {
        vm.assume(numUnenrollable > 0 && numUnenrollable <= numOfChallengers);

        _registerOperator();
        IRemoteChallenger[] memory challengers = _deployChallengers(numOfChallengers);
        address[] memory unenrollableChallengers = new address[](numUnenrollable);
        IRemoteChallenger[] memory otherChallengeChallengers =
            new IRemoteChallenger[](numOfChallengers - numUnenrollable);
        for (uint8 i = 0; i < numUnenrollable; i++) {
            unenrollableChallengers[i] = address(challengers[i]);
        }
        for (uint8 i = numUnenrollable; i < numOfChallengers; i++) {
            otherChallengeChallengers[i - numUnenrollable] = challengers[i];
        }

        vm.startPrank(operator);
        _tsm.enrollIntoChallengers(challengers);

        for (uint256 i = 0; i < challengers.length; i++) {
            vm.expectCall(address(slasher), abi.encodeCall(ISlasher.freezeOperator, (operator)));
            challengers[i].handleChallenge(operator);
        }

        _tsm.startUnenrollment(challengers);
        vm.roll(block.number + challengeDelayBlocks);
        _tsm.completeUnenrollment(unenrollableChallengers);

        for (uint256 i = 0; i < unenrollableChallengers.length; i++) {
            vm.expectRevert("TangleServiceManager: Operator not enrolled in challenger");
            IRemoteChallenger(unenrollableChallengers[i]).handleChallenge(operator);
        }
        for (uint256 i = 0; i < otherChallengeChallengers.length; i++) {
            vm.expectCall(address(slasher), abi.encodeCall(ISlasher.freezeOperator, (operator)));
            otherChallengeChallengers[i].handleChallenge(operator);
        }
        vm.stopPrank();
    }

    function testFuzz_deregisterOperator_withEnrollment() public {
        uint8 numOfChallengers = 1;
        vm.assume(numOfChallengers > 0);

        _registerOperator();
        IRemoteChallenger[] memory challengers = _deployChallengers(numOfChallengers);

        vm.startPrank(operator);
        _tsm.enrollIntoChallengers(challengers);
        _assertChallengers(challengers, EnrollmentStatus.ENROLLED, 0);

        vm.expectRevert("TangleServiceManager: Invalid unenrollment");
        _ecdsaStakeRegistry.deregisterOperator();

        _tsm.startUnenrollment(challengers);

        vm.expectRevert("TangleServiceManager: Invalid unenrollment");
        _ecdsaStakeRegistry.deregisterOperator();

        vm.roll(block.number + challengeDelayBlocks);

        _ecdsaStakeRegistry.deregisterOperator();

        assertEq(_tsm.getOperatorChallengers(operator).length, 0);
        vm.stopPrank();
    }

    // ============ Utility Functions ============

    function _registerOperator() internal {
        ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature =
            _getOperatorSignature(operatorPrivateKey, operator, address(_tsm), emptySalt, maxExpiry);

        vm.prank(operator);
        _ecdsaStakeRegistry.registerOperatorWithSignature(operatorSignature, avsSigningKey);
    }

    function _deployChallengers(uint8 numOfChallengers) internal returns (IRemoteChallenger[] memory challengers) {
        challengers = new IRemoteChallenger[](numOfChallengers);
        for (uint8 i = 0; i < numOfChallengers; i++) {
            challengers[i] = new TestRemoteChallenger(_tsm);
        }
    }

    function _assertChallengers(
        IRemoteChallenger[] memory _challengers,
        EnrollmentStatus _expectedstatus,
        uint256 _expectUnenrollmentBlock
    ) internal {
        for (uint256 i = 0; i < _challengers.length; i++) {
            Enrollment memory enrollment = _tsm.getChallengerEnrollment(operator, _challengers[i]);
            assertEq(uint8(enrollment.status), uint8(_expectedstatus));
            if (_expectUnenrollmentBlock != 0) {
                assertEq(enrollment.unenrollmentStartBlock, _expectUnenrollmentBlock);
            }
        }
    }

    function _getOperatorSignature(
        uint256 _operatorPrivateKey,
        address operatorToSign,
        address avs,
        bytes32 salt,
        uint256 expiry
    ) internal view returns (ISignatureUtils.SignatureWithSaltAndExpiry memory operatorSignature) {
        operatorSignature.salt = salt;
        operatorSignature.expiry = expiry;
        {
            bytes32 digestHash =
                avsDirectory.calculateOperatorAVSRegistrationDigestHash(operatorToSign, avs, salt, expiry);
            (uint8 v, bytes32 r, bytes32 s) = vm.sign(_operatorPrivateKey, digestHash);
            operatorSignature.signature = abi.encodePacked(r, s, v);
        }
        return operatorSignature;
    }
}
