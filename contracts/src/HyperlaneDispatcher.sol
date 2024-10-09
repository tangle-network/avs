// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.8.0;

import {IMailbox} from "./interfaces/vendored/IMailbox.sol";
import {TypeCasts} from "./libs/TypeCasts.sol";

contract HyperlaneDispatcher {
    // Hyperlane Mailbox contract
    IMailbox public immutable mailbox;

    // Tangle domain ID
    uint32 public constant TANGLE_DOMAIN = 5845;

    // Recipient address on Tangle (should be set to the appropriate contract address)
    bytes32 public constant TANGLE_RECIPIENT = 0x0000000000000000000000001234567890123456789012345678901234567890; // Replace with actual address

    constructor(address _mailbox) {
        mailbox = IMailbox(_mailbox);
    }

    function _dispatchToTangle(address operator, bytes memory validatorKeys, bytes32 accountKey) external payable {
        bytes memory message = abi.encode(operator, validatorKeys, accountKey);

        uint256 fee = mailbox.quoteDispatch(TANGLE_DOMAIN, TANGLE_RECIPIENT, message);
        require(msg.value >= fee, "Insufficient fee");

        mailbox.dispatch{value: fee}(TANGLE_DOMAIN, TANGLE_RECIPIENT, message);

        // Refund excess fee
        if (msg.value > fee) {
            payable(msg.sender).transfer(msg.value - fee);
        }
    }

    // Allow the contract to receive ETH
    receive() external payable {}
}
