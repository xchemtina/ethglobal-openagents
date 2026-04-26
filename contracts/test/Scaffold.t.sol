// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../src/CapabilityRegistry.sol";
import "../src/AgentReputation.sol";
import "../src/ProposalRegistry.sol";

contract ScaffoldTest {
    function testMintCapabilityToken() public {
        CapabilityRegistry registry = new CapabilityRegistry();
        uint256 tokenId = registry.mint("sofia.chimiaclaw.eth", bytes32(uint256(1)), "encrypted-state-cid");
        require(tokenId == 1, "bad token id");
    }

    function testSetReputation() public {
        AgentReputation reputation = new AgentReputation();
        reputation.setScore(address(this), keccak256("DFT"), 42);
        require(reputation.score(address(this), keccak256("DFT")) == 42, "bad score");
    }

    function testAnchorProposal() public {
        ProposalRegistry registry = new ProposalRegistry();
        bytes32 hash = keccak256("proposal");
        registry.anchor(hash, "cid");
        require(registry.anchored(hash), "not anchored");
    }
}
