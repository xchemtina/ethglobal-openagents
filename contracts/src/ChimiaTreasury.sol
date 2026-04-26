// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract ChimiaTreasury {
    address public governor;

    event GovernorSet(address indexed governor);
    event Payment(address indexed to, uint256 amount);

    constructor(address initialGovernor) {
        governor = initialGovernor;
        emit GovernorSet(initialGovernor);
    }

    receive() external payable {}

    function setGovernor(address newGovernor) external {
        require(msg.sender == governor, "not governor");
        governor = newGovernor;
        emit GovernorSet(newGovernor);
    }

    function pay(address payable to, uint256 amount) external {
        require(msg.sender == governor, "not governor");
        to.transfer(amount);
        emit Payment(to, amount);
    }
}
