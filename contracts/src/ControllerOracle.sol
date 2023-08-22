// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {IERC20, SafeERC20} from "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";
import {OwnableUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/access/OwnableUpgradeable.sol";
import {Initializable} from "openzeppelin-contracts-upgradeable/contracts/proxy/utils/Initializable.sol";
import {IControllerOracle} from "./interfaces/IControllerOracle.sol";
import {IOPCrossDomainMessenger} from "./interfaces/IOPCrossDomainMessenger.sol";
import {IAdapter} from "./interfaces/IAdapter.sol";
// solhint-disable-next-line no-global-import
import "./utils/Utils.sol" as Utils;

contract ControllerOracle is Initializable, IControllerOracle, OwnableUpgradeable {
    using SafeERC20 for IERC20;

    struct GroupData {
        uint256 epoch;
        uint256 groupCount;
        mapping(uint256 => Group) groups; // group_index => Group struct
        uint256 idealNumberOfGroups;
        uint256 groupMaxCapacity;
        uint256 defaultNumberOfCommitters;
    }

    // *Constants*
    uint16 private constant _BALANCE_BASE = 1;

    // *Controller Config*
    IERC20 private _arpa;
    address private _chainMessenger;
    IOPCrossDomainMessenger private _l2CrossDomainMessenger;
    address private _adapterContractAddress;

    // *Node State Variables*
    mapping(address => uint256) private _withdrawableEths; // maps node address to withdrawable eth amount
    mapping(address => uint256) private _arpaRewards; // maps node address to arpa rewards

    // *Group Variables*
    GroupData internal _groupData;

    // *Task Variables*
    uint256 private _lastOutput;

    // *Events*
    event NodeRewarded(address indexed nodeAddress, uint256 ethAmount, uint256 arpaAmount);
    event GroupUpdated(
        uint256 epoch, uint256 indexed groupIndex, uint256 indexed groupEpoch, address indexed committer
    );

    // *Errors*
    error GroupObsolete(uint256 groupIndex, uint256 relayedGroupEpoch, uint256 currentGroupEpoch);
    error SenderNotAdapter();
    error SenderNotChainMessenger();
    error InvalidZeroAddress();

    function initialize(
        address arpa,
        address chainMessenger,
        address l2CrossDomainMessenger,
        address adapterContractAddress,
        uint256 lastOutput
    ) public initializer {
        _arpa = IERC20(arpa);
        _chainMessenger = chainMessenger;
        _l2CrossDomainMessenger = IOPCrossDomainMessenger(l2CrossDomainMessenger);
        _adapterContractAddress = adapterContractAddress;
        _lastOutput = lastOutput;

        __Ownable_init();
    }

    function updateGroup(address committer, Group memory group) external {
        if (
            msg.sender != address(_l2CrossDomainMessenger)
                || _l2CrossDomainMessenger.xDomainMessageSender() != _chainMessenger
        ) {
            revert SenderNotChainMessenger();
        }

        uint256 groupIndex = group.index;

        if (group.epoch <= _groupData.groups[groupIndex].epoch) {
            revert GroupObsolete(groupIndex, group.epoch, _groupData.groups[groupIndex].epoch);
        }

        _groupData.epoch++;

        if (_groupData.groups[groupIndex].epoch == 0) {
            _groupData.groupCount++;
        }

        _copyGroup(group);

        emit GroupUpdated(_groupData.epoch, groupIndex, group.epoch, committer);
    }

    function setChainMessenger(address chainMessenger) external onlyOwner {
        if (chainMessenger == address(0)) {
            revert InvalidZeroAddress();
        }
        _chainMessenger = chainMessenger;
    }

    function setL2CrossDomainMessenger(address l2CrossDomainMessenger) external onlyOwner {
        if (l2CrossDomainMessenger == address(0)) {
            revert InvalidZeroAddress();
        }
        _l2CrossDomainMessenger = IOPCrossDomainMessenger(l2CrossDomainMessenger);
    }

    function setAdapterContractAddress(address adapterContractAddress) external onlyOwner {
        if (adapterContractAddress == address(0)) {
            revert InvalidZeroAddress();
        }
        _adapterContractAddress = adapterContractAddress;
    }

    function nodeWithdraw(address recipient) external override(IControllerOracle) {
        if (recipient == address(0)) {
            revert InvalidZeroAddress();
        }
        uint256 ethAmount = _withdrawableEths[msg.sender];
        uint256 arpaAmount = _arpaRewards[msg.sender];
        if (arpaAmount > _BALANCE_BASE) {
            _arpaRewards[msg.sender] = _BALANCE_BASE;
            _arpa.safeTransfer(recipient, arpaAmount - _BALANCE_BASE);
        }
        if (ethAmount > _BALANCE_BASE) {
            _withdrawableEths[msg.sender] = _BALANCE_BASE;
            IAdapter(_adapterContractAddress).nodeWithdrawETH(recipient, ethAmount - _BALANCE_BASE);
        }
    }

    function addReward(address[] memory nodes, uint256 ethAmount, uint256 arpaAmount)
        public
        override(IControllerOracle)
    {
        if (msg.sender != _adapterContractAddress) {
            revert SenderNotAdapter();
        }
        for (uint256 i = 0; i < nodes.length; i++) {
            _withdrawableEths[nodes[i]] += ethAmount;
            _arpaRewards[nodes[i]] += arpaAmount;
            emit NodeRewarded(nodes[i], ethAmount, arpaAmount);
        }
    }

    function setLastOutput(uint256 lastOutput) external override(IControllerOracle) {
        if (msg.sender != _adapterContractAddress) {
            revert SenderNotAdapter();
        }
        _lastOutput = lastOutput;
    }

    function getValidGroupIndices() public view override(IControllerOracle) returns (uint256[] memory) {
        uint256[] memory groupIndices = new uint256[](_groupData.groupCount); //max length is group count
        uint256 index = 0;
        for (uint256 i = 0; i < _groupData.groupCount; i++) {
            Group memory g = _groupData.groups[i];
            if (g.isStrictlyMajorityConsensusReached) {
                groupIndices[index] = i;
                index++;
            }
        }

        return Utils.trimTrailingElements(groupIndices, index);
    }

    function getGroupEpoch() external view override(IControllerOracle) returns (uint256) {
        return _groupData.epoch;
    }

    function getGroupCount() external view override(IControllerOracle) returns (uint256) {
        return _groupData.groupCount;
    }

    function getGroup(uint256 groupIndex) public view override(IControllerOracle) returns (Group memory) {
        return _groupData.groups[groupIndex];
    }

    function getGroupThreshold(uint256 groupIndex) public view override(IControllerOracle) returns (uint256, uint256) {
        return (_groupData.groups[groupIndex].threshold, _groupData.groups[groupIndex].size);
    }

    function getMember(uint256 groupIndex, uint256 memberIndex)
        public
        view
        override(IControllerOracle)
        returns (Member memory)
    {
        return _groupData.groups[groupIndex].members[memberIndex];
    }

    function getBelongingGroup(address nodeAddress)
        external
        view
        override(IControllerOracle)
        returns (int256, int256)
    {
        for (uint256 i = 0; i < _groupData.groupCount; i++) {
            int256 memberIndex = _getMemberIndexByAddress(i, nodeAddress);
            if (memberIndex != -1) {
                return (int256(i), memberIndex);
            }
        }
        return (-1, -1);
    }

    function getNodeWithdrawableTokens(address nodeAddress)
        public
        view
        override(IControllerOracle)
        returns (uint256, uint256)
    {
        return (
            _withdrawableEths[nodeAddress] == 0 ? 0 : (_withdrawableEths[nodeAddress] - _BALANCE_BASE),
            _arpaRewards[nodeAddress] == 0 ? 0 : (_arpaRewards[nodeAddress] - _BALANCE_BASE)
        );
    }

    function getLastOutput() external view returns (uint256) {
        return _lastOutput;
    }

    function _copyGroup(Group memory group) internal {
        Group storage g = _groupData.groups[group.index];
        g.index = group.index;
        g.epoch = group.epoch;
        g.threshold = group.threshold;
        g.size = group.size;
        g.isStrictlyMajorityConsensusReached = group.isStrictlyMajorityConsensusReached;
        delete g.members;
        for (uint256 i = 0; i < group.size; i++) {
            address memberAddress = group.members[i].nodeIdAddress;
            g.members.push(group.members[i]);
            // Initialize withdrawable eths and arpa rewards to save gas for adapter call
            if (_withdrawableEths[memberAddress] == 0 && _arpaRewards[memberAddress] == 0) {
                _withdrawableEths[memberAddress] = _BALANCE_BASE;
                _arpaRewards[memberAddress] = _BALANCE_BASE;
            }
        }
        g.committers = group.committers;
        g.publicKey = group.publicKey;
    }

    function _getMemberIndexByAddress(uint256 groupIndex, address nodeAddress) internal view returns (int256) {
        Group memory g = _groupData.groups[groupIndex];
        for (uint256 i = 0; i < g.members.length; i++) {
            if (g.members[i].nodeIdAddress == nodeAddress) {
                return int256(i);
            }
        }
        return -1;
    }
}
