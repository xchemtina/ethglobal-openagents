// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract ProposalRegistry {
    mapping(bytes32 => bool) public anchored;

    event ProposalAnchored(bytes32 indexed artifactHash, string cid);

    function anchor(bytes32 artifactHash, string calldata cid) external {
        anchored[artifactHash] = true;
        emit ProposalAnchored(artifactHash, cid);
    }
}
