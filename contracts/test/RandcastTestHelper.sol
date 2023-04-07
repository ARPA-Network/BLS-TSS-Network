// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "forge-std/Test.sol";
import "../src/Adapter.sol";
import {Staking, ArpaTokenInterface} from "Staking-v0.1/Staking.sol";
import "./ControllerForTest.sol";
import "./mock/MockArpaEthOracle.sol";
import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
import "openzeppelin-contracts/contracts/utils/Strings.sol";

abstract contract RandcastTestHelper is Test {
    ControllerForTest controller;
    Adapter adapter;
    MockArpaEthOracle oracle;
    IERC20 arpa;
    Staking staking;

    address public admin = address(0xABCD);
    address public stakingDeployer = address(0xBCDE);
    address public user = address(0x11);

    // Nodes: To be Registered
    address public node1 = address(0x1);
    address public node2 = address(0x2);
    address public node3 = address(0x3);
    address public node4 = address(0x4);
    address public node5 = address(0x5);
    address public node6 = address(0x6);
    address public node7 = address(0x7);
    address public node8 = address(0x8);
    address public node9 = address(0x9);
    address public node10 = address(0xA);
    address public node11 = address(0xB);
    address public node12 = address(0xC);

    // Staking params
    /// @notice The ARPA Token
    ArpaTokenInterface ARPAAddress;
    /// @notice The initial maximum total stake amount across all stakers
    uint256 initialMaxPoolSize = 50_000_00 * 1e18;
    /// @notice The initial maximum stake amount for a single community staker
    uint256 initialMaxCommunityStakeAmount = 2_500_00 * 1e18;
    /// @notice The minimum stake amount that a community staker can stake
    uint256 minCommunityStakeAmount = 1e12;
    /// @notice The minimum stake amount that an operator can stake
    uint256 operatorStakeAmount = 500_00 * 1e18;
    /// @notice The minimum number of node operators required to initialize the
    /// staking pool.
    uint256 minInitialOperatorCount = 1;
    /// @notice The minimum reward duration after pool config updates and pool
    /// reward extensions
    uint256 minRewardDuration = 1 days;
    /// @notice Used to calculate delegated stake amount
    /// = amount / delegation rate denominator = 100% / 100 = 1%
    uint256 delegationRateDenominator = 20;
    /// @notice The freeze duration for stakers after unstaking
    uint256 unstakeFreezingDuration = 14 days;

    uint256 rewardAmount = 1_500_00 * 1e18;

    uint256 t = 3;
    uint256 n = 5;

    // Node Partial Public Keys
    bytes partialPublicKey1 =
        hex"0f43a1f52305b2e8eb2c0fbed1be75f29526e9b6d81cb04971ef07289d263c7a02a0ae0b19ee2bf2dfa045092130770253191c95a71f2a36ebc1421a9e183b940cec8d1a59cc49a63de075ba92a5ad2570b08fc870970150ac24283df7d8b7390fe8912631f2228d82c01df7abe12aa7d619d213ca417cbc20ca728c704bca49";
    bytes partialPublicKey2 =
        hex"0dce7fbd5e5fd4c7c5d07abf9b0488026d0a6e0d97ab853867fd03d460af6e3a2a55b4c300d4ada98d4c1d652049ecb1fc884820d74d14675616c291d9e152ea2855dfd92ff71b043cdd80ae30019ee45e60ae2da408be402ebed54af8c91f371a54f07a261716395129f9cfab6f611658388d6f9c1306a78d15f11fae63c8fb";
    bytes partialPublicKey3 =
        hex"2d2e7bc494344582d8fbdcd6a1dd583a25d7ff34356def9e876e1545d7fe943a10244eedd3c4c57a8e7225fd42d0d07aa032b5bcc2cc6acc022ed0c8aea96cae2c922c760836e6f9e35d4c00c875580a2156dd1f898fe35ec1acf042508220962c97917a438f84040476ffac48ea213bf3a8c51196d2c8dd84b9ddd3477cdbf8";
    bytes partialPublicKey4 =
        hex"2365ccbbbdf4bd79a6b480f0bb07ab10b0bdc3f51d954dd2a27a5d87d37b0587144d1381eeb24f28248ddf5b10de95f91f732438d0a913be6c4d0bb3948b353b15001f911d281d85073ddd23587f280587c5c13bf766fda73742937546e387e8114e9084f5011a0ff6753dba34813c5805b1dc126145d3044daea8100f701be1";
    bytes partialPublicKey5 =
        hex"19607cd16db7e55acfcba3a2d6f9e8649ee885e83a284136d98582e42a784e0912a633c2fc7a15b2ccedd66b4ec55063582b6b5b260796a1dafb4189db4f9a10218ed52346e4867c395852a3703a11cb264892743f9d5bb3c44581d9e543f3c71d18f0c9ae30fd1eb9d2edb564e5ed104f6091604acd2ebe03ad50ba22d3ebee";
    bytes badKey =
        hex"111111550230862eccaa516975047156d5c7cdc299f5fce0aaf04e9246c1ab2122f8c83061984377026e4769de7cc228004221275241ee6a33622043a3c730fc183f7bff0be8b3e21d9d56bc5ed2566ce3193c9df3396bd8cdc457e7c57ecbc010092c9cf423391bff81f73b1b33ac475dbf2b941b23acc7aa26324a57e5951b";

    // Group Public Key
    bytes publicKey =
        hex"137bde2a3eca9e26d5023c8a31c7db75db47b4d1776efc144bc9cfa36403125510292172c806e0d9dd29958c8b359ea9c693179c505558cc95ca8ce6a690eb800652ce1fadb1895c06e5f28e871d8e3797f749941108195d2106a782464a09ed23ece01e5c6512317cd413fbecc36032ab7ba45f62704e9808ec2b6a2dd03d8c";

    // Partial Signatures
    // msg: "de7c516e867a427226866fa41566ad0eb0ff69f54e4408babd2f59c54238fa860000000000000000000000000000000000000000000000000000000000000001"
    uint256 partial1_1 = 0xa7b0bf678df3ba00d8d25f952d80634635a2ae2bd1e8b480fc2cf7c3d264f5d9;
    uint256 partial1_2 = 0x0a1db9a6dd4efe430498616ad8630c3efb6f89b71a978128dba1c1d33ac35ee8;
    uint256 partial1_3 = 0x0aaf81e1d868eed5eea9b66e1b47cbe66dc5a36f273625bb69c68f5797a7339d;
    uint256 partial1_4 = 0x15d1b24d486e923e8efd834bbf0c7e482e22cd9f263db13708b399853da96ae3;
    uint256 partial1_5 = 0x1fe75aa1bf5c46bc51adba3783b4aa15b14d80f3a7ace0360c4429c43e295b1c;
    uint256 signature1 = 0x9859ed63e7d820a42c664139264441525984e9034078b2fc983eef00291588ff;
    uint256[] sig1 = [signature1, partial1_1, partial1_2, partial1_3, partial1_4, partial1_5];

    // msg: "d33ade962639dfbe2c2cb144c8b81b9b41426183f4a8f9d97a3797267085e3e90000000000000000000000000000000000000000000000000000000000000012"
    uint256 partial2_1 = 0x1d5c0300007948ff0294ea1f8e795c52af099396b407e7276e7fcb7da814524d;
    uint256 partial2_2 = 0x2a57b3f486530ae0d1ec72931650fe908991046a2def72f7d11a1dd7ab5f95e4;
    uint256 partial2_3 = 0x10763865020114ddc20aedaf8c22e7467266da0e6190d06f3a4e56a483ec7edc;
    uint256 partial2_4 = 0x053c1c68b4fa2aaa703c5b74a337d1013e0e5c27b709637f11ddc4104b91a31f;
    uint256 partial2_5 = 0xad4869169ad6185c5d4e0b40724a7867f11c91d838dddd3765aae2ba360bde0c;
    uint256 signature2 = 0x28e0e8b54c9fc8e7e97aeec70d5dac07752f7f6d2aebe3a62176cfe9d0da346b;
    uint256[] sig2 = [signature2, partial2_1, partial2_2, partial2_3, partial2_4, partial2_5];

    // msg: "d43a01309e5ce06f563855ea4d63bfd19566144d62073344ca66a52fe459fcd20000000000000000000000000000000000000000000000000000000000000023"
    uint256 partial3_1 = 0x02ecd86ac78fba0044c3af9f2d20f19c1eb0b817c3cf1633a21e9e54bcabf3b9;
    uint256 partial3_2 = 0x21e305c78f09009152edbf7222621e868afdbe5ff0cf36bc8df3d0a7312a2739;
    uint256 partial3_3 = 0xaf6163173a1c9967377dd4a9eefa8842db87682be214c8b5c4be81acf3ca14ec;
    uint256 partial3_4 = 0x970e7620096c5d7ff6c2dfc41bd0e732330531bed9222287075c8cd16eb9bad3;
    uint256 partial3_5 = 0x0e69fd733c1d38f6ad75f8cee338fc3dca33a317c841e1635fd5d096e86aacad;
    uint256 signature3 = 0xab32ad45583c54b04f944fb7c256886e92c9b081bcd016e6d326157e16b0540b;
    uint256[] sig3 = [signature3, partial3_1, partial3_2, partial3_3, partial3_4, partial3_5];

    // msg: "edea6587954fc90bf55a2e710f2edda6d40cbc59050201e7e04b44af906eb1dc0000000000000000000000000000000000000000000000000000000000000034"
    uint256 partial4_1 = 0x1bd9a80aa816302169a2bda79f25295505744b00762573304c627e235c5489fb;
    uint256 partial4_2 = 0x236bb25effd1e4de35ca8c968d06e3661719667bde72f43340e608e99a757709;
    uint256 partial4_3 = 0x175db9284c6f33bc65e0b597ff015672a46a793c0b4afed4da06e92ff85b4353;
    uint256 partial4_4 = 0xa9edc57ecdd0c3a32b7bdb78d683c49233ccda86f9f63faead489cf1d0b1ad41;
    uint256 partial4_5 = 0xa325d272338d872a80e3bc5a262eb162600658f6bd7d794a5bbd87ce663041b1;
    uint256 signature4 = 0x9f8e9654dd1a3b86c8bcdf6ad91704992e24112f450754ce2b76efbbde7c4ce7;
    uint256[] sig4 = [signature4, partial4_1, partial4_2, partial4_3, partial4_4, partial4_5];

    // msg: "3884a5de852ba49a376e504abed8501ce14c9fd7d0a8a4aaca2a81d9d87b35da0000000000000000000000000000000000000000000000000000000000000045"
    uint256 partial5_1 = 0x8b2b823510064bbc826d442a36e10e19903f12d2b5d8889497ade0feb995fc7c;
    uint256 partial5_2 = 0x994fbd013506b0323a3cf527521471b0d65b2e908b113ed82501133431d3c78f;
    uint256 partial5_3 = 0x80cce4a7a23536d9532a9e70d4abf7938ac3b4c11b385be7a45d9fb01637f4b1;
    uint256 partial5_4 = 0x2a0aaa9f7875564955e438f8ab29b105b095d6b97ab1e0acb4de68a8d71e4aff;
    uint256 partial5_5 = 0x1cd4512a230edc07e81ee0a330bf0c25095e2df768e8fa3d28914a28b840ba09;
    uint256 signature5 = 0x99485ca68e9b66724a768c47e04531a2f63cb3e5754f2a5b2a17784d48abf73c;
    uint256[] sig5 = [signature5, partial5_1, partial5_2, partial5_3, partial5_4, partial5_5];

    // msg: "5e834c9e0ee3c1df597ce48258054d49ed258ecb6d31fbce6a78be9daf9afaa10000000000000000000000000000000000000000000000000000000000000056"
    uint256 partial6_1 = 0x83abcdb3e9e96f1f9227d7e37fdb92e7cf05576e35f404d45f48c5b554e65e12;
    uint256 partial6_2 = 0x81a5a264abc76a54bf6f0f7f6bf2e4c775e73a4fa0c513857b6354d6b6505d26;
    uint256 partial6_3 = 0x979584bd5813c849b0dbdf3e97c64ddbc6f0675a95c6eedb06b89c607c7644ec;
    uint256 partial6_4 = 0x90e4535cc9aebc9c3ae363f2d12a49d2e4558cad5e0023e7912e60cffcfe2ffc;
    uint256 partial6_5 = 0x22f69cf529e9cd57288c42b020ae1581e6977e5587e113d6358e766f462affdc;
    uint256 signature6 = 0x156ae717ee09b79a3956a536b7793a5459c9e42a540594f27b3b0885e2da8d71;
    uint256[] sig6 = [signature6, partial6_1, partial6_2, partial6_3, partial6_4, partial6_5];

    // msg: "e0ab0f3bc77b9c2b203d07aaa5af51280d8bba6c5bb1cd54763959c1b87708540000000000000000000000000000000000000000000000000000000000000067"
    uint256 partial7_1 = 0x08bf742012c31bc9aaf96258e26e88292211dca4bf6556062092459a5a1c31a5;
    uint256 partial7_2 = 0x0ae64cc1c9a2bd67609656ac5e14a2e522356ef09d2f3ccd4f3104ad20f333e7;
    uint256 partial7_3 = 0x928cf016444f840ff1b91de1050d7e1bbdb76afd5b46c4211102a8601abf038d;
    uint256 partial7_4 = 0x20341836bb2dd010bc8a8df9aa47b65848b3650266d9a96937849fcbc0da39fa;
    uint256 partial7_5 = 0xa9060557a4f8f3a7aa033c916068d379823d98f2c4ce230a1c2e75917113653f;
    uint256 signature7 = 0x062562ba880a4822833e1b280d2e985cb15dd3077e66abda262de66776ae9480;
    uint256[] sig7 = [signature7, partial7_1, partial7_2, partial7_3, partial7_4, partial7_5];

    // msg: "428874369318654d220b552cd69526384de1a9854b9a194f3ee725424d32e4bd0000000000000000000000000000000000000000000000000000000000000078"
    uint256 partial8_1 = 0x15c9fea090a22f87231d9d44c6d7046556a42eabe891d43cd786a2a566c59df2;
    uint256 partial8_2 = 0x2601fdffd8193fdc034b50dfb0dff66190715292dec0dbcbb3f0b787beab4e48;
    uint256 partial8_3 = 0x8de124ee2c0ef60ab0fe344831610feb110fb1b707bbf6fe7bdb49ff32b43bf2;
    uint256 partial8_4 = 0x8ef5ceb26b858046ca0d6a4f42da9331d0b01d6883111e2005d5d49fbdd1be45;
    uint256 partial8_5 = 0x0019670cff4259fe152b53f228864c68d7564798cb09236677e2a0a73f6df58a;
    uint256 signature8 = 0xabb882aa8a8ff347686556ae6998f14e29b60b3f16ef5bf09a67d46dac06248c;
    uint256[] sig8 = [signature8, partial8_1, partial8_2, partial8_3, partial8_4, partial8_5];

    // msg: "640670d5370a0b1142d202dd8d733cdc5317af68692ed48653ec348d2432c6f60000000000000000000000000000000000000000000000000000000000000089"
    uint256 partial9_1 = 0x241800efb6bafaf3e0701911137156975dfffaf04c0f7ff9620660192e52062e;
    uint256 partial9_2 = 0x9da59c71df582722c4c6713745c027656c8ddb11294c4e35f47f48d9751dc84b;
    uint256 partial9_3 = 0x99c85efca91b34cee4ba9edf93901548a3d377c207b36003c1c647d3e3e0ee46;
    uint256 partial9_4 = 0xa7ffd70ac96bb618bd1d89df319ef8caf2b1a69ebfb208b3549de272b9854d42;
    uint256 partial9_5 = 0x10417e79175775f12ae8f8fa8571aff64a566c2f0d43e8c9d1550b50ebedaaa3;
    uint256 signature9 = 0x0c55d56d241c9d323fc541b5ef1674a0d7d012bc604464fcba7e72db09aa0dd7;
    uint256[] sig9 = [signature9, partial9_1, partial9_2, partial9_3, partial9_4, partial9_5];

    // msg: "1e049c83faf1abc374ad113f0e50995b4ed8a5be475156a346e773bb0d5cea7d000000000000000000000000000000000000000000000000000000000000009a"
    uint256 partial10_1 = 0x80b4a22c52d462886031de42d8113b9bfcab1e691373c0387289c4126b7660c5;
    uint256 partial10_2 = 0x23ab53561447aa449768f36d42ddf96484677af992afc2d595798eeaecb8fa23;
    uint256 partial10_3 = 0xaf1b6f4e298b7dbdd428f2bedf29af44394a1a051580982cc1be2ae1ca03a4e6;
    uint256 partial10_4 = 0x154bd43fe3e514638e455a6c09c7f45f8f655112555c56b0f9939f5df3e87616;
    uint256 partial10_5 = 0x8ccb08b65a7a5830c1bf92de4dfdfa3b6a5f172cdf9582fdc26ffe5e6ae89356;
    uint256 signature10 = 0x27fe7e0b003b339b407874fa0cabfe4b9df8ce862ec4eabe6e317f99bb55a0a8;
    uint256[] sig10 = [signature10, partial10_1, partial10_2, partial10_3, partial10_4, partial10_5];

    // msg: "912879b65da7afad27afb740fed587019e3aac62a0731fcdb33a38a5e04a9dbc0000000000000000000000000000000000000000000000000000000000000001"
    uint256 partial11_1 = 0x0e618d96d30ef2579b7302a3c1eaf512221e0584437bbbc31a084cc57212a7cf;
    uint256 partial11_2 = 0x22760b163e645f88bf8afc1a8af8950f14346199bdbc177b0745db2c0ff8087f;
    uint256 partial11_3 = 0xa5b5c0bc5ea3e352980dae9ec03001f7a7a365edf90c659bf56beecb721e48df;
    uint256 partial11_4 = 0x200926692dc71a030f2532ca423ed23c8e971cde5722a83dcc3c0c4d8f39497d;
    uint256 partial11_5 = 0xa56ea516bd4b4a7f6012132f40bd43a3db0179a833599638a02e0be1fe965934;
    uint256 signature11 = 0xa9823520357ffdb0f7aa76d6fc7ae559289336e3c1dfcf84f830308af03f503c;
    uint256[] sig11 = [signature11, partial11_1, partial11_2, partial11_3, partial11_4, partial11_5];

    // msg: "34fa9bc41a34b4a1fb4dc01764f8f0cc61f6fc90284a603c49e393c2202135520000000000000000000000000000000000000000000000000000000000000001"
    uint256 partial12_1 = 0x9c08dfa2c203dc5eb9cc044387dc204378e5f9cc9521b9227d27a345e79337ac;
    uint256 partial12_2 = 0x246460deb8e7d38b356ece81a61289327cf7ef6912cd8ad58d6ae72a1c761409;
    uint256 partial12_3 = 0x09d79a07b1c9a29ab852410658536c153ba18133d50dabdae20bb6d791667868;
    uint256 partial12_4 = 0x2db11e0bf711c2f21df24c71e4c4ec21bc6b1d6475ded082a50b655494c6c930;
    uint256 partial12_5 = 0x0b239f7e2d787287aa012b45d675251766463f8a8490a0cffe4f3c671e864041;
    uint256 signature12 = 0xa41c00557357ddd77a247a5995bfc3e6b2923d98575d46a6325208935e60cc25;
    uint256[] sig12 = [signature12, partial12_1, partial12_2, partial12_3, partial12_4, partial12_5];

    // msg: "a4371e17c84d644e0823a95223011a30b995e36afd180b719769ad6ce88ceeae0000000000000000000000000000000000000000000000000000000000000001"
    uint256 partial13_1 = 0x8ef9a2dd719dc0e0e96e5baa81cc4177141fb21af6a29210a37ecc166f819329;
    uint256 partial13_2 = 0x10534c1673f82d97df90adcfb04deceb81be832191f7c38b0b3ce2d25276811a;
    uint256 partial13_3 = 0xae718a17309070d21c58623ec883b640eb49aeed1a7abcfea0281a10f6e7fd64;
    uint256 partial13_4 = 0x0a43269c573eab2804273f65b474e4ed328f5b1aa5a03c41be9f48cdaa05653a;
    uint256 partial13_5 = 0x29e64a4aefdee243ceda6ad462b33d7a37936bcd5ec513ecd6fe303178d76577;
    uint256 signature13 = 0x9d700195d65c76b2567a2b1763fbb19dc61256a3ce32e778e670631c21a23608;
    uint256[] sig13 = [signature13, partial13_1, partial13_2, partial13_3, partial13_4, partial13_5];

    uint256[][] sig = [sig1, sig2, sig3, sig4, sig5, sig6, sig7, sig8, sig9, sig10, sig11, sig12, sig13];

    // Node DKG Communication Public Keys

    bytes DKGPubkey1 =
        hex"03972476103d7f22478847415690b3820b6b9b967373ff47401e567a960867850471e7f19de23c72da9b7ee4347a79446e16fbd1e9f8423633f19cefe849af852287527509cd04a386486abbfca0259ccb6475d38258653adda62b2d63f85323217e6bea0d34fd20001f1d94d2cc7fd1bfb0b068cc6ec580264c641aab262aec";
    bytes DKGPubkey2 =
        hex"060aaddcf708eec0cf0cf9beca74be474aec8a301bd1d450e741072a930834a500cc52b5ff51c92c6bdb532194868a7f8603a4bced51b9f998476e730a9278d4091cd7e0efad5211e6f99505f9024c78a247b561263e211ba20446da4c08a6042a77c83d6d9928ddf2cf655c85d3bd8f8d58c0465a34b85410de5b6a9017e3b5";
    bytes DKGPubkey3 =
        hex"13bcb25821e4b92af197cbedc54a602cc3da980823562760dff0ce5b26055708187e233e156278b658842b3300c61ba73c31e3c501cd19173a2239057165f8882bf771a3b071cc7a6950009bbb6b37c5fc81207e72b0f1a71d6b07ffcbc0e6cc1e29d4b4543ceaa24534cfda714bb12bd8f788848ed377064c6cbf3ca7a16e3f";
    bytes DKGPubkey4 =
        hex"22e12f5c34692f7559377d7ece5659e31dc6f1b591b3edf186f0e2319358aa8a068646547efb0d76a8c7482444a1a4c61bedc56fcec3acf47fac799934921823163f489095579c7d26beb915f92bdbf808ca3f94fa9c2a0632701f94bc79e9eb0cf5d2a979b3e94fed83d02398573e5af5abb693b5eace143b6e044df1435d0e";
    bytes DKGPubkey5 =
        hex"222451063a1a09012717d1d7aa27eb5900f0cab24f3b86ecf4005edb08d8b7c601b14f5f4519bc3aa8662868996986104b0d17d720e69a0f5bc7917811a5009c17b1834fe80db270a22805e126801e328828c98ebaa078a58daec8cba75dc6b31980e4303c2d67976fa6423e4ec6f2acad16cf51c07c7098f5d0dfacc3f8a016";
    bytes DKGPubkey6 =
        hex"1b5969815bda9a3a6a07b9bd09541853c7c06b4046d925790775ae85807a47e507e3e19a949b1cc73f64649358a95a432e618c0299d8cdbf704b5c72c6e67c3302d610e17acef4f1b1fcf11409095b23db4e012e9d9bed5401296e1872820fa12f9a411f31a7e8e4b5ddb495f3a211b0c44760a7e389427db6467dea5378c83a";
    bytes DKGPubkey7 =
        hex"2a2b0eeb21e345723d39f0576a14cba0c2dfe06c3d75cfc17437bfbc5858dd3f1454cfc0877f29b20a5781f9a98857e50e8700ba6428685c36e17f321b8963d42002513d18d510a22cfee59385418d580589b83540ef9b0e8201888c666b517c20718d1ad3a85cf313b8eab6620c0e560764b215f7b967a8a20d6ccdcf2bb2d2";
    bytes DKGPubkey8 =
        hex"28ae5c88d9b18ed4af86fcff85ff9cc2b60e4f1ccdf8dbabd9f4e2a03ad8847c0858628f0e88b66a1a091b20c4c110a806cff4a7c3cf125bc91312ed1e1b367227d47f9473bc1465960c163a7a320c26934eb00e3024a1caa290976547f131ab194290e2950ef6ec1ce07793fe759ffa3750ef17f77ec7850c79f9dad130a96a";
    bytes DKGPubkey9 =
        hex"151727e94c3cafd40ca38420d0517c3b73b778f6d20a7a2cce8573b676bf37b62973e1eaa1a77224da262fa883f86423542623088015e5630a3e4d49e0f6eff70a5323707557d41b0840808f0a7dbb546cba9b95f2449cf5ca075585ef738cf42f4dd51e48f08a6375a6e663cf7b3d28d613ffc1d6c52d62224171f05060add4";
    bytes DKGPubkey10 =
        hex"226105c8a5e04c372089a4e5df221a3f50c4608ed5aa2e6ad624f8cb6558a5cf241888a90c01ee4afde82a7697d335abb34cdbf9048beb1c2587ed295b9ca86a2027d7317ed541a90ef20af458f5941760dc2644d24c7dcc2551e229be8342092fa3ce1b1ebaf16796bec6377ff8c161ad5d96d42c8115368d2f691278c4e0eb";
    bytes DKGPubkey11 =
        hex"0a136df5e9169f08b5207392a27b2410115e26dbf83a8d4033fb1cef6b5e1f1c16dce818ffd3b85e9d586f945369770c683a78bc138fea50b1c01a3d9c5be5181cc30078accd999e7a479ae3cb73154f7af41f923c1e3efa093711ee7389c21d1878700264e4d7b1914204def9e35f76cdfec0c8d0c43c1abce317b5c51c306f";
    bytes DKGPubkey12 =
        hex"02769fcd16915bb205af814a7520996c1200ce9dc01d1a15aa87fc34e5f30bd80371873ee754e8a9ef1242de811756088e55daba7263ade0daa25e00d2381ac22eabb44e702f14e18b0e899ea576715b6e0335f10696f3c1ef37bf35cda721090900d027f0e293a5da5a3fa147fa725bb2b1fa639112808bdfdcc516cffa580d";

    function fulfillRequest(bytes32 requestId, uint256 sigIndex) internal {
        IAdapter.Callback memory callback = adapter.getPendingRequest(requestId);
        // mock confirmation times and SIGNATURE_TASK_EXCLUSIVE_WINDOW = 10;
        vm.roll(block.number + callback.requestConfirmations + 10);

        // mock fulfillRandomness directly
        IAdapter.PartialSignature[] memory partialSignatures = new IAdapter.PartialSignature[](3);
        partialSignatures[0] = IAdapter.PartialSignature(0, sig[sigIndex][1]);
        partialSignatures[1] = IAdapter.PartialSignature(1, sig[sigIndex][2]);
        partialSignatures[2] = IAdapter.PartialSignature(2, sig[sigIndex][3]);
        adapter.fulfillRandomness(
            0, // fixed group 0
            requestId,
            sig[sigIndex][0],
            partialSignatures
        );
    }

    function prepareSubscription(address consumer, uint96 balance) internal returns (uint64) {
        uint64 subId = adapter.createSubscription();
        arpa.approve(address(adapter), balance);
        adapter.fundSubscription(subId, balance);
        adapter.addConsumer(subId, consumer);
        return subId;
    }

    function getBalance(uint64 subId) internal view returns (uint96, uint96) {
        (uint96 balance, uint96 inflightCost,,,) = adapter.getSubscription(subId);
        return (balance, inflightCost);
    }

    function prepareStakingContract(address sender, address arpaAddress, address[] memory operators) internal {
        vm.stopPrank();

        Staking.PoolConstructorParams memory params = Staking.PoolConstructorParams(
            ArpaTokenInterface(arpaAddress),
            initialMaxPoolSize,
            initialMaxCommunityStakeAmount,
            minCommunityStakeAmount,
            operatorStakeAmount,
            minInitialOperatorCount,
            minRewardDuration,
            delegationRateDenominator,
            unstakeFreezingDuration
        );
        vm.prank(sender);
        staking = new Staking(params);

        // add operators
        vm.prank(sender);
        staking.addOperators(operators);

        // start the staking pool
        deal(address(arpa), sender, rewardAmount);
        vm.prank(sender);
        arpa.approve(address(staking), rewardAmount);
        vm.prank(sender);
        staking.start(rewardAmount, 30 days);

        // let a user stake to accumulate some rewards
        stake(sender);

        for (uint256 i = 0; i < operators.length; i++) {
            stake(operators[i]);
        }

        // warp to 10 days to earn some delegation rewards for nodes
        vm.warp(10 days);
    }

    function stake(address sender) internal {
        deal(address(arpa), sender, operatorStakeAmount);
        vm.prank(sender);
        arpa.approve(address(staking), operatorStakeAmount);
        vm.prank(sender);
        staking.stake(operatorStakeAmount);
    }

    function prepareAnAvailableGroup() public {
        vm.stopPrank();

        // deal nodes
        vm.deal(node1, 1 * 10 ** 18);
        vm.deal(node2, 1 * 10 ** 18);
        vm.deal(node3, 1 * 10 ** 18);
        vm.deal(node4, 1 * 10 ** 18);
        vm.deal(node5, 1 * 10 ** 18);

        // Register Node 1
        vm.prank(node1);
        controller.nodeRegister(DKGPubkey1);

        // Register Node 2
        vm.prank(node2);
        controller.nodeRegister(DKGPubkey2);

        // Register Node 3
        vm.prank(node3);
        controller.nodeRegister(DKGPubkey3);

        // Register Node 4
        vm.prank(node4);
        controller.nodeRegister(DKGPubkey4);

        // Register Node 5
        vm.prank(node5);
        controller.nodeRegister(DKGPubkey5);

        uint256 groupIndex = 0;
        uint256 groupEpoch = 3;

        address[] memory disqualifiedNodes = new address[](0);
        IController.CommitDkgParams memory params;

        // Succesful Commit: Node 1
        params = IController.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey1, disqualifiedNodes);
        vm.prank(node1);
        controller.commitDkg(params);

        // Succesful Commit: Node 2
        params = IController.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey2, disqualifiedNodes);
        vm.prank(node2);
        controller.commitDkg(params);

        // Succesful Commit: Node 3
        params = IController.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey3, disqualifiedNodes);
        vm.prank(node3);
        controller.commitDkg(params);

        // Succesful Commit: Node 4
        params = IController.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey4, disqualifiedNodes);
        vm.prank(node4);
        controller.commitDkg(params);

        // Succesful Commit: Node 5
        params = IController.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey5, disqualifiedNodes);
        vm.prank(node5);
        controller.commitDkg(params);
    }

    function printGroupInfo(uint256 groupIndex) public {
        IController.Group memory g = controller.getGroup(groupIndex);

        uint256 groupCount = controller.getGroupCount();
        emit log("--------------------");
        emit log_named_uint("printing group info for: groupIndex", groupIndex);
        emit log("--------------------");
        emit log_named_uint("Total groupCount", groupCount);
        emit log_named_uint("g.index", g.index);
        emit log_named_uint("g.epoch", g.epoch);
        emit log_named_uint("g.size", g.size);
        emit log_named_uint("g.threshold", g.threshold);
        emit log_named_uint("g.members.length", g.members.length);
        emit log_named_uint("g.isStrictlyMajorityConsensusReached", g.isStrictlyMajorityConsensusReached ? 1 : 0);
        for (uint256 i = 0; i < g.members.length; i++) {
            emit log_named_address(
                string.concat("g.members[", Strings.toString(i), "].nodeIdAddress"), g.members[i].nodeIdAddress
            );
            for (uint256 j = 0; j < g.members[i].partialPublicKey.length; j++) {
                emit log_named_uint(
                    string.concat("g.members[", Strings.toString(i), "].partialPublicKey[", Strings.toString(j), "]"),
                    g.members[i].partialPublicKey[j]
                );
            }
        }
        // print committers
        emit log_named_uint("g.committers.length", g.committers.length);
        for (uint256 i = 0; i < g.committers.length; i++) {
            emit log_named_address(string.concat("g.committers[", Strings.toString(i), "]"), g.committers[i]);
        }
        // print commit cache info
        emit log_named_uint("g.commitCacheList.length", g.commitCacheList.length);
        for (uint256 i = 0; i < g.commitCacheList.length; i++) {
            // print commit result public key
            for (uint256 j = 0; j < g.commitCacheList[i].commitResult.publicKey.length; j++) {
                emit log_named_uint(
                    string.concat(
                        "g.commitCacheList[", Strings.toString(i), "].commitResult.publicKey[", Strings.toString(j), "]"
                    ),
                    g.commitCacheList[i].commitResult.publicKey[j]
                );
            }
            // print commit result disqualified nodes
            uint256 disqualifiedNodesLength = g.commitCacheList[i].commitResult.disqualifiedNodes.length;
            for (uint256 j = 0; j < disqualifiedNodesLength; j++) {
                emit log_named_address(
                    string.concat(
                        "g.commitCacheList[",
                        Strings.toString(i),
                        "].commitResult.disqualifiedNodes[",
                        Strings.toString(j),
                        "].nodeIdAddress"
                    ),
                    g.commitCacheList[i].commitResult.disqualifiedNodes[j]
                );
            }

            for (uint256 j = 0; j < g.commitCacheList[i].nodeIdAddress.length; j++) {
                emit log_named_address(
                    string.concat(
                        "g.commitCacheList[",
                        Strings.toString(i),
                        "].nodeIdAddress[",
                        Strings.toString(j),
                        "].nodeIdAddress"
                    ),
                    g.commitCacheList[i].nodeIdAddress[j]
                );
            }
        }
        // print coordinator info
        address coordinatorAddress = controller.getCoordinator(groupIndex);
        emit log_named_address("\nCoordinator", coordinatorAddress);
    }

    function printNodeInfo(address nodeAddress) public {
        // print node address
        emit log("\n");
        emit log("--------------------");
        emit log_named_address("printing info for node", nodeAddress);
        emit log("--------------------");

        Controller.Node memory node = controller.getNode(nodeAddress);

        emit log_named_address("n.idAddress", node.idAddress);
        emit log_named_bytes("n.dkgPublicKey", node.dkgPublicKey);
        emit log_named_string("n.state", toText(node.state));
        emit log_named_uint("n.pendingUntilBlock", node.pendingUntilBlock);
    }

    function printMemberInfo(uint256 groupIndex, uint256 memberIndex) public {
        emit log(
            string.concat(
                "\nGroupIndex: ", Strings.toString(groupIndex), " MemberIndex: ", Strings.toString(memberIndex), ":"
            )
        );

        Controller.Member memory m = controller.getMember(groupIndex, memberIndex);

        // emit log_named_uint("m.index", m.index);
        emit log_named_address("m.nodeIdAddress", m.nodeIdAddress);
        for (uint256 i = 0; i < m.partialPublicKey.length; i++) {
            emit log_named_uint(string.concat("m.partialPublicKey[", Strings.toString(i), "]"), m.partialPublicKey[i]);
        }
    }

    function toUInt256(bool x) internal pure returns (uint256 r) {
        assembly {
            r := x
        }
    }

    function toBool(uint256 x) internal pure returns (string memory r) {
        // x == 0 ? r = "False" : "True";
        if (x == 1) {
            r = "True";
        } else if (x == 0) {
            r = "False";
        } else {}
    }

    function toText(bool x) internal pure returns (string memory r) {
        uint256 inUint = toUInt256(x);
        string memory inString = toBool(inUint);
        r = inString;
    }

    function checkIsStrictlyMajorityConsensusReached(uint256 groupIndex) public view returns (bool) {
        Controller.Group memory g = controller.getGroup(groupIndex);
        return g.isStrictlyMajorityConsensusReached;
    }

    function nodeInGroup(address nodeIdAddress, uint256 groupIndex) public view returns (bool) {
        bool nodeFound = false;
        for (uint256 i = 0; i < controller.getGroup(groupIndex).members.length; i++) {
            if (nodeIdAddress == controller.getGroup(0).members[i].nodeIdAddress) {
                nodeFound = true;
            }
        }
        return nodeFound;
    }
}
