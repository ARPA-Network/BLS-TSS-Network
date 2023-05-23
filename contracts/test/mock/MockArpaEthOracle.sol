// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

contract MockArpaEthOracle {
    int256 private _weiPerUnitArpa;
    uint256 private _updatedAt;

    function setWeiPerUnitArpa(int256 weiPerUnitArpa) public {
        _weiPerUnitArpa = weiPerUnitArpa;
    }

    function setUpdatedAt(uint256 updatedAt) public {
        _updatedAt = updatedAt;
    }

    function latestRoundData()
        external
        view
        returns (uint80 roundId, int256, uint256 startedAt, uint256, uint80 answeredInRound)
    {
        return (0, _weiPerUnitArpa, 0, _updatedAt, 0);
    }
}
