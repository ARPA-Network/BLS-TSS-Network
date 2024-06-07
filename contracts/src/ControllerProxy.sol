// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "openzeppelin-contracts/contracts/access/Ownable.sol";
import "./Adapter.sol";
import "./interfaces/IController.sol";
import {ERC1967Proxy} from "openzeppelin-contracts/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract ControllerProxy is ERC1967Proxy {
    struct ModifiedDkgData {
        bytes publicKey;
        bytes partialPublicKey;
        address[] disqualifiedNodes;
        bool[3] modifyFlag;
    }

    mapping(address => ModifiedDkgData) modifyDkgData;

    constructor(address _logic, bytes memory _data) ERC1967Proxy(_logic, _data){
    }

    function setModifiedPublicKey(address node, bytes calldata publicKey) external {
        modifyDkgData[node].publicKey = publicKey;
        modifyDkgData[node].modifyFlag[0] = true;
    }

    function setModifiedPartialPublicKey(address node, bytes calldata partialPublicKey) external {
        modifyDkgData[node].partialPublicKey = partialPublicKey;
        modifyDkgData[node].modifyFlag[1] = true;
    }

    function setModifiedDisqualifiedNodes(address node, address[] calldata disqualifiedNodes) external {
        modifyDkgData[node].disqualifiedNodes = disqualifiedNodes;
        modifyDkgData[node].modifyFlag[2] = true;
    }

    function clearModifiedFlag(address node) external {
        modifyDkgData[node].modifyFlag[0] = false;
        modifyDkgData[node].modifyFlag[1] = false;
        modifyDkgData[node].modifyFlag[2] = false;
    }

    function getModifiedDkgData(address node) external view returns (ModifiedDkgData memory) {
        return modifyDkgData[node];
    }

    event testProxy(address test);

    function commitDkg(IController.CommitDkgParams memory params) external {
        bytes memory publicKeyModified = params.publicKey;
        bytes memory partialPublicKeyModified = params.partialPublicKey;
        address[] memory disqualifiedNodesModified = params.disqualifiedNodes;
        if (modifyDkgData[msg.sender].modifyFlag[0]) {
            emit testProxy(msg.sender);
            publicKeyModified = modifyDkgData[msg.sender].publicKey;
        }
        if (modifyDkgData[msg.sender].modifyFlag[1]) {
            partialPublicKeyModified = modifyDkgData[msg.sender].partialPublicKey;
        }
        if (modifyDkgData[msg.sender].modifyFlag[2]) {
            disqualifiedNodesModified = modifyDkgData[msg.sender].disqualifiedNodes;
        }

        params.publicKey = publicKeyModified;
        params.partialPublicKey = partialPublicKeyModified;
        params.disqualifiedNodes = disqualifiedNodesModified;

        (bool success,) = _implementation().delegatecall(abi.encodeWithSelector(IController.commitDkg.selector, params));
        require(success, "modified delegatecall reverted");
    }
}
