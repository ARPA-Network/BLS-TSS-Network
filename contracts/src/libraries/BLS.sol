// SPDX-License-Identifier: LGPL 3.0
pragma solidity ^0.8.15;

import {BN256G2} from "./BN256G2.sol";

library BLS {
    // Field order
    uint256 constant N =
        21888242871839275222246405745257275088696311157297823662689037894645226208583;

    // curve order
    uint256 constant r =
        21888242871839275222246405745257275088548364400416034343698204186575808495617;

    // Genarator of G1 - according to py_ecc implementation
    uint256 constant G1x = 1;
    uint256 constant G1y = 2;

    // Constant `a` of EC equation
    uint256 internal constant AA = 0;
    // Constant `b` of EC equation
    uint256 internal constant BB = 3;

    // Negated genarator of G1
    uint256 constant nG1x = 1;
    uint256 constant nG1y =
        21888242871839275222246405745257275088696311157297823662689037894645226208581;

    // Genarator of G2
    uint256 constant G2x1 =
        11559732032986387107991004021392285783925812861821192530917403151452391805634;
    uint256 constant G2x0 =
        10857046999023057135944570762232829481370756359578518086990519993285655852781;
    uint256 constant G2y1 =
        4082367875863433681332203403145435568316851327593401208105741076214120093531;
    uint256 constant G2y0 =
        8495653923123431417604973247489272438418190587263600148770280649306958101930;

    // Negated genarator of G2
    uint256 constant nG2x1 =
        11559732032986387107991004021392285783925812861821192530917403151452391805634;
    uint256 constant nG2x0 =
        10857046999023057135944570762232829481370756359578518086990519993285655852781;
    uint256 constant nG2y1 =
        17805874995975841540914202342111839520379459829704422454583296818431106115052;
    uint256 constant nG2y0 =
        13392588948715843804641432497768002650278120570034223513918757245338268106653;

    uint256 constant FIELD_MASK =
        0x3fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;

    // curve param x. We use s here to prevent confusion with coordinate x.
    uint256 constant s = 4965661367192848881;

    //eps^((p-1)/3)
    uint256 internal constant epsExp0x0 =
        21575463638280843010398324269430826099269044274347216827212613867836435027261;
    uint256 internal constant epsExp0x1 =
        10307601595873709700152284273816112264069230130616436755625194854815875713954;

    //eps^((p-1)/2)
    uint256 internal constant epsExp1x0 =
        2821565182194536844548159561693502659359617185244120367078079554186484126554;
    uint256 internal constant epsExp1x1 =
        3505843767911556378687030309984248845540243509899259641013678093033130930403;

    error MustNotBeInfinity();

    function verifySingle(
        uint256[2] memory signature,
        uint256[4] memory pubkey,
        uint256[2] memory message
    ) internal view returns (bool) {
        uint256[12] memory input = [
            signature[0],
            signature[1],
            nG2x1,
            nG2x0,
            nG2y1,
            nG2y0,
            message[0],
            message[1],
            pubkey[1],
            pubkey[0],
            pubkey[3],
            pubkey[2]
        ];
        uint256[1] memory out;
        bool success;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            success := staticcall(sub(gas(), 2000), 8, input, 384, out, 0x20)
            switch success
            case 0 {
                invalid()
            }
        }
        require(success, "");
        return out[0] != 0;
    }

    function verifyMultiple(
        uint256[2] memory signature,
        uint256[4][] memory pubkeys,
        uint256[2][] memory messages
    ) internal view returns (bool) {
        uint256 size = pubkeys.length;
        require(size > 0, "BLS: number of public key is zero");
        require(
            size == messages.length,
            "BLS: number of public keys and messages must be equal"
        );
        uint256 inputSize = (size + 1) * 6;
        uint256[] memory input = new uint256[](inputSize);
        input[0] = signature[0];
        input[1] = signature[1];
        input[2] = nG2x1;
        input[3] = nG2x0;
        input[4] = nG2y1;
        input[5] = nG2y0;
        for (uint256 i = 0; i < size; i++) {
            input[i * 6 + 6] = messages[i][0];
            input[i * 6 + 7] = messages[i][1];
            input[i * 6 + 8] = pubkeys[i][1];
            input[i * 6 + 9] = pubkeys[i][0];
            input[i * 6 + 10] = pubkeys[i][3];
            input[i * 6 + 11] = pubkeys[i][2];
        }
        uint256[1] memory out;
        bool success;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            success := staticcall(
                sub(gas(), 2000),
                8,
                add(input, 0x20),
                mul(inputSize, 0x20),
                out,
                0x20
            )
            switch success
            case 0 {
                invalid()
            }
        }
        require(success, "");
        return out[0] != 0;
    }

    function hashToPoint(bytes memory data)
        internal
        view
        returns (uint256[2] memory p)
    {
        bool found;
        bytes32 candidateHash = keccak256(data);
        while (true) {
            (p, found) = mapToPoint(candidateHash);
            if (found) {
                break;
            }
            candidateHash = keccak256(bytes.concat(candidateHash));
        }
    }

    function mapToPoint(bytes32 _x)
        internal
        view
        returns (uint256[2] memory p, bool found)
    {
        uint256 y;
        uint256 x = uint256(_x) % N;
        (y, found) = deriveYOnG1(x);
        if (found) {
            p[0] = x;
            p[1] = y > N / 2 ? N - y : y;
        }
    }

    function deriveYOnG1(uint256 x) internal view returns (uint256, bool) {
        uint256 y;
        y = mulmod(x, x, N);
        y = mulmod(y, x, N);
        y = addmod(y, 3, N);
        return sqrt(y);
    }

    function isValidPublicKey(uint256[4] memory publicKey)
        internal
        pure
        returns (bool)
    {
        if (
            (publicKey[0] >= N) ||
            (publicKey[1] >= N) ||
            (publicKey[2] >= N || (publicKey[3] >= N))
        ) {
            return false;
        } else {
            return isOnCurveG2(publicKey);
        }
    }

    function isValidSignature(uint256[2] memory signature)
        internal
        pure
        returns (bool)
    {
        if ((signature[0] >= N) || (signature[1] >= N)) {
            return false;
        } else {
            return isOnCurveG1(signature);
        }
    }

    function fromBytesPublicKey(bytes memory point)
        internal
        pure
        returns (uint256[4] memory pubkey)
    {
        require(point.length == 128, "invalid encoding");
        uint256 x0;
        uint256 x1;
        uint256 y0;
        uint256 y1;
        assembly {
            x0 := mload(add(point, 32))
            x1 := mload(add(point, 64))
            y0 := mload(add(point, 96))
            y1 := mload(add(point, 128))
        }
        pubkey = [x0, x1, y0, y1];
    }

    // function pubkeyToUncompressed(uint256[2] memory compressed)
    //     internal
    //     pure
    //     returns (uint256[4] memory uncompressed)
    // {
    //     uint256 decision = compressed[1] >> 254;
    //     if (decision & 1 == 1) {
    //         revert MustNotBeInfinity();
    //     }
    //     uint256 x0 = compressed[0];
    //     uint256 x1 = compressed[1] & FIELD_MASK;
    //     uint256 y0 = deriveY(2, x0);
    //     uint256 y1 = deriveY(2, x1);
    //     if (((decision >> 1) ^ (y1 > N / 2 ? 1 : 0)) == 1) {
    //         y0 = N - y0;
    //         y1 = N - y1;
    //     }
    //     uncompressed[0] = x0;
    //     uncompressed[1] = x1;
    //     uncompressed[2] = y0;
    //     uncompressed[3] = y1;
    // }

    function signatureToUncompressed(uint256 compressed)
        internal
        view
        returns (uint256[2] memory uncompressed)
    {
        uint256 x = compressed & FIELD_MASK;
        uint256 decision = compressed >> 254;
        if (decision & 1 == 1) {
            revert MustNotBeInfinity();
        }
        uint256 y;
        (y, ) = deriveYOnG1(x);

        if (((decision >> 1) ^ (y > N / 2 ? 1 : 0)) == 1) {
            y = N - y;
        }
        return [x, y];
    }

    function isValidCompressedPublicKey(uint256[2] memory publicKey)
        internal
        view
        returns (bool)
    {
        uint256 x0 = publicKey[0];
        uint256 x1 = publicKey[1] & FIELD_MASK;
        if ((x0 >= N) || (x1 >= N)) {
            return false;
        } else if ((x0 == 0) && (x1 == 0)) {
            return false;
        } else {
            return isOnCurveG2([x0, x1]);
        }
    }

    function isValidCompressedSignature(uint256 signature)
        internal
        view
        returns (bool)
    {
        uint256 x = signature & FIELD_MASK;
        if (x >= N) {
            return false;
        } else if (x == 0) {
            return false;
        }
        return isOnCurveG1(x);
    }

    function isOnCurveG1(uint256[2] memory point)
        internal
        pure
        returns (bool _isOnCurve)
    {
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            let t0 := mload(point)
            let t1 := mload(add(point, 32))
            let t2 := mulmod(t0, t0, N)
            t2 := mulmod(t2, t0, N)
            t2 := addmod(t2, 3, N)
            t1 := mulmod(t1, t1, N)
            _isOnCurve := eq(t1, t2)
        }
    }

    function isOnCurveG1(uint256 x) internal view returns (bool _isOnCurve) {
        bool callSuccess;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            let t0 := x
            let t1 := mulmod(t0, t0, N)
            t1 := mulmod(t1, t0, N)
            // x ^ 3 + b
            t1 := addmod(t1, 3, N)

            let freemem := mload(0x40)
            mstore(freemem, 0x20)
            mstore(add(freemem, 0x20), 0x20)
            mstore(add(freemem, 0x40), 0x20)
            mstore(add(freemem, 0x60), t1)
            // (N - 1) / 2 = 0x183227397098d014dc2822db40c0ac2ecbc0b548b438e5469e10460b6c3e7ea3
            mstore(
                add(freemem, 0x80),
                0x183227397098d014dc2822db40c0ac2ecbc0b548b438e5469e10460b6c3e7ea3
            )
            // N = 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
            mstore(
                add(freemem, 0xA0),
                0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
            )
            callSuccess := staticcall(
                sub(gas(), 2000),
                5,
                freemem,
                0xC0,
                freemem,
                0x20
            )
            _isOnCurve := eq(1, mload(freemem))
        }
    }

    function isOnCurveG2(uint256[4] memory point)
        internal
        pure
        returns (bool _isOnCurve)
    {
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            // x0, x1
            let t0 := mload(point)
            let t1 := mload(add(point, 32))
            // x0 ^ 2
            let t2 := mulmod(t0, t0, N)
            // x1 ^ 2
            let t3 := mulmod(t1, t1, N)
            // 3 * x0 ^ 2
            let t4 := add(add(t2, t2), t2)
            // 3 * x1 ^ 2
            let t5 := addmod(add(t3, t3), t3, N)
            // x0 * (x0 ^ 2 - 3 * x1 ^ 2)
            t2 := mulmod(add(t2, sub(N, t5)), t0, N)
            // x1 * (3 * x0 ^ 2 - x1 ^ 2)
            t3 := mulmod(add(t4, sub(N, t3)), t1, N)

            // x ^ 3 + b
            t0 := addmod(
                t2,
                0x2b149d40ceb8aaae81be18991be06ac3b5b4c5e559dbefa33267e6dc24a138e5,
                N
            )
            t1 := addmod(
                t3,
                0x009713b03af0fed4cd2cafadeed8fdf4a74fa084e52d1852e4a2bd0685c315d2,
                N
            )

            // y0, y1
            t2 := mload(add(point, 64))
            t3 := mload(add(point, 96))
            // y ^ 2
            t4 := mulmod(addmod(t2, t3, N), addmod(t2, sub(N, t3), N), N)
            t3 := mulmod(shl(1, t2), t3, N)

            // y ^ 2 == x ^ 3 + b
            _isOnCurve := and(eq(t0, t4), eq(t1, t3))
        }
    }

    function isOnCurveG2(uint256[2] memory x)
        internal
        view
        returns (bool _isOnCurve)
    {
        bool callSuccess;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            // x0, x1
            let t0 := mload(add(x, 0))
            let t1 := mload(add(x, 32))
            // x0 ^ 2
            let t2 := mulmod(t0, t0, N)
            // x1 ^ 2
            let t3 := mulmod(t1, t1, N)
            // 3 * x0 ^ 2
            let t4 := add(add(t2, t2), t2)
            // 3 * x1 ^ 2
            let t5 := addmod(add(t3, t3), t3, N)
            // x0 * (x0 ^ 2 - 3 * x1 ^ 2)
            t2 := mulmod(add(t2, sub(N, t5)), t0, N)
            // x1 * (3 * x0 ^ 2 - x1 ^ 2)
            t3 := mulmod(add(t4, sub(N, t3)), t1, N)
            // x ^ 3 + b
            t0 := add(
                t2,
                0x2b149d40ceb8aaae81be18991be06ac3b5b4c5e559dbefa33267e6dc24a138e5
            )
            t1 := add(
                t3,
                0x009713b03af0fed4cd2cafadeed8fdf4a74fa084e52d1852e4a2bd0685c315d2
            )

            // is non residue ?
            t0 := addmod(mulmod(t0, t0, N), mulmod(t1, t1, N), N)
            let freemem := mload(0x40)
            mstore(freemem, 0x20)
            mstore(add(freemem, 0x20), 0x20)
            mstore(add(freemem, 0x40), 0x20)
            mstore(add(freemem, 0x60), t0)
            // (N - 1) / 2 = 0x183227397098d014dc2822db40c0ac2ecbc0b548b438e5469e10460b6c3e7ea3
            mstore(
                add(freemem, 0x80),
                0x183227397098d014dc2822db40c0ac2ecbc0b548b438e5469e10460b6c3e7ea3
            )
            // N = 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
            mstore(
                add(freemem, 0xA0),
                0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
            )
            callSuccess := staticcall(
                sub(gas(), 2000),
                5,
                freemem,
                0xC0,
                freemem,
                0x20
            )
            _isOnCurve := eq(1, mload(freemem))
        }
    }

    // function isOnSubgroupG2Naive(uint256[4] memory point) internal view returns (bool) {
    //     uint256 t0;
    //     uint256 t1;
    //     uint256 t2;
    //     uint256 t3;
    //     // solium-disable-next-line security/no-inline-assembly
    //     assembly {
    //         t0 := mload(add(point, 0))
    //         t1 := mload(add(point, 32))
    //         // y0, y1
    //         t2 := mload(add(point, 64))
    //         t3 := mload(add(point, 96))
    //     }
    //     uint256 xx;
    //     uint256 xy;
    //     uint256 yx;
    //     uint256 yy;

    //     (xx, xy, yx, yy) = BN256G2.ECTwistMul(r, t0, t1, t2, t3);
    //     return xx == 0 && xy == 0 && yx == 0 && yy == 0;
    // }

    function isNonResidueFP(uint256 e)
        internal
        view
        returns (bool isNonResidue)
    {
        bool callSuccess;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            let freemem := mload(0x40)
            mstore(freemem, 0x20)
            mstore(add(freemem, 0x20), 0x20)
            mstore(add(freemem, 0x40), 0x20)
            mstore(add(freemem, 0x60), e)
            // (N - 1) / 2 = 0x183227397098d014dc2822db40c0ac2ecbc0b548b438e5469e10460b6c3e7ea3
            mstore(
                add(freemem, 0x80),
                0x183227397098d014dc2822db40c0ac2ecbc0b548b438e5469e10460b6c3e7ea3
            )
            // N = 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
            mstore(
                add(freemem, 0xA0),
                0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
            )
            callSuccess := staticcall(
                sub(gas(), 2000),
                5,
                freemem,
                0xC0,
                freemem,
                0x20
            )
            isNonResidue := eq(1, mload(freemem))
        }
        require(callSuccess, "BLS: isNonResidueFP modexp call failed");
        return !isNonResidue;
    }

    function isNonResidueFP2(uint256[2] memory e)
        internal
        view
        returns (bool isNonResidue)
    {
        uint256 a = addmod(mulmod(e[0], e[0], N), mulmod(e[1], e[1], N), N);
        bool callSuccess;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            let freemem := mload(0x40)
            mstore(freemem, 0x20)
            mstore(add(freemem, 0x20), 0x20)
            mstore(add(freemem, 0x40), 0x20)
            mstore(add(freemem, 0x60), a)
            // (N - 1) / 2 = 0x183227397098d014dc2822db40c0ac2ecbc0b548b438e5469e10460b6c3e7ea3
            mstore(
                add(freemem, 0x80),
                0x183227397098d014dc2822db40c0ac2ecbc0b548b438e5469e10460b6c3e7ea3
            )
            // N = 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
            mstore(
                add(freemem, 0xA0),
                0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
            )
            callSuccess := staticcall(
                sub(gas(), 2000),
                5,
                freemem,
                0xC0,
                freemem,
                0x20
            )
            isNonResidue := eq(1, mload(freemem))
        }
        require(callSuccess, "BLS: isNonResidueFP2 modexp call failed");
        return !isNonResidue;
    }

    function sqrt(uint256 xx) internal view returns (uint256 x, bool hasRoot) {
        bool callSuccess;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            let freemem := mload(0x40)
            mstore(freemem, 0x20)
            mstore(add(freemem, 0x20), 0x20)
            mstore(add(freemem, 0x40), 0x20)
            mstore(add(freemem, 0x60), xx)
            // (N + 1) / 4 = 0xc19139cb84c680a6e14116da060561765e05aa45a1c72a34f082305b61f3f52
            mstore(
                add(freemem, 0x80),
                0xc19139cb84c680a6e14116da060561765e05aa45a1c72a34f082305b61f3f52
            )
            // N = 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
            mstore(
                add(freemem, 0xA0),
                0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
            )
            callSuccess := staticcall(
                sub(gas(), 2000),
                5,
                freemem,
                0xC0,
                freemem,
                0x20
            )
            x := mload(freemem)
            hasRoot := eq(xx, mulmod(x, x, N))
        }
        require(callSuccess, "BLS: sqrt modexp call failed");
    }

    function endomorphism(
        uint256 xx,
        uint256 xy,
        uint256 yx,
        uint256 yy
    )
        internal
        pure
        returns (
            uint256,
            uint256,
            uint256,
            uint256
        )
    {
        // x coordinate endomorphism
        // (xx, N - xy) is the conjugate of (xx, xy)
        (uint256 xxe, uint256 xye) = BN256G2._FQ2Mul(
            epsExp0x0,
            epsExp0x1,
            xx,
            N - xy
        );
        // y coordinate endomorphism
        // (yx, N - yy) is the conjugate of (yx, yy)
        (uint256 yxe, uint256 yye) = BN256G2._FQ2Mul(
            epsExp1x0,
            epsExp1x1,
            yx,
            N - yy
        );

        return (xxe, xye, yxe, yye);
    }

    /// Using https://eprint.iacr.org/2022/348.pdf
    // function isOnSubgroupG2DLZZ(uint256[4] memory point) internal view returns (bool) {
    //     uint256 t0;
    //     uint256 t1;
    //     uint256 t2;
    //     uint256 t3;
    //     // solium-disable-next-line security/no-inline-assembly
    //     assembly {
    //         t0 := mload(add(point, 0))
    //         t1 := mload(add(point, 32))
    //         // y0, y1
    //         t2 := mload(add(point, 64))
    //         t3 := mload(add(point, 96))
    //     }

    //     uint256 xx;
    //     uint256 xy;
    //     uint256 yx;
    //     uint256 yy;
    //     //s*P
    //     (xx, xy, yx, yy) = BN256G2.ECTwistMul(s, t0, t1, t2, t3);

    //     uint256 xx0;
    //     uint256 xy0;
    //     uint256 yx0;
    //     uint256 yy0;
    //     //(s+1)P
    //     (xx0, xy0, yx0, yy0) = BN256G2.ECTwistAdd(t0, t1, t2, t3, xx, xy, yx, yy);

    //     uint256[4] memory end0;
    //     //phi(sP)
    //     (end0[0], end0[1], end0[2], end0[3]) = endomorphism(xx, xy, yx, yy);
    //     uint256[4] memory end1;
    //     //phi^2(sP)
    //     (end1[0], end1[1], end1[2], end1[3]) = endomorphism(end0[0], end0[1], end0[2], end0[3]);
    //     //(s+1)P + phi(sP)
    //     (xx0, xy0, yx0, yy0) = BN256G2.ECTwistAdd(xx0, xy0, yx0, yy0, end0[0], end0[1], end0[2], end0[3]);
    //     //(s+1)P + phi(sP) + phi^2(sP)
    //     (xx0, xy0, yx0, yy0) = BN256G2.ECTwistAdd(xx0, xy0, yx0, yy0, end1[0], end1[1], end1[2], end1[3]);
    //     //2sP
    //     (xx, xy, yx, yy) = BN256G2.ECTwistAdd(xx, xy, yx, yy, xx, xy, yx, yy);
    //     //phi^3(2sP)
    //     (end0[0], end0[1], end0[2], end0[3]) = endomorphism(xx, xy, yx, yy);
    //     (end0[0], end0[1], end0[2], end0[3]) = endomorphism(end0[0], end0[1], end0[2], end0[3]);
    //     (end0[0], end0[1], end0[2], end0[3]) = endomorphism(end0[0], end0[1], end0[2], end0[3]);

    //     return xx0 == end0[0] && xy0 == end0[1] && yx0 == end0[2] && yy0 == end0[3];
    // }

    // Helped aggregation : https://geometry.xyz/notebook/Optimized-BLS-multisignatures-on-EVM
    // Making verification of multisignatures efficient
    // each signer submits two public keys(in G1&G2) corresponding to their secret key

    /// @notice verify if both public keys have the same secret
    /// @dev not sufficient to check for rogue-key attacks, use verifyHelpedAggregationPublicKeysRec instead
    /// @param pubkeyG1 public key in G1
    /// @param pubkeyG2 public key in G2
    function verifyHelpedAggregationPublicKeys(
        uint256[2] memory pubkeyG1,
        uint256[4] memory pubkeyG2
    ) internal view returns (bool) {
        uint256[12] memory input = [
            pubkeyG1[0],
            pubkeyG1[1],
            G2x1,
            G2x0,
            G2y1,
            G2y0,
            nG1x,
            nG1y,
            pubkeyG2[1],
            pubkeyG2[0],
            pubkeyG2[3],
            pubkeyG2[2]
        ];
        uint256[1] memory out;
        bool success;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            success := staticcall(sub(gas(), 2000), 8, input, 384, out, 0x20)
            switch success
            case 0 {
                invalid()
            }
        }
        require(success, "");
        return out[0] != 0;
    }

    /// @notice verifies if a list of pubkeysG2 is equivalent to aggPubKeyG1 that is aggregated off chain
    /// @dev not sufficient to check for rogue-key attacks, use verifyHelpedAggregationPublicKeysRec instead
    /// @param aggPubkeyG1 public key in G1(aggregated off chain)
    /// @param pubkeysG2 list of public keys in G2
    function verifyHelpedAggregationPublicKeysMultiple(
        uint256[2] memory aggPubkeyG1,
        uint256[4][] memory pubkeysG2
    ) internal view returns (bool) {
        uint256 size = pubkeysG2.length;
        require(size > 0, "BLS: number of public key is zero");
        uint256 inputSize = (size + 1) * 6;
        uint256[] memory input = new uint256[](inputSize);
        input[0] = aggPubkeyG1[0];
        input[1] = aggPubkeyG1[1];
        input[2] = G2x1;
        input[3] = G2x0;
        input[4] = G2y1;
        input[5] = G2y0;
        for (uint256 i = 0; i < size; i++) {
            input[i * 6 + 6] = nG1x;
            input[i * 6 + 7] = nG1y;
            input[i * 6 + 8] = pubkeysG2[i][1];
            input[i * 6 + 9] = pubkeysG2[i][0];
            input[i * 6 + 10] = pubkeysG2[i][3];
            input[i * 6 + 11] = pubkeysG2[i][2];
        }
        uint256[1] memory out;
        bool success;
        // solium-disable-next-line security/no-inline-assembly
        assembly {
            success := staticcall(
                sub(gas(), 2000),
                8,
                add(input, 0x20),
                mul(inputSize, 0x20),
                out,
                0x20
            )
            switch success
            case 0 {
                invalid()
            }
        }
        require(success, "");
        return out[0] != 0;
    }

    /// @notice Add two points in G1
    function addPoints(uint256[2] memory p1, uint256[2] memory p2)
        internal
        view
        returns (uint256[2] memory ret)
    {
        uint256[4] memory input;
        input[0] = p1[0];
        input[1] = p1[1];
        input[2] = p2[0];
        input[3] = p2[1];
        bool success;
        assembly {
            success := staticcall(sub(gas(), 2000), 6, input, 0xc0, ret, 0x60)
        }
        require(success);
    }

    /// @notice multiple a point in G1 by a scaler
    function mulPoint(uint256[2] memory p, uint256 n)
        internal
        view
        returns (uint256[2] memory ret)
    {
        uint256[3] memory input;
        input[0] = p[0];
        input[1] = p[1];
        input[2] = n;
        bool success;
        assembly {
            success := staticcall(sub(gas(), 2000), 7, input, 0x80, ret, 0x60)
        }
        require(success);
    }

    // /// @notice https://gist.github.com/kobigurk/257c1783ddf556e330f31ed57febc1d9
    // /// @dev the recommended way to verify if both public keys have the same secret
    // /// as it checks if the signer provides public keys corresponding to an actual
    // /// secret, preventing the public key attack
    // /// @dev can also work for aggregated publickeys and signtures
    // /// @param pubkeyG1 public key in G1
    // /// @param pubkeyG2 public key in G2
    // /// @param data to be signed by the signer
    // /// @param signature of the data by the signer
    // function verifyHelpedAggregationPublicKeysRec(
    //     uint256[2] memory pubkeyG1,
    //     uint256[4] memory pubkeyG2,
    //     bytes memory data,
    //     uint256[2] memory signature
    // ) internal view returns (bool) {
    //     uint256[2] memory hash = hashToPoint(data);
    //     require(verifySingle(signature, pubkeyG2, hash), "verification failed");

    //     uint256 alpha = uint256(
    //         keccak256(abi.encodePacked(data, signature[0], signature[1], pubkeyG2[0], pubkeyG2[1], pubkeyG2[2], pubkeyG2[3], pubkeyG1[0], pubkeyG1[1]))
    //     );
    //     uint256[2] memory scaled_p1 = mulPoint(pubkeyG1, alpha);
    //     uint256[2] memory scaled_g1_generator = mulPoint([G1x, G1y], alpha);

    //     uint256[2] memory g1_part_sig = addPoints(signature, scaled_p1);
    //     uint256[2] memory g1_part_hash = addPoints(hash, scaled_g1_generator);
    //     return verifySingle(g1_part_sig, pubkeyG2, g1_part_hash);
    // }
}
