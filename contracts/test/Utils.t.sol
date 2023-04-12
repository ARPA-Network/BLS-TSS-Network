// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "forge-std/Test.sol";
import "src/utils/Utils.sol";

contract UtilsTest is Test {
    string mnemonic = "test test test test test test test test test test test junk";

    function setUp() public {}

    function testPickRandomIndex() public {
        uint256 seed = 2459565876494606882;
        uint256[] memory indices = new uint256[](10);
        for (uint256 i = 0; i < 10; i++) {
            indices[i] = i;
        }
        uint256[] memory membersToMove = pickRandomIndex(seed, indices, 5);
        for (uint256 i = 0; i < membersToMove.length; i++) {
            emit log_uint(membersToMove[i]);
        }
    }

    function testPrivateKeyByDefaultMnemonic() public {
        for (uint32 i = 10; i < 10 + 5; i++) {
            uint256 sk = vm.deriveKey(mnemonic, i);
            emit log_bytes(abi.encodePacked(sk));
            address addr = vm.rememberKey(sk);
            emit log_address(addr);
        }
    }
}
