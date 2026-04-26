// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract SettlementEscrow {
    struct Intent {
        address payer;
        address recipient;
        uint256 amount;
        string artifactCid;
        bool released;
    }

    uint256 public nextIntentId = 1;
    mapping(uint256 => Intent) public intents;

    event IntentOpened(uint256 indexed intentId, address indexed payer, address indexed recipient, uint256 amount, string artifactCid);
    event Released(uint256 indexed intentId);

    function open(address recipient, string calldata artifactCid) external payable returns (uint256 intentId) {
        intentId = nextIntentId++;
        intents[intentId] = Intent(msg.sender, recipient, msg.value, artifactCid, false);
        emit IntentOpened(intentId, msg.sender, recipient, msg.value, artifactCid);
    }

    function release(uint256 intentId) external {
        Intent storage intent = intents[intentId];
        require(msg.sender == intent.payer, "not payer");
        require(!intent.released, "released");
        intent.released = true;
        payable(intent.recipient).transfer(intent.amount);
        emit Released(intentId);
    }
}
