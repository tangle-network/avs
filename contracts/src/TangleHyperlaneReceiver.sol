// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.8.0;

import {IMailbox} from "./interfaces/vendored/IMailbox.sol";
import {TypeCasts} from "./libs/TypeCasts.sol";

contract TangleHyperlaneReceiver {
    // Hyperlane Mailbox contract
    IMailbox public immutable mailbox;

    // Expected origin domain (Ethereum mainnet)
    uint32 public constant ETHEREUM_DOMAIN = 1;

    // Expected sender address (should be set to the TangleServiceManager address on Ethereum)
    bytes32 public immutable EXPECTED_SENDER;

    event MessageReceived(bytes32 operator, bytes validatorKeys, bytes32 accountKey);

    constructor(address _mailbox, address _expectedSender) {
        mailbox = IMailbox(_mailbox);
        EXPECTED_SENDER = TypeCasts.addressToBytes32(_expectedSender);
    }

    function handle(
        uint32 _origin,
        bytes32 _sender,
        bytes calldata _message
    ) external payable {
        require(msg.sender == address(mailbox), "Only mailbox can call handle");
        require(_origin == ETHEREUM_DOMAIN, "Invalid origin domain");
        require(_sender == EXPECTED_SENDER, "Invalid sender");

        (bytes32 operator, bytes memory validatorKeys, bytes32 accountKey) = abi.decode(_message, (bytes32, bytes, bytes32));

        emit MessageReceived(operator, validatorKeys, accountKey);

        // Send TNT to the operator 
        
    }
}
