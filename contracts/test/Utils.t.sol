// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Test} from "forge-std/Test.sol";
import {Strings} from "openzeppelin-contracts/contracts/utils/Strings.sol";
// solhint-disable-next-line no-global-import
import "../src/utils/Utils.sol" as Utils;

contract UtilsTest is Test {
    string private _mnemonic = "test test test test test test test test test test test junk";

    function testPickRandomIndex() public {
        uint256 seed = 2459565876494606882;
        uint256[] memory indices = new uint256[](10);
        for (uint256 i = 0; i < 10; i++) {
            indices[i] = i;
        }
        uint256[] memory membersToMove = Utils.pickRandomIndex(seed, indices, 5);
        for (uint256 i = 0; i < membersToMove.length; i++) {
            emit log_uint(membersToMove[i]);
        }
    }

    function testPrivateKeyByDefaultMnemonic() public {
        for (uint32 i = 0; i < 20; i++) {
            uint256 sk = vm.deriveKey(_mnemonic, i);
            emit log_named_bytes(Strings.toString(i), abi.encodePacked(sk));
            address addr = vm.rememberKey(sk);
            emit log_named_address(Strings.toString(i), addr);
        }
    }
}
