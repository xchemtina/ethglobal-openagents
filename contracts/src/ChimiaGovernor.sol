// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface IAgentReputation {
    function score(address agent, bytes32 domain) external view returns (uint256);
}

interface IProposalRegistry {
    function anchored(bytes32 artifactHash) external view returns (bool);
}

contract ChimiaGovernor {
    IAgentReputation public reputation;
    IProposalRegistry public proposals;
    uint256 public quorum;

    event ExecutionAuthorized(bytes32 indexed proposalArtifactHash, address indexed target, bytes32 calldataHash);

    constructor(address reputation_, address proposals_, uint256 quorum_) {
        reputation = IAgentReputation(reputation_);
        proposals = IProposalRegistry(proposals_);
        quorum = quorum_;
    }

    function authorizeExecution(bytes32 proposalArtifactHash, address target, bytes calldata data) external returns (bool) {
        require(proposals.anchored(proposalArtifactHash), "proposal not anchored");
        require(target != address(0), "bad target");
        emit ExecutionAuthorized(proposalArtifactHash, target, keccak256(data));
        return true;
    }
}
