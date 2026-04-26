// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract AgentReputation {
    mapping(address => mapping(bytes32 => uint256)) public scoreOf;

    event ScoreSet(address indexed agent, bytes32 indexed domain, uint256 score);

    function setScore(address agent, bytes32 domain, uint256 newScore) external {
        scoreOf[agent][domain] = newScore;
        emit ScoreSet(agent, domain, newScore);
    }

    function score(address agent, bytes32 domain) external view returns (uint256) {
        return scoreOf[agent][domain];
    }
}
