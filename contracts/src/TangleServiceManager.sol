// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.8.12;

// ============ Internal Imports ============
import {Enrollment, EnrollmentStatus, EnumerableMapEnrollment} from "./libs/EnumerableMapEnrollment.sol";
import {IAVSDirectory} from "../lib/eigenlayer-middleware/lib/eigenlayer-contracts/src/contracts/interfaces/IAVSDirectory.sol";
import {IRegistryCoordinator} from "../lib/eigenlayer-middleware/src/interfaces/IRegistryCoordinator.sol";
import {IRemoteChallenger} from "./interfaces/IRemoteChallenger.sol";
import {ISlasher} from "../lib/eigenlayer-middleware/lib/eigenlayer-contracts/src/contracts/interfaces/ISlasher.sol";
import {IStakeRegistry} from "../lib/eigenlayer-middleware/src/interfaces/IStakeRegistry.sol";
import {ServiceManagerBase} from "../lib/eigenlayer-middleware/src/ServiceManagerBase.sol";

contract TangleServiceManager is ServiceManagerBase {
    // ============ Libraries ============

    using EnumerableMapEnrollment for EnumerableMapEnrollment.AddressToEnrollmentMap;

    // ============ Public Storage ============

    // Slasher contract responsible for slashing operators
    ISlasher internal immutable slasher;

    // Mapping of operators to challengers they are enrolled in (enumerable required for remove-all)
    mapping(address => EnumerableMapEnrollment.AddressToEnrollmentMap) internal enrolledChallengers;

    // Mapping to store operator keys
    mapping(address => OperatorKeys) public operatorKeys;

    // Structure to store operator keys
    struct OperatorKeys {
        bytes validatorKeys;
        bytes32 accountKey;
    }

    // ============ Events ============

    /**
     * @notice Emitted when an operator is enrolled in a challenger
     * @param operator The address of the operator
     * @param challenger The address of the challenger
     */
    event OperatorEnrolledToChallenger(address operator, IRemoteChallenger challenger);

    /**
     * @notice Emitted when an operator is queued for unenrollment from a challenger
     * @param operator The address of the operator
     * @param challenger The address of the challenger
     * @param unenrollmentStartBlock The block number at which the unenrollment was queued
     * @param challengeDelayBlocks The number of blocks to wait before unenrollment is complete
     */
    event OperatorQueuedUnenrollmentFromChallenger(
        address operator, IRemoteChallenger challenger, uint256 unenrollmentStartBlock, uint256 challengeDelayBlocks
    );

    /**
     * @notice Emitted when an operator is unenrolled from a challenger
     * @param operator The address of the operator
     * @param challenger The address of the challenger
     * @param unenrollmentEndBlock The block number at which the unenrollment was completed
     */
    event OperatorUnenrolledFromChallenger(
        address operator, IRemoteChallenger challenger, uint256 unenrollmentEndBlock
    );

    /**
     * @notice Event emitted when an operator sets their keys
     * @param operator The address of the operator
     * @param validatorKeys The validator key set by the operator
     * @param accountKey The account key set by the operator
     */
    event OperatorKeysSet(address indexed operator, bytes validatorKeys, bytes32 accountKey);

    // ============ Modifiers ============

    // Only allows the challenger the operator is enrolled in to call the function
    modifier onlyEnrolledChallenger(address operator) {
        (bool exists,) = enrolledChallengers[operator].tryGet(msg.sender);
        require(exists, "TangleServiceManager: Operator not enrolled in challenger");
        _;
    }

    // ============ Constructor ============

    constructor(
        IAVSDirectory _avsDirectory,
        IRegistryCoordinator _registryCoordinator,
        IStakeRegistry _stakeRegistry,
        ISlasher _slasher
    ) ServiceManagerBase(_avsDirectory, _registryCoordinator, _stakeRegistry) {
        slasher = _slasher;
    }

    function initialize(address initialOwner) external initializer {
        __ServiceManagerBase_init(initialOwner);
    }

    // ============ External Functions ============

    /**
     * @notice Enrolls as an operator into a list of challengers
     * @param _challengers The list of challengers to enroll into
     */
    function enrollIntoChallengers(IRemoteChallenger[] memory _challengers) external {
        for (uint256 i = 0; i < _challengers.length; i++) {
            enrollIntoChallenger(_challengers[i]);
        }
    }

    /**
     * @notice starts an operator for unenrollment from a list of challengers
     * @param _challengers The list of challengers to unenroll from
     */
    function startUnenrollment(IRemoteChallenger[] memory _challengers) external {
        for (uint256 i = 0; i < _challengers.length; i++) {
            startUnenrollment(_challengers[i]);
        }
    }

    /**
     * @notice Completes the unenrollment of an operator from a list of challengers
     * @param _challengers The list of challengers to unenroll from
     */
    function completeUnenrollment(address[] memory _challengers) external {
        _completeUnenrollment(msg.sender, _challengers);
    }

    /**
     * @notice returns the status of a challenger an operator is enrolled in
     * @param _operator The address of the operator
     * @param _challenger specified IRemoteChallenger contract
     */
    function getChallengerEnrollment(address _operator, IRemoteChallenger _challenger)
        external
        view
        returns (Enrollment memory enrollment)
    {
        return enrolledChallengers[_operator].get(address(_challenger));
    }

    /**
     * @notice forwards a call to the Slasher contract to freeze an operator
     * @param operator The address of the operator to freeze.
     * @dev only the enrolled challengers can call this function
     */
    function freezeOperator(address operator) external virtual onlyEnrolledChallenger(operator) {
        slasher.freezeOperator(operator);
    }

    // ============ Public Functions ============

    /**
     * @notice returns the list of challengers an operator is enrolled in
     * @param _operator The address of the operator
     */
    function getOperatorChallengers(address _operator) public view returns (address[] memory) {
        return enrolledChallengers[_operator].keys();
    }

    /**
     * @notice Enrolls as an operator into a single challenger
     * @param challenger The challenger to enroll into
     */
    function enrollIntoChallenger(IRemoteChallenger challenger) public {
        require(enrolledChallengers[msg.sender].set(address(challenger), Enrollment(EnrollmentStatus.ENROLLED, 0)));
        emit OperatorEnrolledToChallenger(msg.sender, challenger);
    }

    /**
     * @notice starts an operator for unenrollment from a challenger
     * @param challenger The challenger to unenroll from
     */
    function startUnenrollment(IRemoteChallenger challenger) public {
        (bool exists, Enrollment memory enrollment) = enrolledChallengers[msg.sender].tryGet(address(challenger));
        require(
            exists && enrollment.status == EnrollmentStatus.ENROLLED, "TangleServiceManager: challenger isn't enrolled"
        );

        enrolledChallengers[msg.sender].set(
            address(challenger), Enrollment(EnrollmentStatus.PENDING_UNENROLLMENT, uint248(block.number))
        );
        emit OperatorQueuedUnenrollmentFromChallenger(
            msg.sender, challenger, block.number, challenger.challengeDelayBlocks()
        );
    }

    /**
     * @notice Completes the unenrollment of an operator from a challenger
     * @param challenger The challenger to unenroll from
     */
    function completeUnenrollment(address challenger) public {
        _completeUnenrollment(msg.sender, challenger);
    }

    // ============ Internal Functions ============

    /**
     * @notice Completes the unenrollment of an operator from a list of challengers
     * @param operator The address of the operator
     * @param _challengers The list of challengers to unenroll from
     */
    function _completeUnenrollment(address operator, address[] memory _challengers) internal {
        for (uint256 i = 0; i < _challengers.length; i++) {
            _completeUnenrollment(operator, _challengers[i]);
        }
    }

    /**
     * @notice Completes the unenrollment of an operator from a challenger
     * @param operator The address of the operator
     * @param _challenger The challenger to unenroll from
     */
    function _completeUnenrollment(address operator, address _challenger) internal {
        IRemoteChallenger challenger = IRemoteChallenger(_challenger);
        (bool exists, Enrollment memory enrollment) = enrolledChallengers[operator].tryGet(address(challenger));

        require(
            exists && enrollment.status == EnrollmentStatus.PENDING_UNENROLLMENT
                && block.number >= enrollment.unenrollmentStartBlock + challenger.challengeDelayBlocks(),
            "TangleServiceManager: Invalid unenrollment"
        );

        enrolledChallengers[operator].remove(address(challenger));
        emit OperatorUnenrolledFromChallenger(operator, challenger, block.number);
    }

//    /// @inheritdoc ServiceManagerBase
//    function _deregisterOperatorFromAVS(address operator) internal virtual override {
//        address[] memory challengers = getOperatorChallengers(operator);
//        _completeUnenrollment(operator, challengers);
//
//        IAVSDirectory(_avsDirectory).deregisterOperatorFromAVS(operator);
//        emit OperatorDeregisteredFromAVS(operator);
//    }

    /// @notice Allows an operator to set their validator and account keys
    /// @param _validatorKeys The validator keys for the operator
    /// @param _accountKey The account key for the operator
    function setOperatorKeys(bytes memory _validatorKeys, bytes32 _accountKey) external {
        require(_validatorKeys.length != 0 && _accountKey != bytes32(0), "Invalid keys");

        operatorKeys[msg.sender] = OperatorKeys({validatorKeys: _validatorKeys, accountKey: _accountKey});

        emit OperatorKeysSet(msg.sender, _validatorKeys, _accountKey);
    }
}
