// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IController {
    struct Member {
        address nodeIdAddress;
        uint256[4] partialPublicKey;
    }
    
    struct CommitResult {
        uint256 groupEpoch;
        uint256[4] publicKey;
        address[] disqualifiedNodes;
    }
    
    struct CommitCache {
        address[] nodeIdAddress;
        CommitResult commitResult;
    }
    
    struct Group {
        uint256 index;
        uint256 epoch;
        uint256 size;
        uint256 threshold;
        Member[] members;
        address[] committers;
        CommitCache[] commitCacheList;
        bool isStrictlyMajorityConsensusReached;
        uint256[4] publicKey;
    }
    
    struct CommitDkgParams {
        uint256 groupIndex;
        uint256 groupEpoch;
        bytes publicKey;
        bytes partialPublicKey;
        address[] disqualifiedNodes;
    }
    
    function getGroup(uint256 groupIndex) external view returns (Group memory);
    
    function commitDkg(CommitDkgParams memory params) external;
    
    function getCoordinator(uint256 groupIndex) external view returns (address);
}