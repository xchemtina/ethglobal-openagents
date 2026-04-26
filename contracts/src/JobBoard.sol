// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract JobBoard {
    struct Job {
        address submitter;
        string artifactCid;
        uint256 bounty;
        bool claimed;
    }

    uint256 public nextJobId = 1;
    mapping(uint256 => Job) public jobs;

    event JobPosted(uint256 indexed jobId, address indexed submitter, string artifactCid, uint256 bounty);
    event JobClaimed(uint256 indexed jobId, address indexed worker);

    function postJob(string calldata artifactCid) external payable returns (uint256 jobId) {
        jobId = nextJobId++;
        jobs[jobId] = Job(msg.sender, artifactCid, msg.value, false);
        emit JobPosted(jobId, msg.sender, artifactCid, msg.value);
    }

    function claim(uint256 jobId) external {
        require(!jobs[jobId].claimed, "claimed");
        jobs[jobId].claimed = true;
        emit JobClaimed(jobId, msg.sender);
    }
}
