// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Test} from "forge-std/Test.sol";
import {BLS} from "../src/libraries/BLS.sol";

contract BLSTest is Test {
    function testHashToPoint() public {
        uint256[2] memory p = BLS.hashToPoint("hello");

        bool onCurve = BLS.isOnCurveG1([p[0], p[1]]);

        assertTrue(onCurve);
        assertEq(p[0], 8624054983400697718141956802447871958869122227252176640970531463441248681148);
        assertEq(p[1], 4714976728303045455220568421558489936628555206298476747206964600484068714510);
    }

    function testG2() public {
        uint256 x1 = 17278932652257254326372213117531887034602515361911989965175376143830222599936;
        uint256 y1 = 20031973458929592857375281132123518018056190313544844569373014685916759857230;
        uint256 x2 = 20064204146850969181106739462941001903229194709880588954223883176426803937774;
        uint256 y2 = 14587034285780894150170698790164184681018642187371753355261524461651965258915;

        uint256[4] memory point1 = [x1, x2, y1, y2];
        uint256[4] memory point2 = [x1, x2, BLS.N - y1, BLS.N - y2];
        bool onCurve1 = BLS.isOnCurveG2(point1);
        emit log_uint(onCurve1 ? 1 : 0);
        bool onCurve2 = BLS.isOnCurveG2(point2);
        emit log_uint(onCurve2 ? 1 : 0);
    }

    function testDecompress() public {
        uint256 cx1 = 131449440775817426560367863668973187564446271105289002152564551034334957958;
        uint256 y1 = 5142471158538969335790353460140971440396771055705923842924855903685812733855;

        uint256 cx2 = 2993052591263300251421730215909578160265361713291681977422434783744805156453;
        uint256 y2 = 8236704679255897636411889564940276141365191991937272748153173274454109057806;

        uint256[2] memory uncompressed1 = BLS.decompress(cx1);
        uint256[2] memory uncompressed2 = BLS.decompress(cx2);
        assertEq(uncompressed1[1], y1);
        assertEq(uncompressed2[1], y2);
    }

    function testVerifySignature() public {
        uint256[7] memory params1 = [
            0xde7c516e867a427226866fa41566ad0eb0ff69f54e4408babd2f59c54238fa86,
            0x0000000000000000000000000000000000000000000000000000000000000001,
            0x116da8c89a0d090f3d8644ada33a5f1c8013ba7204aeca62d66d931b99afe6e7,
            0x12740934ba9615b77b6a49b06fcce83ce90d67b1d0e2a530069e3a7306569a91,
            0x076441042e77b6309644b56251f059cf14befc72ac8a6157d30924e58dc4c172,
            0x25222d9816e5f86b4a7dedd00d04acc5c979c18bd22b834ea8c6d07c0ba441db,
            0x8810fb08e61f12011197f55c2bc5e1e77576ecbf56d73794686e1940e106828e
        ];

        uint256[7] memory params2 = [
            0xde7c516e867a427226866fa41566ad0eb0ff69f54e4408babd2f59c54238fa86,
            0x0000000000000000000000000000000000000000000000000000000000000001,
            0x1a507c593ab755ddc738a62bb1edbf00de9d2e0f6829a663c53fa281ca3a296b,
            0x17bfa426fe907fb295063261d2348ad72f3b40c1aaeb8a0e31e29b341d9cc14f,
            0x247fe0adc753328cb9250964f16b77693d273892270be5cfbb4aca3b625606cc,
            0x17e4867e1df6f439500568aaa567952b5c47f3b4eb3a824fcee17000917ce1d0,
            0x2dcb14c407beb29593b6ee1d0db90642f95d23441fe7bb68f195c116563b5882
        ];

        verifySignature(params1);
        verifySignature(params2);
    }

    function verifySignature(uint256[7] memory params) public {
        bytes memory message = abi.encodePacked(params[0], params[1]);
        emit log_bytes(message);
        uint256[2] memory msgPoint = BLS.hashToPoint(message);
        emit log_uint(msgPoint[0]);
        emit log_uint(msgPoint[1]);
        bytes memory publicKey = abi.encodePacked(params[2], params[3], params[4], params[5]);
        uint256[2] memory sig = BLS.decompress(params[6]);
        emit log_uint(sig[0]);
        emit log_uint(sig[1]);

        bool res = BLS.verifySingle(sig, BLS.fromBytesPublicKey(publicKey), msgPoint);

        emit log_uint(res ? 1 : 0);
        assertTrue(res);
    }

    function testVerifyPartials() public {
        uint256[2] memory message = [
            0xde7c516e867a427226866fa41566ad0eb0ff69f54e4408babd2f59c54238fa86,
            0x0000000000000000000000000000000000000000000000000000000000000001
        ];

        uint256 sig1 = 0x9b91156f3d3159a2f86c1105fe556e08853766cfa6487132f7f74a8a77692ac5;
        uint256 sig2 = 0x927794d6470ed17d2e92feab2c1614c62fc43fe846ca2d1edb9deda274946a81;
        uint256 sig3 = 0x1b44db4e542a498529ec1256d8829e6f9380cefd4741a8846ea7df6986c3a7e9;

        bytes memory pub1 =
            hex"047b565c2e1724fda37d648746d778618f995f6635bb38a71be2f60c09ffbea011a8fe485a7a3a41c7eab1004bb1b5f90b49210173c24cb90dfe99f6c92970660b80428a38cff7734a4c853bd87b55dc2b3f850081323658326fd8468660aa170c74ee6c4fb599c426e4041fb7795164ea0dd1ac362437d3c82647705a5d13a1";
        bytes memory pub2 =
            hex"047da8ba9644377f4fd0fe43b2fd472479c144013ed7279b73c13c1cf7560c69034a73ce9107f377ba82581fe32c9ace62bb36b417c079096a4ee6cb6bec24cc2d8b5bc9d8147a73ae42c246f2250f0646b818cdf8eea116264516fbd71be0f4168bba7c6dfec34cc12f3a69d35c5b5ee635a3c1e5a1115a3c1778cd954e2939";
        bytes memory pub3 =
            hex"01989cbe9c28b6fe28b152c9654d052567e27778b5b520105b53543f183239de135b1253a42042b6a11bc70287faee74c4c6a2d0a3cc14d83ad12eb3f380be55053c30615fba4b4d09dddfab945841294b6e955777ce811d691d91c9e66f1cf72b2534eddac940455454fa797e6a7336738fa56f30742a95cb0ae7cd1a9ab883";

        uint256[2][] memory partials = new uint256[2][](3);
        partials[0] = BLS.decompress(sig1);
        partials[1] = BLS.decompress(sig2);
        partials[2] = BLS.decompress(sig3);
        uint256[4][] memory pubkeys = new uint256[4][](3);
        pubkeys[0] = BLS.fromBytesPublicKey(pub1);
        pubkeys[1] = BLS.fromBytesPublicKey(pub2);
        pubkeys[2] = BLS.fromBytesPublicKey(pub3);

        verifyPartials(partials, pubkeys, message);
    }

    function verifyPartials(uint256[2][] memory partials, uint256[4][] memory pubkeys, uint256[2] memory message)
        public
    {
        bytes memory realSeed = abi.encodePacked(message[0], message[1]);
        emit log_bytes(realSeed);
        uint256[2] memory msgPoint = BLS.hashToPoint(realSeed);
        emit log_uint(msgPoint[0]);
        emit log_uint(msgPoint[1]);

        bool res = BLS.verifyPartials(partials, pubkeys, msgPoint);

        emit log_uint(res ? 1 : 0);
        assertTrue(res);
    }
}
