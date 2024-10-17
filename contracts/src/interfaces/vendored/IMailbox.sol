// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.8.0;

interface IMailbox {
    function dispatch(uint32 _destinationDomain, bytes32 _recipientAddress, bytes calldata _messageBody)
        external
        payable
        returns (bytes32);

    function quoteDispatch(uint32 _destinationDomain, bytes32 _recipientAddress, bytes calldata _messageBody)
        external
        returns (uint256);

    function process(bytes calldata _metadata, bytes calldata _message) external;
}
