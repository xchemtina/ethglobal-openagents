// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract CapabilityRegistry {
    struct AgentToken {
        address owner;
        string ensName;
        bytes32 capabilityFingerprint;
        string headArtifactCid;
        string encryptedStateCid;
    }

    uint256 public nextTokenId = 1;
    mapping(uint256 => AgentToken) public tokens;
    mapping(uint256 => address) public ownerOf;

    event Minted(uint256 indexed tokenId, address indexed owner, string ensName, bytes32 capabilityFingerprint);
    event HeadArtifactUpdated(uint256 indexed tokenId, string headArtifactCid);

    function mint(string calldata ensName, bytes32 capabilityFingerprint, string calldata encryptedStateCid) external returns (uint256 tokenId) {
        tokenId = nextTokenId++;
        tokens[tokenId] = AgentToken(msg.sender, ensName, capabilityFingerprint, "", encryptedStateCid);
        ownerOf[tokenId] = msg.sender;
        emit Minted(tokenId, msg.sender, ensName, capabilityFingerprint);
    }

    function updateHeadArtifact(uint256 tokenId, string calldata headArtifactCid) external {
        require(ownerOf[tokenId] == msg.sender, "not owner");
        tokens[tokenId].headArtifactCid = headArtifactCid;
        emit HeadArtifactUpdated(tokenId, headArtifactCid);
    }
}
