// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

contract MockArpaEthOracle {
    int256 weiPerUnitArpa;
    uint256 updatedAt;

    function setWeiPerUnitArpa(int256 _weiPerUnitArpa) public {
        weiPerUnitArpa = _weiPerUnitArpa;
    }

    function setUpdatedAt(uint256 _updatedAt) public {
        updatedAt = _updatedAt;
    }

    function latestRoundData()
        external
        view
        returns (uint80 roundId, int256, uint256 startedAt, uint256, uint80 answeredInRound)
    {
        return (0, weiPerUnitArpa, 0, updatedAt, 0);
    }
}
