// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "openzeppelin-contracts/contracts/access/Ownable.sol";
import "./Adapter.sol";
import "./interfaces/IController.sol";

contract ControllerProxy {
    struct ModifiedDkgData {
        bytes publicKey;
        bytes partialPublicKey;
        address[] disqualifiedNodes;
        bool[3] modifyFlag;
    }

    mapping(address => ModifiedDkgData) modifyDkgData;

    constructor(address controller) {
        setImplementation(controller);
    }

    /**
     * @dev Storage slot with the address of the current implementation.
     * This is the keccak-256 hash of "eip1967.proxy.implementation" subtracted by 1, and is
     * validated in the constructor.
     */
    bytes32 internal constant _IMPLEMENTATION_SLOT = 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc;

    function implementation() public view returns (address r) {
        assembly {
            r := sload(_IMPLEMENTATION_SLOT)
        }
    }

    function setImplementation(address newImplementation) public {
        assembly {
            sstore(_IMPLEMENTATION_SLOT, newImplementation)
        }
    }

    /**
     * @dev Delegates the current call to `implementation`.
     *
     * This function does not return to its internal call site, it will return directly to the external caller.
     */
    function _delegate(address _implementation) internal virtual {
        assembly {
            // Copy msg.data. We take full control of memory in this inline assembly
            // block because it will not return to Solidity code. We overwrite the
            // Solidity scratch pad at memory position 0.
            calldatacopy(0, 0, calldatasize())

            // Call the implementation.
            // out and outsize are 0 because we don't know the size yet.
            let result := delegatecall(gas(), _implementation, 0, calldatasize(), 0, 0)

            // Copy the returned data.
            returndatacopy(0, 0, returndatasize())

            switch result
            // delegatecall returns 0 on error.
            case 0 { revert(0, returndatasize()) }
            default { return(0, returndatasize()) }
        }
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

        (bool success,) = implementation().delegatecall(abi.encodeWithSelector(IController.commitDkg.selector, params));
        require(success, "modified delegatecall reverted");
    }

    fallback() external payable {
        _delegate(implementation());
    }

    receive() external payable {
        _delegate(implementation());
    }
}
