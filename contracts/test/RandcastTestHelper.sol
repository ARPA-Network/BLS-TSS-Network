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
    address public node10 = address(0x10);
    address public node11 = address(0x11);
    address public node12 = address(0x12);
    address public node13 = address(0x13);
    address public node14 = address(0x14);
    address public node15 = address(0x15);
    address public node16 = address(0x16);
    address public node17 = address(0x17);
    address public node18 = address(0x18);
    address public node19 = address(0x19);
    address public node20 = address(0x20);
    address public node21 = address(0x21);
    address public node22 = address(0x22);
    address public node23 = address(0x23);
    address public node24 = address(0x24);
    address public node25 = address(0x25);
    address public node26 = address(0x26);
    address public node27 = address(0x27);
    address public node28 = address(0x28);
    address public node29 = address(0x29);
    address public node30 = address(0x30);
    address public node31 = address(0x31);

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
    bytes badKey =
        hex"111111550230862eccaa516975047156d5c7cdc299f5fce0aaf04e9246c1ab2122f8c83061984377026e4769de7cc228004221275241ee6a33622043a3c730fc183f7bff0be8b3e21d9d56bc5ed2566ce3193c9df3396bd8cdc457e7c57ecbc010092c9cf423391bff81f73b1b33ac475dbf2b941b23acc7aa26324a57e5951b";

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
    bytes partialPublicKey6 =
        hex"02d0d020c4f1e28649af7e44e80673bde0c9735e80d476a9112c335c576f287f0babbc8dc54a7415ea528b44014bcdbcf80958cf96f23ff31b33be662aa7c64a0b93d75b674a56deda1c85aed61963d42fc52300f6564638ef0db361d19d9f47090c76742f6e912a38d252d970655cc10d368c96819c644c07a1c3c3e270ec5b";
    bytes partialPublicKey7 =
        hex"2d029c56156c8d50a80308be72e4c575169ffadaa0ac3fdbc3a555108e30499a24e544b89745307f62b944872c0a57eb01014586b6a262879896bffa34720815001e3eb956f1ff5403645a93fbc8c8cd6f491f0f8173fcb2e63a4fd69cda090c108f1b6b7ebf9063ca951c5fad7858db146d62a4b744fd194085eae03272b8cb";
    bytes partialPublicKey8 =
        hex"2cf6850ef8b8f238d4d26dd457cbcfc5c0b5880b33598b6301b02b27806f931f180c697dfe128ac546cd2df5ce2c3d69f599d6f00a4afca3eeb8eddfe4fbe80425b381e7272244483cbb85a4f6e95d72c2c402cd28d089b772322b4d7f10f5ce011c55f4f40527a790f59c573e932f734372f4a621443ff15993e518c2244869";
    bytes partialPublicKey9 =
        hex"01a64349a99285f39d94358c60fa8a7981ba01905f5165e84ce5f71bb11fa90905aa35e57d8c975129ec30a8ba9bca5e301d4af5a69efd91fdbd8a6da2d8e7e7011df0d088ad0c45304a84d8ad6227e2c12cbe84091d9ed1a43f60334b3e99a61c3d6fed8facfec7f8d432bff82f459648aa18ef15ac39b65211d33afee9c96e";
    bytes partialPublicKey10 =
        hex"15078251b04e3224fc23e46bc6edb1153ba3dcf3e306d86b7af18bc2dd9d175a245a89a244a1d2534dc688d623a2c958a16d17c5e5887c48638fd3fdd0e514eb299223b68ba5449e588b26cf7ccac4c9960b8be8bb75e7cc4d7b6eba9a185698084beefa7569783f895db22a8cfe1bf0ce327290dbfee7404378fa804e9a504e";
    bytes partialPublicKey11 =
        hex"09fbf5a67aebad6fc4cc3c8172dfcae20a78c0c0dbcc66cbac30548449040d9107be1d2b0274ca7641e15c63956b17f940e71def2597b3fd855f425f02d52bde01bb35ca79a007321d840a112ebb18246daeb3d6a0cf2cf5009c88a7296ae9fe0fb9015b63965664e1bde1d2b438fa3f47a34868037b51984b458df6de2fcde2";
    bytes partialPublicKey12 =
        hex"05ae3faf558b76d653f603816db30298dc392dd7085fb2261dc1ac94aea992402f22ce63d538c0c3b279ef1f24ca742ad9fbadd2d1661558ad42448820d45f38054205d4aa7ccbf13a392bc0156028be22dc4b246ff55c8be89a1a6aae0072800ec9e5e920447828f24bab06673b140553194ddb4816d3569f05b321fe6a16ac";
    bytes partialPublicKey13 =
        hex"1f81d6f66cb9bca463f5c982932051d6efcd3ebacf0d329acb34eb88f99c37ea24e4e5250ada8f78e26115874510868a83e5a8e8bc2f02c4b68bef93d1ff4eff1d599183c52d1854e690393d7fea0c1742b3a6cb65da7962567857dbc9ec59951890e95411457f1eddacddd8e48a2bf20d387e2ef0c28926b2670af9be2651da";
    bytes partialPublicKey14 =
        hex"15deb2c08af87609f33dda5e238d2238171f02c8b83efb0dd219a08cb00fd4e5198c8b67b3b23613ffaadca04a915bc3c7b4005b24c95c9b5bcd359bdc4fe5be0ae74e03ceefe4320bd7ea922dcd10e5544968ce712c8a3528184cf1711d3abf2397fbde04db466d56bc76e00ef98b3998e088d9d5df77db7db2b3b47c6fe6da";
    bytes partialPublicKey15 =
        hex"1b2fe929bf337dcd8b6cff5987fc6ab9d51786be5d4a4f30cf93043fdb4d2511106a8eae1ef8f27e5a075aa9ab77c31d9e1e4754e51ac3e05713784f614088081d8cd701bb98ffe3ea4abf9cd7688e19bff4c7df39bb3374accc16fb5fa4cc820469785848e16513a0a10acd9ddfbbab71e9ac609a7c4b392ae71d105cfe12c1";
    bytes partialPublicKey16 =
        hex"1842af364c4ecb6b03817282013f6dea6ab8030430211c6864cfa15a2be7b3572601183be460fef786efdd547ededb7d66db01c26bdc1175f95329e69e6c4e5a058ca48103cbf2fe2305043840d6f2931b220e880966bae80329ea11d2ef66652dfdaf3e7626670852c08580e066c04dee85b87b5057d4f63ab6307e976b7075";
    bytes partialPublicKey17 =
        hex"0dc9b25ce21e70260ec8c2d00a958cc3497f31f97d89ead7e53db913e545513403afd4d8127650e27a1a4ab407a3885661fae54ad528307d408f2434e64ad8e720ca10878afcd2d7b223827bbc0bf2b470cf370dafb4533a5741ef30d194c71526092da4f087e82b74c6978c7395466d61cafe7cde14d204f65e8f90ab37685e";
    bytes partialPublicKey18 =
        hex"2aae5c35388f7c56641561a94b10a3041c5c0f89a85c7e84efd0ed12a16847601fa9061813e60c15356d31b94f76d5bc49cbca1ad692faa263b7f51821f3c9490c18aa2d239df8c86761447b00f13e20ea228bc77d5d4f3fc5229705c4c0a47e189bd1404768f72f3f9e05c246268a0442fd0f74d0946935f4e629711df0ce29";
    bytes partialPublicKey19 =
        hex"16d7176ca5d3379734a3382b176240561b4d0c67d9fe9eed329ef8e7da3ed64f0486ecb6297351577067ca04275a22459cc78080ce542d7019341194b48a04912bbb04c939ec7974ea7964b6b63c949b6471b0b285d8a3964321482f20110abc1dacb8b2d98869d4087b56c73edaf2433a0e15da5763ae73be3b1325fbc3bb4f";
    bytes partialPublicKey20 =
        hex"1e59a2366be62d1e8a896042c6e730836d4fd86b08ea99b3f9a321476f28fed82903085edebbbee3311327411e2a516752e01628f99fbdb7adabea05087edaaf0c77c193a9bf00ecdd6db17f4bd0a62906778f05288ea41232520a5173177d42268f4edf76aafa4ef866f9ab05edc5d934ad3c6a4cf4791494d69161c63ebb93";
    bytes partialPublicKey21 =
        hex"1ebcb6d0b928d20ce54ce1308aea4d2a494097af99abf6cffe1d70b06d51990315ec31d094ee9669d982573075ad5d6e821141067ebee72e6bdceae012f6edb21d9675c2f7966b93c42812fe56e37022b1ce490dea4b32e2e30b912e7a6b1f30112fc7975d304dfc642008db588ab6e508969df9e2810d2460993138022ad7fe";
    bytes partialPublicKey22 =
        hex"168c11170372c213d211e63981f8f1b1197421e36a985ca280c4ac22271ecbc7122115c328f2d50f8be64c13e3ad9131273b04dc178be81abcf4e341dccf4a412deb96eacc8c1c29212aa33a33b319c97b3953f453145a60c77f40b55eb1e43f1cd502ff5ab9cbc571e0c313e9d8e24cd371b67dbf69c9251b579dfef4006f7b";
    bytes partialPublicKey23 =
        hex"0cf07b941e16031a517c04ec9d609cb40af7b52bce8e303afd9c562dafa6529e1d125fa582577d156a5d75fd4e310c9e57b89aca02975918d94c30a42ae4518e270e84b4e54bff80c90cb3d380318f203c03dd4b68eff7b3f2ab40e7e240a91422453f756bccc6f08ca924c1e7761f16e1eb8a04128f0f5befaa01b3c2861495";
    bytes partialPublicKey24 =
        hex"169700ccf896f8383fca4bbfbdcd904f0660137df673c17a64982778fa5ef8b5146611822a5b2b1d0a12014ac7fa3edf8fb4c03a88d7472259761f645225f1f11f13985b9d9d72c2898fcc861e7ed4b903a6c4264e2f88011746042eef548b302ec9b46274fdd0cf1e90c7265eebd6d8a978ae13c3b437df61bfcc1e833f6675";
    bytes partialPublicKey25 =
        hex"25d0b18ce3db3f6fd89cfe9ded93f0de2b8706ecdc881ca55fb9a7bbbda1bc2b14562863a93e48fb7adab2215adda74c60cb4543764287416e1b3f763698eac517f91167a4ec794fe40ac38686853c6afb903ac7277188b24f864f21092cba5819af94354e87da41949bb6f0147d8732603a0514ce035b38fcc4500f0635002d";
    bytes partialPublicKey26 =
        hex"20e635876bff0c85c6483a8aa226a77ab5ca1400a42eb437815b87736be467112856c74f65cda9d687b15a31f6ee7013c87d583fe0a136c83de26bc58aecbe100bbd4b7fa1ac87dbf0a22e6b7552b47f066aaaa139f24ce7a254c0c95f727bd42d16e47e516185b08f418078b136da8728f1dec618774ae1f3c3de91a76497a6";
    bytes partialPublicKey27 =
        hex"2bf5e7817ffdcece5c95ae51bfe9c3c9c1c88f7f0fc6e71007c0975932ee299c17bc336982a24d3152c0fb4cc2c20d7169f12577d5ef5c0e7f0ab47ad25f3c6205b3345ec11c7e14e4f7f75b74a117d43ef3c2ff06223748500cf3be80ea76f62e0e379c05193fc7c02a20b8043730f9c70224ad8e9add5ae1299f7d658dcec0";
    bytes partialPublicKey28 =
        hex"099902e2ff850f9728c3d8590e4679dd7020016122a8b02a445734401d84c0a112f567ad2e59ad56597be09b32327d6c117aeae851bbb6a57834c540e70112532229525608e4bb77e19f5cbf6c29f0688b1a0d3e2c95130ca1b573937748406d0a2ddf83043f16338d16934655ab5bf0a98f852175fd0e5a98b3c814796c9ff5";
    bytes partialPublicKey29 =
        hex"15bba2efcd11ba492a05416bec6e9874fc8824e3ca203c836f09fd5cebfaa97f19da77dcf30e229213ba74700a8dc98584bcdef31a593ab21df1882cb39c04ea1770407e8e10e8641abe4c8272bc4c4dbffcebaf998da4da94da162bfab61c86138857ec6e5f29b9ec6f10bea46ddcc4f3f1a1616dca6cc75311db6e1f444c40";
    bytes partialPublicKey30 =
        hex"0439a467b71865456e12bc1e0288dbecc792f995b4ffe65edccd65685c6ffb2d263767e649803ef0e6c2e3bf337e15b96fed5fc9bc733120dc9d951a517a50b70676befd48323831428cf0fc248964f54c3243b05281d639e7d3dbe9c65cea1d29f43b91d1788624ecddc783680201fe37ce3945a23dcf1775d972b147349fd4";
    bytes partialPublicKey31 =
        hex"1bea29bd8ff674254af4cacfa0635d83a6a657245a3787dd1b02eeb48acc5bde0f45180089d1a063f6bfa4f5fe24432d850b09ed8f543fe80e518927a85faab81677681fd9599e3b40812ba667cbdfb1a8280c0a3d70ef818dd7357723c0bb0328a31c3ca59c2ebafd27082e191ea2b90cbecf91fe5996a4de6930870ec6ef2e";

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
    bytes DKGPubkey13 =
        hex"1084037cf652c6b899a802d89ba3d956f6db21df9600afb8130cdff87e3114aa1ec505018e95ee4079175390277f85dadfd4cb14684c7ccf70aaa5610243f0dc194fd9a91973df4b723f30f6745bd5fc6715c6b8b967485a8f56c382d8d587061cb231f5c0264b1da20ca62f27ceaf073520a14684a7116ac052e9a165312329";
    bytes DKGPubkey14 =
        hex"0e37da2d14dfed950ef4c957622fc404907dd4727c2dc8aa90d0a1ae7460cecc2e98bc51ce7e9ce78f99b29b4522ba9d5739673c139d5f42df5bb29fcf900d1e19df80ff457a30c8b1cc2ddd315fb0d90b3252c83c7bdd6598f92d5fb61dede10b3a12f5abdc24733561ad7840ed9bf630af66c782053fbba2fcecca3df6156c";
    bytes DKGPubkey15 =
        hex"074472599286dc642c53293025b5d3527421d3fd223aa03c8547cf532abbb28b1c3f6871d541068621b021693fd82f8e11fb5234a566162d223f1c802b0ef46b23ad911bb77905deddd950d6659ef30f9ff7e526c4b115af784c424c996152bc0a6c2ffb43789a6162704af6efd1073e5db1b26b0e0c075bcdef1d6720d112d2";
    bytes DKGPubkey16 =
        hex"2daa2bfd2aa5a52bf87fc9881b8853af1dff3ef078e57cdc008e139e9c9a6d0808b9a3893d0c6203525b5b6c061e03c15985e3b98b12112e138f2e5c019c94a506523cd4246eadd1c0f4ccec4ce6510d16af7a5ca81a0efef909ed157ca1f28e0e1c936caef01c77e2dd4e2fae2deb4f7667e403bd6bbf26fc22a51e534a1362";
    bytes DKGPubkey17 =
        hex"0c83420b3850f3a84cb89e6fb809b1cce466e565a57a9ca92caaec53c818fd161ca7218f79bff8a3f540533400b741e59dd805191056e006b49c7380ab7f20d11505619811f4104dd8f719783bdd17d243d868f657a34ef85d25f95ea147b30822fa86172140d0a448297763ae6d1762a6ff1ffb17ede6a7cfd1ee85849521d4";
    bytes DKGPubkey18 =
        hex"26f2d00ff3506f30c845e1394a50f0f1c13382cfc96a3247f5947da2646f70a8074b387f2fdbb22cacb665897a96583da6451ccf7c2ee9e5e4cfc722d361fd4b0071b8cbcbb33b784b960af42cb85204a348f0c7439e4b528f0293a1bbb6e7782f1357f7f6d195f81de7af348de72dc3cd46b25375fef546d13ced6c269b904a";
    bytes DKGPubkey19 =
        hex"24877c190e3cf4d6b28438de1073fc21f3fbb01185890fbb765dfd89134cee7829462ca92db9281fda2fe8c286e4a392fb6cea3ee7cc0afd3ea7bba2f6e52e272fdff2fb446c3262bc0c948addfe5246625d8302fed8faafd0eff2d41bb87c46074dd7e30f90d28ab6eea8dcda4acf99635e8e04a86a84a73673045fe5a6bff4";
    bytes DKGPubkey20 =
        hex"13525b51832fbd15d28710625b0f068ea54b83b06550068bb30ebf09d0d3dede17522390ca06c052a073472554ae9b5ffe3ebc827f0b9e101d77991466c1c4ff153c63bed03a1701c9c498409313d55114f3e1c4250e0993a629e38207cf4628128c51a2adb70132c9fff6f915c426c83dd7f0e903a27b81b09aed22e32a7254";
    bytes DKGPubkey21 =
        hex"0d7bd41999330dc5de39e3c7968109248ee0cb3aaef9886edf35b2d1795c4628264e0889d705a09a48f7ef7c977ab65c06047beb8eaf92f9d1d659f048aa7ee80656022f43ad8b8699c1a24e8524eee3b58e770fd228410ba8740c9a59f620bb18ab7766de1d8948ad3884c587e8a5db74cb8d855f72e839655f0bd4be3b3269";
    bytes DKGPubkey22 =
        hex"10a8926f4fc5c9fb47fca629f54788b7e35e8b63ae075df03c12384ad95ffbbf00f423cac9f5cb86120cdc6751c9b919f7453a122e6d35f6df4d71004a0f3fb307a6a0dd22b97eea9c52c9ebc35e782e1e8e81793b356e86877b7dd6831cdb352db62a3c87b1f3098a03e0997ced6ff3a56fa20e714a085716964aa38eac5e77";
    bytes DKGPubkey23 =
        hex"11d7e79762dd465cb6a14136ef2cfd640439eaa14d491f475ff63e20e6af2eab1e20cd5a19e935f347a242cca07a8cb19829d6a25ee6870710f6b4a6a263446d2b9e1b624618d8823e5805546ebbde824b027d2c71834f3f6c318e2acce955132a010223ac67791521a7b201a85f5089163cc00a50861f8b6c6d4c5858b3dd23";
    bytes DKGPubkey24 =
        hex"29b9cbb2bf062d92f9c8890f0b632417d9c4647de626a9db0024fa8ddbca62c82e29b689528668fb5977141c1cea3e6a7e5f01003ef5b7fbfe0d47800252e70e2f9c0645f7ded1a7d3f559e96e171777be887288eebdee7c1f00f108265f9e7322393582da26e44ba3b341a0777509679087fb1566a0b1718bee535740d6705c";
    bytes DKGPubkey25 =
        hex"23ec89e7530e59b1cff12a68fd3810111f7f5d26fd761b9ede362319c09b4fe82dbb1d5d5189f9c4bef5e609d8d611138c8403741697fe8c8ab27e3f86e556d90c507175aa253f40a6fa5524970dd6045db2e5b8a33ab94e236e8d785be0498811f8697477fa498f89cdaac500a8efd58e19d8f305ed74e795b4103d389c4667";
    bytes DKGPubkey26 =
        hex"013f9896d09307269853552158e678bcbabe2bf73a8d6de440814c2bfd004f7725588038570d6bd5f36b11d6bb30ef6afa127736074aead142366681aed0faf3119933b646d6b9a4eace32dde2eec7c99566175451fc7c2fd7969f83352ee77a289cfeb74d77382ce147b1a85c059f2c14cc120f1e36000a9d3f9def18f686e7";
    bytes DKGPubkey27 =
        hex"0e62efae973011fefd625ec0a30b0b21a60e51a6e0fc096d8d0decd96eb41900214c5417c16ee8dd755c5015ee76765f9ddcc786737a7eafc9e71b7464341d253020aba78aff9ec00d5f637541b4a16ef7544faa96d1b9b12bdb941ee57ca5711904981f11f26106edaa28b71b0dd86f6f76b59d4f8d6e87465c43940ef82f4f";
    bytes DKGPubkey28 =
        hex"30616a7c8793d2be94fc779a6fe53fc557eb35cdb7a2728bb6bb742a33fcb3da0b3479542894f9041b67a592e402c09304889ff4bbc91f6551c0574676092de2179aacb88e37db438c78ce251f045301d8257f51e1afcc1c7801ba0db2bbeba10f2e5d1f9f7959a852d723dc8366a5515cc4d6050b65e777aeb8eb687846bbbc";
    bytes DKGPubkey29 =
        hex"05815f165ad9b553ac646dc32d30b6ac091f363627271a0f930e621151a29e7f02b30a5e851f29b7f69cca0c5854e1fcd33d46495e6650397cc3b3f17f8fe73f129228283d5ec9140536ef185a378601b6eaac946432f3168a184a49c6e1d8ff204ca31dee68fb4c9daae9bffd0140f945b41645d0af31eeda233ef953937fdd";
    bytes DKGPubkey30 =
        hex"022221e5b08ce50d1ca6ab0a48a505b5a3bdde0df728046cab70291237579ac8296210e1acf702e24862999f209c11754a73066b228cf1d557e6336606e88b281b1840b659125bf92932447ba8e761fabad95162230bc46fc1f79c173501039000b4ac4cbf623af0a10943ba328d6dcaae3a36a32a6f7eaee41cec1405fcb6d7";
    bytes DKGPubkey31 =
        hex"06a8e68091b66c6e6213fdab7adcaa1c7ef2145aecbad0518de6398ea84175180f55f1c03517e174a95b4317ffee31c4f99bb64b6d0ea179ca7f8e86a54574780678fb7b25f7aa9ef3b7e4108834a5f5517f0d6ce295efb363323e45c434a6162a730a0a82bf1d78eb6ac49c2841f77ed71fc14e5df27625dbeb97de7b78628a";

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
        emit log("----------------------------------------");
        emit log_named_uint("printing group info for: groupIndex", groupIndex);
        emit log("----------------------------------------");
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
        emit log("----------------------------------------");
        emit log_named_address("printing info for node", nodeAddress);
        emit log("----------------------------------------");

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
