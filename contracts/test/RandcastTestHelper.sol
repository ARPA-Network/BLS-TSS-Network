// SPDX-License-Identifier: MIT
pragma solidity ^0.8.18;

import {Test} from "forge-std/Test.sol";
import {IController} from "../src/interfaces/IController.sol";
import {IAdapter} from "../src/interfaces/IAdapter.sol";
import {AdapterForTest, Adapter} from "./AdapterForTest.sol";
import {Staking} from "Staking-v0.1/Staking.sol";
import {ControllerForTest, Controller} from "./ControllerForTest.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
import {Strings} from "openzeppelin-contracts/contracts/utils/Strings.sol";
import {ERC1967Proxy} from "openzeppelin-contracts/contracts/proxy/ERC1967/ERC1967Proxy.sol";

//solhint-disable-next-line max-states-count
abstract contract RandcastTestHelper is Test {
    ControllerForTest internal _controller;
    ERC1967Proxy internal _adapter;
    AdapterForTest internal _adapterImpl;
    IERC20 internal _arpa;
    Staking internal _staking;

    address internal _admin = address(0xABCD);
    address internal _stakingDeployer = address(0xBCDE);
    address internal _user = address(0x11);

    // Nodes: To be Registered
    address internal _node1 = address(0x1);
    address internal _node2 = address(0x2);
    address internal _node3 = address(0x3);
    address internal _node4 = address(0x4);
    address internal _node5 = address(0x5);
    address internal _node6 = address(0x6);
    address internal _node7 = address(0x7);
    address internal _node8 = address(0x8);
    address internal _node9 = address(0x9);
    address internal _node10 = address(0x10);
    address internal _node11 = address(0x11);
    address internal _node12 = address(0x12);
    address internal _node13 = address(0x13);
    address internal _node14 = address(0x14);
    address internal _node15 = address(0x15);
    address internal _node16 = address(0x16);
    address internal _node17 = address(0x17);
    address internal _node18 = address(0x18);
    address internal _node19 = address(0x19);
    address internal _node20 = address(0x20);
    address internal _node21 = address(0x21);
    address internal _node22 = address(0x22);
    address internal _node23 = address(0x23);
    address internal _node24 = address(0x24);
    address internal _node25 = address(0x25);
    address internal _node26 = address(0x26);
    address internal _node27 = address(0x27);
    address internal _node28 = address(0x28);
    address internal _node29 = address(0x29);
    address internal _node30 = address(0x30);
    address internal _node31 = address(0x31);

    // Staking params
    /// @notice The initial maximum total stake amount across all stakers
    uint256 internal _initialMaxPoolSize = 50_000_00 * 1e18;
    /// @notice The initial maximum stake amount for a single community staker
    uint256 internal _initialMaxCommunityStakeAmount = 2_500_00 * 1e18;
    /// @notice The minimum stake amount that a community staker can stake
    uint256 internal _minCommunityStakeAmount = 1e12;
    /// @notice The minimum stake amount that an operator can stake
    uint256 internal _operatorStakeAmount = 500_00 * 1e18;
    /// @notice The minimum number of node operators required to initialize the
    /// _staking pool.
    uint256 internal _minInitialOperatorCount = 1;
    /// @notice The minimum reward duration after pool config updates and pool
    /// reward extensions
    uint256 internal _minRewardDuration = 1 days;
    /// @notice Used to calculate delegated stake amount
    /// = amount / delegation rate denominator = 100% / 100 = 1%
    uint256 internal _delegationRateDenominator = 20;
    /// @notice The freeze duration for stakers after unstaking
    uint256 internal _unstakeFreezingDuration = 14 days;

    uint256 internal _rewardAmount = 1_500_00 * 1e18;

    uint256 internal _t = 3;
    uint256 internal _n = 5;

    bytes internal _partialPublicKey1 =
        hex"18348db3ede7292fc88600b96e4d3edba371027a5533ef42fadeae694b2a3cd12bfcdf2dacb1d9fe8ff626c65edae4da2f052d385acd0354ec08e20a22b0debe2ef44d4f6630495e64cbecec26259fb2eb6ca4ca62ee0cafbdabc34cfabac32123262af20af318c9cf267ced51e649ed07c9285c16b58f8d12fcc32b5665f1c9";
    bytes internal _partialPublicKey2 =
        hex"0b261af35ef536b756e42b2163b5d5c92299f84b33f88a47308107a48eaa1e2108fb1f0b2f96744aeb8812a0d8c06d873b2d04ff442992f6fcaf9f53880f1f5d25d023d4b027214876040c3b21eb71fb8e78a2f60983a7016635cca1c42c179f1c7b6c98574a8c0614a83de7cb6a580d90a8e9cb9300b11a4f5d34356c299e02";
    bytes internal _partialPublicKey3 =
        hex"26f57d792783c664ff3258add467e95cdf4dda9a4c7b8acf8c0d3b27ce9ac79629a1b770c75acf41167e1f8b553dbb937b58fb00fd78f84a99aa9fbdc4e804c92b8b23e9169ae02b6c9798fc7683d3e7a8fd78acc802804b26fc48cfefc73f772731237bcf18b4caea40b8ab91197c801596a5d8afe8fd2a851db8aef0085678";
    bytes internal _partialPublicKey4 =
        hex"229bc153dae4d0c8faef82b65ef8a2fb6ccfca516f9920e6689432024eddb134052cc6a0b30899b864f130c241ddc98f5f22190f9e09097db908a9462c341e4a2224f12c0bfbeb89d8c4b2bf29b17301ba5b3e262406803e5367513babc3f5300ce97cc52ac34e2ab2721d5985e6e547d05f11e63dffba2f0be68f0c02faf340";
    bytes internal _partialPublicKey5 =
        hex"130bffbcbe3c30ccd3587a210a252f093523b827e9e2c2eabd43b4cd8443fa5b146188767023935a380c753352cc2fa308a1089c7f4760f0f99b4c6d3eff90812ff86aa2125f6d1e1809c44eeb4148586ad039a60b3494ec89494798537100e818800b908edfbfc281a2eda277e552f7f160aa93a70bb7f16c60074a2b114c67";
    bytes internal _partialPublicKey6 =
        hex"02d0d020c4f1e28649af7e44e80673bde0c9735e80d476a9112c335c576f287f0babbc8dc54a7415ea528b44014bcdbcf80958cf96f23ff31b33be662aa7c64a0b93d75b674a56deda1c85aed61963d42fc52300f6564638ef0db361d19d9f47090c76742f6e912a38d252d970655cc10d368c96819c644c07a1c3c3e270ec5b";
    bytes internal _partialPublicKey7 =
        hex"2d029c56156c8d50a80308be72e4c575169ffadaa0ac3fdbc3a555108e30499a24e544b89745307f62b944872c0a57eb01014586b6a262879896bffa34720815001e3eb956f1ff5403645a93fbc8c8cd6f491f0f8173fcb2e63a4fd69cda090c108f1b6b7ebf9063ca951c5fad7858db146d62a4b744fd194085eae03272b8cb";
    bytes internal _partialPublicKey8 =
        hex"2cf6850ef8b8f238d4d26dd457cbcfc5c0b5880b33598b6301b02b27806f931f180c697dfe128ac546cd2df5ce2c3d69f599d6f00a4afca3eeb8eddfe4fbe80425b381e7272244483cbb85a4f6e95d72c2c402cd28d089b772322b4d7f10f5ce011c55f4f40527a790f59c573e932f734372f4a621443ff15993e518c2244869";
    bytes internal _partialPublicKey9 =
        hex"01a64349a99285f39d94358c60fa8a7981ba01905f5165e84ce5f71bb11fa90905aa35e57d8c975129ec30a8ba9bca5e301d4af5a69efd91fdbd8a6da2d8e7e7011df0d088ad0c45304a84d8ad6227e2c12cbe84091d9ed1a43f60334b3e99a61c3d6fed8facfec7f8d432bff82f459648aa18ef15ac39b65211d33afee9c96e";
    bytes internal _partialPublicKey10 =
        hex"15078251b04e3224fc23e46bc6edb1153ba3dcf3e306d86b7af18bc2dd9d175a245a89a244a1d2534dc688d623a2c958a16d17c5e5887c48638fd3fdd0e514eb299223b68ba5449e588b26cf7ccac4c9960b8be8bb75e7cc4d7b6eba9a185698084beefa7569783f895db22a8cfe1bf0ce327290dbfee7404378fa804e9a504e";
    bytes internal _partialPublicKey11 =
        hex"09fbf5a67aebad6fc4cc3c8172dfcae20a78c0c0dbcc66cbac30548449040d9107be1d2b0274ca7641e15c63956b17f940e71def2597b3fd855f425f02d52bde01bb35ca79a007321d840a112ebb18246daeb3d6a0cf2cf5009c88a7296ae9fe0fb9015b63965664e1bde1d2b438fa3f47a34868037b51984b458df6de2fcde2";
    bytes internal _partialPublicKey12 =
        hex"05ae3faf558b76d653f603816db30298dc392dd7085fb2261dc1ac94aea992402f22ce63d538c0c3b279ef1f24ca742ad9fbadd2d1661558ad42448820d45f38054205d4aa7ccbf13a392bc0156028be22dc4b246ff55c8be89a1a6aae0072800ec9e5e920447828f24bab06673b140553194ddb4816d3569f05b321fe6a16ac";
    bytes internal _partialPublicKey13 =
        hex"1f81d6f66cb9bca463f5c982932051d6efcd3ebacf0d329acb34eb88f99c37ea24e4e5250ada8f78e26115874510868a83e5a8e8bc2f02c4b68bef93d1ff4eff1d599183c52d1854e690393d7fea0c1742b3a6cb65da7962567857dbc9ec59951890e95411457f1eddacddd8e48a2bf20d387e2ef0c28926b2670af9be2651da";
    bytes internal _partialPublicKey14 =
        hex"15deb2c08af87609f33dda5e238d2238171f02c8b83efb0dd219a08cb00fd4e5198c8b67b3b23613ffaadca04a915bc3c7b4005b24c95c9b5bcd359bdc4fe5be0ae74e03ceefe4320bd7ea922dcd10e5544968ce712c8a3528184cf1711d3abf2397fbde04db466d56bc76e00ef98b3998e088d9d5df77db7db2b3b47c6fe6da";
    bytes internal _partialPublicKey15 =
        hex"1b2fe929bf337dcd8b6cff5987fc6ab9d51786be5d4a4f30cf93043fdb4d2511106a8eae1ef8f27e5a075aa9ab77c31d9e1e4754e51ac3e05713784f614088081d8cd701bb98ffe3ea4abf9cd7688e19bff4c7df39bb3374accc16fb5fa4cc820469785848e16513a0a10acd9ddfbbab71e9ac609a7c4b392ae71d105cfe12c1";
    bytes internal _partialPublicKey16 =
        hex"1842af364c4ecb6b03817282013f6dea6ab8030430211c6864cfa15a2be7b3572601183be460fef786efdd547ededb7d66db01c26bdc1175f95329e69e6c4e5a058ca48103cbf2fe2305043840d6f2931b220e880966bae80329ea11d2ef66652dfdaf3e7626670852c08580e066c04dee85b87b5057d4f63ab6307e976b7075";
    bytes internal _partialPublicKey17 =
        hex"0dc9b25ce21e70260ec8c2d00a958cc3497f31f97d89ead7e53db913e545513403afd4d8127650e27a1a4ab407a3885661fae54ad528307d408f2434e64ad8e720ca10878afcd2d7b223827bbc0bf2b470cf370dafb4533a5741ef30d194c71526092da4f087e82b74c6978c7395466d61cafe7cde14d204f65e8f90ab37685e";
    bytes internal _partialPublicKey18 =
        hex"2aae5c35388f7c56641561a94b10a3041c5c0f89a85c7e84efd0ed12a16847601fa9061813e60c15356d31b94f76d5bc49cbca1ad692faa263b7f51821f3c9490c18aa2d239df8c86761447b00f13e20ea228bc77d5d4f3fc5229705c4c0a47e189bd1404768f72f3f9e05c246268a0442fd0f74d0946935f4e629711df0ce29";
    bytes internal _partialPublicKey19 =
        hex"16d7176ca5d3379734a3382b176240561b4d0c67d9fe9eed329ef8e7da3ed64f0486ecb6297351577067ca04275a22459cc78080ce542d7019341194b48a04912bbb04c939ec7974ea7964b6b63c949b6471b0b285d8a3964321482f20110abc1dacb8b2d98869d4087b56c73edaf2433a0e15da5763ae73be3b1325fbc3bb4f";
    bytes internal _partialPublicKey20 =
        hex"1e59a2366be62d1e8a896042c6e730836d4fd86b08ea99b3f9a321476f28fed82903085edebbbee3311327411e2a516752e01628f99fbdb7adabea05087edaaf0c77c193a9bf00ecdd6db17f4bd0a62906778f05288ea41232520a5173177d42268f4edf76aafa4ef866f9ab05edc5d934ad3c6a4cf4791494d69161c63ebb93";
    bytes internal _partialPublicKey21 =
        hex"1ebcb6d0b928d20ce54ce1308aea4d2a494097af99abf6cffe1d70b06d51990315ec31d094ee9669d982573075ad5d6e821141067ebee72e6bdceae012f6edb21d9675c2f7966b93c42812fe56e37022b1ce490dea4b32e2e30b912e7a6b1f30112fc7975d304dfc642008db588ab6e508969df9e2810d2460993138022ad7fe";
    bytes internal _partialPublicKey22 =
        hex"168c11170372c213d211e63981f8f1b1197421e36a985ca280c4ac22271ecbc7122115c328f2d50f8be64c13e3ad9131273b04dc178be81abcf4e341dccf4a412deb96eacc8c1c29212aa33a33b319c97b3953f453145a60c77f40b55eb1e43f1cd502ff5ab9cbc571e0c313e9d8e24cd371b67dbf69c9251b579dfef4006f7b";
    bytes internal _partialPublicKey23 =
        hex"0cf07b941e16031a517c04ec9d609cb40af7b52bce8e303afd9c562dafa6529e1d125fa582577d156a5d75fd4e310c9e57b89aca02975918d94c30a42ae4518e270e84b4e54bff80c90cb3d380318f203c03dd4b68eff7b3f2ab40e7e240a91422453f756bccc6f08ca924c1e7761f16e1eb8a04128f0f5befaa01b3c2861495";
    bytes internal _partialPublicKey24 =
        hex"169700ccf896f8383fca4bbfbdcd904f0660137df673c17a64982778fa5ef8b5146611822a5b2b1d0a12014ac7fa3edf8fb4c03a88d7472259761f645225f1f11f13985b9d9d72c2898fcc861e7ed4b903a6c4264e2f88011746042eef548b302ec9b46274fdd0cf1e90c7265eebd6d8a978ae13c3b437df61bfcc1e833f6675";
    bytes internal _partialPublicKey25 =
        hex"25d0b18ce3db3f6fd89cfe9ded93f0de2b8706ecdc881ca55fb9a7bbbda1bc2b14562863a93e48fb7adab2215adda74c60cb4543764287416e1b3f763698eac517f91167a4ec794fe40ac38686853c6afb903ac7277188b24f864f21092cba5819af94354e87da41949bb6f0147d8732603a0514ce035b38fcc4500f0635002d";
    bytes internal _partialPublicKey26 =
        hex"20e635876bff0c85c6483a8aa226a77ab5ca1400a42eb437815b87736be467112856c74f65cda9d687b15a31f6ee7013c87d583fe0a136c83de26bc58aecbe100bbd4b7fa1ac87dbf0a22e6b7552b47f066aaaa139f24ce7a254c0c95f727bd42d16e47e516185b08f418078b136da8728f1dec618774ae1f3c3de91a76497a6";
    bytes internal _partialPublicKey27 =
        hex"2bf5e7817ffdcece5c95ae51bfe9c3c9c1c88f7f0fc6e71007c0975932ee299c17bc336982a24d3152c0fb4cc2c20d7169f12577d5ef5c0e7f0ab47ad25f3c6205b3345ec11c7e14e4f7f75b74a117d43ef3c2ff06223748500cf3be80ea76f62e0e379c05193fc7c02a20b8043730f9c70224ad8e9add5ae1299f7d658dcec0";
    bytes internal _partialPublicKey28 =
        hex"099902e2ff850f9728c3d8590e4679dd7020016122a8b02a445734401d84c0a112f567ad2e59ad56597be09b32327d6c117aeae851bbb6a57834c540e70112532229525608e4bb77e19f5cbf6c29f0688b1a0d3e2c95130ca1b573937748406d0a2ddf83043f16338d16934655ab5bf0a98f852175fd0e5a98b3c814796c9ff5";
    bytes internal _partialPublicKey29 =
        hex"15bba2efcd11ba492a05416bec6e9874fc8824e3ca203c836f09fd5cebfaa97f19da77dcf30e229213ba74700a8dc98584bcdef31a593ab21df1882cb39c04ea1770407e8e10e8641abe4c8272bc4c4dbffcebaf998da4da94da162bfab61c86138857ec6e5f29b9ec6f10bea46ddcc4f3f1a1616dca6cc75311db6e1f444c40";
    bytes internal _partialPublicKey30 =
        hex"0439a467b71865456e12bc1e0288dbecc792f995b4ffe65edccd65685c6ffb2d263767e649803ef0e6c2e3bf337e15b96fed5fc9bc733120dc9d951a517a50b70676befd48323831428cf0fc248964f54c3243b05281d639e7d3dbe9c65cea1d29f43b91d1788624ecddc783680201fe37ce3945a23dcf1775d972b147349fd4";
    bytes internal _partialPublicKey31 =
        hex"1bea29bd8ff674254af4cacfa0635d83a6a657245a3787dd1b02eeb48acc5bde0f45180089d1a063f6bfa4f5fe24432d850b09ed8f543fe80e518927a85faab81677681fd9599e3b40812ba667cbdfb1a8280c0a3d70ef818dd7357723c0bb0328a31c3ca59c2ebafd27082e191ea2b90cbecf91fe5996a4de6930870ec6ef2e";

    bytes internal _badKey =
        hex"111111550230862eccaa516975047156d5c7cdc299f5fce0aaf04e9246c1ab2122f8c83061984377026e4769de7cc228004221275241ee6a33622043a3c730fc183f7bff0be8b3e21d9d56bc5ed2566ce3193c9df3396bd8cdc457e7c57ecbc010092c9cf423391bff81f73b1b33ac475dbf2b941b23acc7aa26324a57e5951b";

    // Group Public Key
    bytes internal _publicKey =
        hex"27effe7b4b2737fc0b7b5257473a5d94ea40e1537e3132e61eb0ab870697ce75099ba18cd50d2e27614e2e41033dac811a2a9483fc8e83667c279be179eb163518b09aae20fa812653d84a7bcd362a2130870688faa574fb39fe3b2d20a27bf50aaf5bc665ec5bcc9179ec0544976326ac7af92eba5a455c3b193599db5b88b4";

    uint256 internal _partialG1I1 = 0x9ae4621c627b5c710431fe96de35af1bb1f71b07c96100f21edbbb3d92eec83c;
    uint256 internal _partialG1I2 = 0x8a179b3e683e5263dff2e6f4bcd4665b969e2368a3d2b5ee5686ab470d9cfb54;
    uint256 internal _partialG1I3 = 0x158365dff6c39f7e9a2060337f461dc518a277f2d4a387cbb74577d9014ed3d8;
    uint256 internal _partialG1I4 = 0x95553d3db9e37bd8fe5dcad78c432156e32e3528244232233bbf98f217f36a35;
    uint256 internal _partialG1I5 = 0x9810802b90b0f12029469fe771fb63221edeedfdf14621d9d71586bba5e70d96;
    uint256 internal _signature1 = 0x0ee1820794c681653fdee5b5721d9c2903cbe670e75fdfaee06dffda777be1aa;
    uint256[] internal _sig1 = [_signature1, _partialG1I1, _partialG1I2, _partialG1I3, _partialG1I4, _partialG1I5];

    // msg: "84a4be63154932c9cca9a34f9a931f1f0fba5d28f76c67f8f192fc9a3f66920d0000000000000000000000000000000000000000000000000000000000000012"
    uint256 internal _partialG2I1 = 0x9bdaadeb7c98aa3b1cc5271de4517b7c5a91bc28a89fa0dfa2823e5c78d1d6e3;
    uint256 internal _partialG2I2 = 0x2ee67d1ac7fbf4cd5fd7a38360a9a687ea82fae7204bcbeff3366284d4848112;
    uint256 internal _partialG2I3 = 0x0e0bd13e4d58fcc0f9e248fe3753e3a25797b94fe52e90fdfe5002a17ee9372e;
    uint256 internal _partialG2I4 = 0x8b9ecf7201369e1f6eedf5460e9e630e73bdb6cb7ca45e5de02bda7cbe0a400b;
    uint256 internal _partialG2I5 = 0x9261e5eb144448a8c0558c7c3d44da82104cc869ee67e2304c1c8f8681cd3b27;
    uint256 internal _signature2 = 0x2d6e04ae42003d4de3d00061da94fbf25a52a99cdeb1fdb894c7bf237102b6fb;
    uint256[] internal _sig2 = [_signature2, _partialG2I1, _partialG2I2, _partialG2I3, _partialG2I4, _partialG2I5];

    // msg: "226a2536aeefd4a8acc9c5ce74f8b152a782214ad8f341886c9530a0ebbcf42f0000000000000000000000000000000000000000000000000000000000000023"
    uint256 internal _partialG3I1 = 0x8ba5392eb8d0671908236d27439fd8effa6cf8d1c046977ad4a9e53bf489948a;
    uint256 internal _partialG3I2 = 0x0b29b844e6b053474e8a28137cf88d5f0754417f7bb1582021183e5de8be5841;
    uint256 internal _partialG3I3 = 0x2b27dab470540b9d399880a37de1da1e4a9334fcae20482c9057e4d28464df84;
    uint256 internal _partialG3I4 = 0xa28315c678f52bf8f0c88bc1abfea115a6f68ac93e5f9deadd11e2073e99e681;
    uint256 internal _partialG3I5 = 0x9226caf7efc028f80e955a1048aefa83db9b55fc0ac98f8d2a89a41bf412f211;
    uint256 internal _signature3 = 0x8b97ae94f5089c0db54abe32654aa17254e18b13a4c6b2c180344c057d987b63;
    uint256[] internal _sig3 = [_signature3, _partialG3I1, _partialG3I2, _partialG3I3, _partialG3I4, _partialG3I5];

    // msg: "75fc83a48d1fac46f0d87e3752dbbf391ce3e3158f954232b10b8662598d9ee50000000000000000000000000000000000000000000000000000000000000034"
    uint256 internal _partialG4I1 = 0xad7c08a0213c63f9549ed72fb0f200ea9bb474b370cf5cfb1c2b7dcf8cab353f;
    uint256 internal _partialG4I2 = 0x038221bc08693401306fe1d71cf90b662cd7bb000210be2eb32ea0b4a1c05bb0;
    uint256 internal _partialG4I3 = 0xa8d9d02764f2a140296559d145cfb62b6f852307014dda6bbd361acfbf842071;
    uint256 internal _partialG4I4 = 0x00c62ab470fafe8bd5d5a7ff1c3c00341781a085ced4c926d3a273c55c7f6abb;
    uint256 internal _partialG4I5 = 0x00a29a2d1975e00d34f52c4aa3c80648c840717a279d90b679adfd3050b7f657;
    uint256 internal _signature4 = 0x1ed9f00d623f14113b3f03e610227f59592cdff823e8ea845a574c8928f895dd;
    uint256[] internal _sig4 = [_signature4, _partialG4I1, _partialG4I2, _partialG4I3, _partialG4I4, _partialG4I5];

    // msg: "522f5dc85539775dc0ab01953f6a286c7ade1d2555cd84e69c26e3b783c61efe0000000000000000000000000000000000000000000000000000000000000045"
    uint256 internal _partialG5I1 = 0x20d45360466b99f3f2dfdbe664a0759419ac72878f6b9f4c19e6649a6a0eb011;
    uint256 internal _partialG5I2 = 0x2535f2148bc9ed276683bce95a8602ea74569bc3c59f6214b043c6a0a2af2ac1;
    uint256 internal _partialG5I3 = 0xa7e65520be24eb9c3dae9a1fc3b9700594d3553bdc147eef6905b8a59fce490f;
    uint256 internal _partialG5I4 = 0x1f54759adc9251580d1e7200f5721286737f7472265abadcc3dfbea1e420b605;
    uint256 internal _partialG5I5 = 0x2ed95c976c48862f93a08966ad891e42920c29f74221b92e593e47810a38114a;
    uint256 internal _signature5 = 0x99c07d16e4001773eff30b95a5a8432e795d595aea078849aee87d8633efd1b3;
    uint256[] internal _sig5 = [_signature5, _partialG5I1, _partialG5I2, _partialG5I3, _partialG5I4, _partialG5I5];

    // msg: "35923fa15bca24d755a28af8fc280d76aef33f7f2a2a13e1e3de99733f3457890000000000000000000000000000000000000000000000000000000000000056"
    uint256 internal _partialG6I1 = 0x0aef1ce894e1aa4a77aa5aa34c1c707fa31a450a2cee8df54299cfadda537eba;
    uint256 internal _partialG6I2 = 0x9ab3ea2c5908d1cfe1a8a42c82e414009968168f82861d40551e358d317f52f2;
    uint256 internal _partialG6I3 = 0x30638ba1f28826754b09765722c5cc7db1ea35fc4ffd59bfd79c807b1f329d22;
    uint256 internal _partialG6I4 = 0x276998186f5d0371bb7be0cae20b7945066536d2ce7c6ec0841284c02c481cdd;
    uint256 internal _partialG6I5 = 0x03dc4d14c9618d4a7cb571f709d53009301283e44ef12e7458152c965baf4011;
    uint256 internal _signature6 = 0x9904f1192103acb4f0e9db719e905bd61ef75a83102a0fdbae9801e1174204b4;
    uint256[] internal _sig6 = [_signature6, _partialG6I1, _partialG6I2, _partialG6I3, _partialG6I4, _partialG6I5];

    // msg: "f87a5f99c8a4fc89828ebb2fc631a9189b5da12804c93ebf2b3343c303957fe10000000000000000000000000000000000000000000000000000000000000067"
    uint256 internal _partialG7I1 = 0xaa13ffd490f0e289c6779ce650897cd7f9f41c6c75ab0551f038c23c9bc373f6;
    uint256 internal _partialG7I2 = 0x842b3df807004770db8c8b6c61b305642d6a86b1228678e3838dddc3c02bceae;
    uint256 internal _partialG7I3 = 0x01145759f894f72ee0e968d9b6964090cdecba3e20a0bfcb8cedce81336ac9c0;
    uint256 internal _partialG7I4 = 0x20198bda63e49748cb333d80992468ba212fb9d2d056a3e51ea66c2d89b4edbf;
    uint256 internal _partialG7I5 = 0xa65303f613dbb1ae15466b7a54f32f8ad8c722eefd489f4cd5fe3b5579263daa;
    uint256 internal _signature7 = 0xaac858d2dae9a8d8fd94206d9d9498324aee7d0b090d9309ab454c547a14429f;
    uint256[] internal _sig7 = [_signature7, _partialG7I1, _partialG7I2, _partialG7I3, _partialG7I4, _partialG7I5];

    // msg: "002bb5e97e37f9b8f0216be6ae850c12105593b743bdd56972a498347bd9d6e80000000000000000000000000000000000000000000000000000000000000078"
    uint256 internal _partialG8I1 = 0x041df8c45bc32b2759833355201b446a6f7c16c5318f1000dd63649eb89b0467;
    uint256 internal _partialG8I2 = 0x2ea0e58215b440c99a006184dc51e010ebc62cff7ee9cc4ef8ca9bdf858cecd6;
    uint256 internal _partialG8I3 = 0x2ecc39eef8f8b880f0d59686478e82d949cf3f17d22ccea55f10c9b9a09440d9;
    uint256 internal _partialG8I4 = 0xa0f5edeb69ec35191b39e53266ae894c70b84ed6f429f6a7394b90de122cb10c;
    uint256 internal _partialG8I5 = 0x8a07d0b8b079d3f3d8b15cf53dc7a84e94a8c9de6c9291b02427aebebea70f39;
    uint256 internal _signature8 = 0x8d3b5204c2abe37e4cff47da7dad0325ad3b319db15ef099250f587fdcc650ee;
    uint256[] internal _sig8 = [_signature8, _partialG8I1, _partialG8I2, _partialG8I3, _partialG8I4, _partialG8I5];

    // msg: "66ecc5843bea2256c4464dae004e7f6335cd6da72bcc8fa905f6a950a361f9480000000000000000000000000000000000000000000000000000000000000089"
    uint256 internal _partialG9I1 = 0x92914b6cee2520dd61a13fdc003120a5f623533342a2bb6c59f83addc809755b;
    uint256 internal _partialG9I2 = 0x1e2b8b332fc43a418e08e3afd0489d31996bf0056c15b14cc0305d405d826595;
    uint256 internal _partialG9I3 = 0xa384368a6119721676bec9ef15fe1df2fb4d5c104563075993516b0449f1fc65;
    uint256 internal _partialG9I4 = 0xa0f54fc59f6b08cf54692b9dc2bf34cf4b989ef5b94707c38b86ec248d581e84;
    uint256 internal _partialG9I5 = 0x252f6dba5d1ddf4563ee9728c4b55b22df07a83adda9577341d7c70b94f8b388;
    uint256 internal _signature9 = 0x2e5f68c2a88a22807df27f7475f0b4c27898a44fb3b5fb20c9d8d24a6f0316cb;
    uint256[] internal _sig9 = [_signature9, _partialG9I1, _partialG9I2, _partialG9I3, _partialG9I4, _partialG9I5];

    // msg: "c8eaa51e153e39c71e3bece1af0856a976ed93ab5a4bf4b17f5a2070e480e97b000000000000000000000000000000000000000000000000000000000000009a"
    uint256 internal _partialG10I1 = 0x9e74546a1c995aa23b6d75a2ed82a17808ce0d363a53366d551062c668c075f6;
    uint256 internal _partialG10I2 = 0xac82391d37195feca20a1ba1cf7f079fb277c9d0872774999f19789fc6d5e4b4;
    uint256 internal _partialG10I3 = 0x15a778c4c5843ba86add9b0b528d0f4399dedb74e1d06a57fe5e7168def9290e;
    uint256 internal _partialG10I4 = 0x1cef99b1186c1e7e8bb52a85abfd91c01401e4e9473e495aa10aacfd25afa7ed;
    uint256 internal _partialG10I5 = 0xa73028342c9e5afeef44aa42691768b013a855a0adf97c0a30533f2f305ec4e0;
    uint256 internal _signature10 = 0x81eddf3b3a0e1d5a863b179b258eb75a3508fbe6960e43363bb97c064fa8ff2a;
    uint256[] internal _sig10 =
        [_signature10, _partialG10I1, _partialG10I2, _partialG10I3, _partialG10I4, _partialG10I5];

    // msg: "6ddcbff04cf4d7733db9b763d93c9c39c11097ae2121800d0d1f2c94d531c8bf0000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG11I1 = 0x8215a95441b59cd0d00c07c12d0cac71b23c476e179baa2201ed09a211b510a7;
    uint256 internal _partialG11I2 = 0xa262bb988def489130bb6de5128af7954dc74f1038f5f10275723f77f162baf5;
    uint256 internal _partialG11I3 = 0x1c45c1abbe6f84aa4565835be5492be42a037d4a3fa19f913d02501e01d6d1c4;
    uint256 internal _partialG11I4 = 0x2ca3f4b59c447939eb5c8aba6cc45d9f233b15f6009b40cc59287824cfa40b41;
    uint256 internal _partialG11I5 = 0x264f8788cc6b06112a91e0a4cbb4f93743ed6ca652f69f447142ac7983707c3d;
    uint256 internal _signature11 = 0xa6ede719438175b33ebe9919202cced3af6c5e22039ea84b4abbcb87e319ffc9;
    uint256[] internal _sig11 =
        [_signature11, _partialG11I1, _partialG11I2, _partialG11I3, _partialG11I4, _partialG11I5];

    // msg: "4cb0e29a4928e365cbe774e0958305b6879ab501c8398fc2394d928ec324ff670000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG12I1 = 0x07c236639fe6a5f23bca92ca7560961389cd2255784bb2bbde7059885b874f38;
    uint256 internal _partialG12I2 = 0x9a29665fa08c2c0bfdc97fca024868f753762d1d0ca76f1efed2090c679714e5;
    uint256 internal _partialG12I3 = 0xaeab5dfad710faa0e53e80af078a6f6613b7b8d5d5ac70c7d18f1d193e134775;
    uint256 internal _partialG12I4 = 0x936ecc09ff79b49ed15bf70ab8a7b7f53c8c4ad13b70352ecbd55148c2314ee3;
    uint256 internal _partialG12I5 = 0x294fbe72c5d0aa613eb50a681ab1328298fcd46ceac44a97be61490a8271ab07;
    uint256 internal _signature12 = 0xac72f3b4beae62b1e917316e5cfd7d877b80c1035b0c9f41b40252c4bfcadfbc;
    uint256[] internal _sig12 =
        [_signature12, _partialG12I1, _partialG12I2, _partialG12I3, _partialG12I4, _partialG12I5];

    // msg: "2a276927aef9a0cb4ae82727c188b2988e782e2e94d8727984b6e76463ae9dea0000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG13I1 = 0x28207d8834355c11e5a2c32efcbe65c2c192e098d7579d694a4018d40feff232;
    uint256 internal _partialG13I2 = 0x07a0da074522c7da232c6a0f825dd3d26d73f58897a21fc270109998ad46050b;
    uint256 internal _partialG13I3 = 0x046ceb33878526029cc3634405b967212b143cee92a04b8c761e67210bcada23;
    uint256 internal _partialG13I4 = 0xa9070d7b669a3e0d8efa1c1e1595a46aa408021d6306e7abc7fc8037708f78c5;
    uint256 internal _partialG13I5 = 0x8efca3f76e36445da841999e241f00f5faab66e615dc7a2bac944aad9403e66e;
    uint256 internal _signature13 = 0x976015590627447594a9348146022a8d096dcea1cbcdc0884cfa4804deb3929f;
    uint256[] internal _sig13 =
        [_signature13, _partialG13I1, _partialG13I2, _partialG13I3, _partialG13I4, _partialG13I5];

    // msg: "03c3e92afb3f0269f37ea28e202bbd315b1d709cc01f2c2afe2805fd4c80d2d40000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG14I1 = 0x9ae4621c627b5c710431fe96de35af1bb1f71b07c96100f21edbbb3d92eec83c;
    uint256 internal _partialG14I2 = 0x8a179b3e683e5263dff2e6f4bcd4665b969e2368a3d2b5ee5686ab470d9cfb54;
    uint256 internal _partialG14I3 = 0x158365dff6c39f7e9a2060337f461dc518a277f2d4a387cbb74577d9014ed3d8;
    uint256 internal _partialG14I4 = 0x95553d3db9e37bd8fe5dcad78c432156e32e3528244232233bbf98f217f36a35;
    uint256 internal _partialG14I5 = 0x9810802b90b0f12029469fe771fb63221edeedfdf14621d9d71586bba5e70d96;
    uint256 internal _signature14 = 0x0ee1820794c681653fdee5b5721d9c2903cbe670e75fdfaee06dffda777be1aa;
    uint256[] internal _sig14 =
        [_signature14, _partialG14I1, _partialG14I2, _partialG14I3, _partialG14I4, _partialG14I5];

    // msg: "6ddcbff04cf4d7733db9b763d93c9c39c11097ae2121800d0d1f2c94d531c8bf0000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG15I1 = 0x8215a95441b59cd0d00c07c12d0cac71b23c476e179baa2201ed09a211b510a7;
    uint256 internal _partialG15I2 = 0xa262bb988def489130bb6de5128af7954dc74f1038f5f10275723f77f162baf5;
    uint256 internal _partialG15I3 = 0x1c45c1abbe6f84aa4565835be5492be42a037d4a3fa19f913d02501e01d6d1c4;
    uint256 internal _partialG15I4 = 0x2ca3f4b59c447939eb5c8aba6cc45d9f233b15f6009b40cc59287824cfa40b41;
    uint256 internal _partialG15I5 = 0x264f8788cc6b06112a91e0a4cbb4f93743ed6ca652f69f447142ac7983707c3d;
    uint256 internal _signature15 = 0xa6ede719438175b33ebe9919202cced3af6c5e22039ea84b4abbcb87e319ffc9;
    uint256[] internal _sig15 =
        [_signature15, _partialG15I1, _partialG15I2, _partialG15I3, _partialG15I4, _partialG15I5];

    // msg: "4cb0e29a4928e365cbe774e0958305b6879ab501c8398fc2394d928ec324ff670000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG16I1 = 0x07c236639fe6a5f23bca92ca7560961389cd2255784bb2bbde7059885b874f38;
    uint256 internal _partialG16I2 = 0x9a29665fa08c2c0bfdc97fca024868f753762d1d0ca76f1efed2090c679714e5;
    uint256 internal _partialG16I3 = 0xaeab5dfad710faa0e53e80af078a6f6613b7b8d5d5ac70c7d18f1d193e134775;
    uint256 internal _partialG16I4 = 0x936ecc09ff79b49ed15bf70ab8a7b7f53c8c4ad13b70352ecbd55148c2314ee3;
    uint256 internal _partialG16I5 = 0x294fbe72c5d0aa613eb50a681ab1328298fcd46ceac44a97be61490a8271ab07;
    uint256 internal _signature16 = 0xac72f3b4beae62b1e917316e5cfd7d877b80c1035b0c9f41b40252c4bfcadfbc;
    uint256[] internal _sig16 =
        [_signature16, _partialG16I1, _partialG16I2, _partialG16I3, _partialG16I4, _partialG16I5];

    // msg: "6528885a4c73f9fca5701f4d6c75201267e8f0036e1f5c1ab4c130f655377f940000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG17I1 = 0x94e61738331ba3be9aaf6edad90d4fa1a1124e06eb92b906245b4748c9a08c06;
    uint256 internal _partialG17I2 = 0x9afaf219b452211b470a2363ed082fc4ce6858c2b52b42113d60fdb4650964a7;
    uint256 internal _partialG17I3 = 0xaf777b5be9b087bf0c559d78ad49c62c0b02e22a5e84912ba703d3f6dc967c4d;
    uint256 internal _partialG17I4 = 0xa758283f5fddfff9241256cee1c4477ad07e6299ffcc277dd2f106b4a5907fba;
    uint256 internal _partialG17I5 = 0x017ec35f4f6e34e53a0fe22e9f0866e292a1237e299d6cce429378eb2250b686;
    uint256 internal _signature17 = 0x275a3cc1b0f2ac6a7e991d956eb356bba6920690612460bf5f414a07b705c57e;
    uint256[] internal _sig17 =
        [_signature17, _partialG17I1, _partialG17I2, _partialG17I3, _partialG17I4, _partialG17I5];

    // msg: "6ddcbff04cf4d7733db9b763d93c9c39c11097ae2121800d0d1f2c94d531c8bf0000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG18I1 = 0x8215a95441b59cd0d00c07c12d0cac71b23c476e179baa2201ed09a211b510a7;
    uint256 internal _partialG18I2 = 0xa262bb988def489130bb6de5128af7954dc74f1038f5f10275723f77f162baf5;
    uint256 internal _partialG18I3 = 0x1c45c1abbe6f84aa4565835be5492be42a037d4a3fa19f913d02501e01d6d1c4;
    uint256 internal _partialG18I4 = 0x2ca3f4b59c447939eb5c8aba6cc45d9f233b15f6009b40cc59287824cfa40b41;
    uint256 internal _partialG18I5 = 0x264f8788cc6b06112a91e0a4cbb4f93743ed6ca652f69f447142ac7983707c3d;
    uint256 internal _signature18 = 0xa6ede719438175b33ebe9919202cced3af6c5e22039ea84b4abbcb87e319ffc9;
    uint256[] internal _sig18 =
        [_signature18, _partialG18I1, _partialG18I2, _partialG18I3, _partialG18I4, _partialG18I5];

    // msg: "9df2e3aabb7acfb198c4e3852c5899068d63ae9a4f30ff0cccc124b207e546630000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG19I1 = 0x8dd929b0ae4a16388755c9a4a6663566fc959adfdbf07667527cc650f219951d;
    uint256 internal _partialG19I2 = 0x02af0add6c532f091353320c32ec96fa735b50f7cadd4151c654af44011daa96;
    uint256 internal _partialG19I3 = 0x10d82fecb683d32acbfb753f26ac136ed523a9b84e43a14428c5b8ab2b1f11ab;
    uint256 internal _partialG19I4 = 0x17af7b5f4fec5e319c5271430696c365e144216f733e8d0c28582838ba7a0bfa;
    uint256 internal _partialG19I5 = 0x961d8e5b5cb6f3d60131a672078e9c8e28e9c8f24c2dddbe6e098817d39a826d;
    uint256 internal _signature19 = 0xab8c2682dfc0ca2a4539617d2b85f39b487f508100c9582ce924914799c991f4;
    uint256[] internal _sig19 =
        [_signature19, _partialG19I1, _partialG19I2, _partialG19I3, _partialG19I4, _partialG19I5];
    uint256[][] internal _sig = [
        _sig1,
        _sig2,
        _sig3,
        _sig4,
        _sig5,
        _sig6,
        _sig7,
        _sig8,
        _sig9,
        _sig10,
        _sig11,
        _sig12,
        _sig13,
        _sig14,
        _sig15,
        _sig16,
        _sig17,
        _sig18,
        _sig19
    ];

    // Node DKG Communication Public Keys
    bytes internal _dkgPubkey1 =
        hex"03972476103d7f22478847415690b3820b6b9b967373ff47401e567a960867850471e7f19de23c72da9b7ee4347a79446e16fbd1e9f8423633f19cefe849af852287527509cd04a386486abbfca0259ccb6475d38258653adda62b2d63f85323217e6bea0d34fd20001f1d94d2cc7fd1bfb0b068cc6ec580264c641aab262aec";
    bytes internal _dkgPubkey2 =
        hex"060aaddcf708eec0cf0cf9beca74be474aec8a301bd1d450e741072a930834a500cc52b5ff51c92c6bdb532194868a7f8603a4bced51b9f998476e730a9278d4091cd7e0efad5211e6f99505f9024c78a247b561263e211ba20446da4c08a6042a77c83d6d9928ddf2cf655c85d3bd8f8d58c0465a34b85410de5b6a9017e3b5";
    bytes internal _dkgPubkey3 =
        hex"13bcb25821e4b92af197cbedc54a602cc3da980823562760dff0ce5b26055708187e233e156278b658842b3300c61ba73c31e3c501cd19173a2239057165f8882bf771a3b071cc7a6950009bbb6b37c5fc81207e72b0f1a71d6b07ffcbc0e6cc1e29d4b4543ceaa24534cfda714bb12bd8f788848ed377064c6cbf3ca7a16e3f";
    bytes internal _dkgPubkey4 =
        hex"22e12f5c34692f7559377d7ece5659e31dc6f1b591b3edf186f0e2319358aa8a068646547efb0d76a8c7482444a1a4c61bedc56fcec3acf47fac799934921823163f489095579c7d26beb915f92bdbf808ca3f94fa9c2a0632701f94bc79e9eb0cf5d2a979b3e94fed83d02398573e5af5abb693b5eace143b6e044df1435d0e";
    bytes internal _dkgPubkey5 =
        hex"222451063a1a09012717d1d7aa27eb5900f0cab24f3b86ecf4005edb08d8b7c601b14f5f4519bc3aa8662868996986104b0d17d720e69a0f5bc7917811a5009c17b1834fe80db270a22805e126801e328828c98ebaa078a58daec8cba75dc6b31980e4303c2d67976fa6423e4ec6f2acad16cf51c07c7098f5d0dfacc3f8a016";
    bytes internal _dkgPubkey6 =
        hex"1b5969815bda9a3a6a07b9bd09541853c7c06b4046d925790775ae85807a47e507e3e19a949b1cc73f64649358a95a432e618c0299d8cdbf704b5c72c6e67c3302d610e17acef4f1b1fcf11409095b23db4e012e9d9bed5401296e1872820fa12f9a411f31a7e8e4b5ddb495f3a211b0c44760a7e389427db6467dea5378c83a";
    bytes internal _dkgPubkey7 =
        hex"2a2b0eeb21e345723d39f0576a14cba0c2dfe06c3d75cfc17437bfbc5858dd3f1454cfc0877f29b20a5781f9a98857e50e8700ba6428685c36e17f321b8963d42002513d18d510a22cfee59385418d580589b83540ef9b0e8201888c666b517c20718d1ad3a85cf313b8eab6620c0e560764b215f7b967a8a20d6ccdcf2bb2d2";
    bytes internal _dkgPubkey8 =
        hex"28ae5c88d9b18ed4af86fcff85ff9cc2b60e4f1ccdf8dbabd9f4e2a03ad8847c0858628f0e88b66a1a091b20c4c110a806cff4a7c3cf125bc91312ed1e1b367227d47f9473bc1465960c163a7a320c26934eb00e3024a1caa290976547f131ab194290e2950ef6ec1ce07793fe759ffa3750ef17f77ec7850c79f9dad130a96a";
    bytes internal _dkgPubkey9 =
        hex"151727e94c3cafd40ca38420d0517c3b73b778f6d20a7a2cce8573b676bf37b62973e1eaa1a77224da262fa883f86423542623088015e5630a3e4d49e0f6eff70a5323707557d41b0840808f0a7dbb546cba9b95f2449cf5ca075585ef738cf42f4dd51e48f08a6375a6e663cf7b3d28d613ffc1d6c52d62224171f05060add4";
    bytes internal _dkgPubkey10 =
        hex"226105c8a5e04c372089a4e5df221a3f50c4608ed5aa2e6ad624f8cb6558a5cf241888a90c01ee4afde82a7697d335abb34cdbf9048beb1c2587ed295b9ca86a2027d7317ed541a90ef20af458f5941760dc2644d24c7dcc2551e229be8342092fa3ce1b1ebaf16796bec6377ff8c161ad5d96d42c8115368d2f691278c4e0eb";
    bytes internal _dkgPubkey11 =
        hex"0a136df5e9169f08b5207392a27b2410115e26dbf83a8d4033fb1cef6b5e1f1c16dce818ffd3b85e9d586f945369770c683a78bc138fea50b1c01a3d9c5be5181cc30078accd999e7a479ae3cb73154f7af41f923c1e3efa093711ee7389c21d1878700264e4d7b1914204def9e35f76cdfec0c8d0c43c1abce317b5c51c306f";
    bytes internal _dkgPubkey12 =
        hex"02769fcd16915bb205af814a7520996c1200ce9dc01d1a15aa87fc34e5f30bd80371873ee754e8a9ef1242de811756088e55daba7263ade0daa25e00d2381ac22eabb44e702f14e18b0e899ea576715b6e0335f10696f3c1ef37bf35cda721090900d027f0e293a5da5a3fa147fa725bb2b1fa639112808bdfdcc516cffa580d";
    bytes internal _dkgPubkey13 =
        hex"1084037cf652c6b899a802d89ba3d956f6db21df9600afb8130cdff87e3114aa1ec505018e95ee4079175390277f85dadfd4cb14684c7ccf70aaa5610243f0dc194fd9a91973df4b723f30f6745bd5fc6715c6b8b967485a8f56c382d8d587061cb231f5c0264b1da20ca62f27ceaf073520a14684a7116ac052e9a165312329";
    bytes internal _dkgPubkey14 =
        hex"0e37da2d14dfed950ef4c957622fc404907dd4727c2dc8aa90d0a1ae7460cecc2e98bc51ce7e9ce78f99b29b4522ba9d5739673c139d5f42df5bb29fcf900d1e19df80ff457a30c8b1cc2ddd315fb0d90b3252c83c7bdd6598f92d5fb61dede10b3a12f5abdc24733561ad7840ed9bf630af66c782053fbba2fcecca3df6156c";
    bytes internal _dkgPubkey15 =
        hex"074472599286dc642c53293025b5d3527421d3fd223aa03c8547cf532abbb28b1c3f6871d541068621b021693fd82f8e11fb5234a566162d223f1c802b0ef46b23ad911bb77905deddd950d6659ef30f9ff7e526c4b115af784c424c996152bc0a6c2ffb43789a6162704af6efd1073e5db1b26b0e0c075bcdef1d6720d112d2";
    bytes internal _dkgPubkey16 =
        hex"2daa2bfd2aa5a52bf87fc9881b8853af1dff3ef078e57cdc008e139e9c9a6d0808b9a3893d0c6203525b5b6c061e03c15985e3b98b12112e138f2e5c019c94a506523cd4246eadd1c0f4ccec4ce6510d16af7a5ca81a0efef909ed157ca1f28e0e1c936caef01c77e2dd4e2fae2deb4f7667e403bd6bbf26fc22a51e534a1362";
    bytes internal _dkgPubkey17 =
        hex"0c83420b3850f3a84cb89e6fb809b1cce466e565a57a9ca92caaec53c818fd161ca7218f79bff8a3f540533400b741e59dd805191056e006b49c7380ab7f20d11505619811f4104dd8f719783bdd17d243d868f657a34ef85d25f95ea147b30822fa86172140d0a448297763ae6d1762a6ff1ffb17ede6a7cfd1ee85849521d4";
    bytes internal _dkgPubkey18 =
        hex"26f2d00ff3506f30c845e1394a50f0f1c13382cfc96a3247f5947da2646f70a8074b387f2fdbb22cacb665897a96583da6451ccf7c2ee9e5e4cfc722d361fd4b0071b8cbcbb33b784b960af42cb85204a348f0c7439e4b528f0293a1bbb6e7782f1357f7f6d195f81de7af348de72dc3cd46b25375fef546d13ced6c269b904a";
    bytes internal _dkgPubkey19 =
        hex"24877c190e3cf4d6b28438de1073fc21f3fbb01185890fbb765dfd89134cee7829462ca92db9281fda2fe8c286e4a392fb6cea3ee7cc0afd3ea7bba2f6e52e272fdff2fb446c3262bc0c948addfe5246625d8302fed8faafd0eff2d41bb87c46074dd7e30f90d28ab6eea8dcda4acf99635e8e04a86a84a73673045fe5a6bff4";
    bytes internal _dkgPubkey20 =
        hex"13525b51832fbd15d28710625b0f068ea54b83b06550068bb30ebf09d0d3dede17522390ca06c052a073472554ae9b5ffe3ebc827f0b9e101d77991466c1c4ff153c63bed03a1701c9c498409313d55114f3e1c4250e0993a629e38207cf4628128c51a2adb70132c9fff6f915c426c83dd7f0e903a27b81b09aed22e32a7254";
    bytes internal _dkgPubkey21 =
        hex"0d7bd41999330dc5de39e3c7968109248ee0cb3aaef9886edf35b2d1795c4628264e0889d705a09a48f7ef7c977ab65c06047beb8eaf92f9d1d659f048aa7ee80656022f43ad8b8699c1a24e8524eee3b58e770fd228410ba8740c9a59f620bb18ab7766de1d8948ad3884c587e8a5db74cb8d855f72e839655f0bd4be3b3269";
    bytes internal _dkgPubkey22 =
        hex"10a8926f4fc5c9fb47fca629f54788b7e35e8b63ae075df03c12384ad95ffbbf00f423cac9f5cb86120cdc6751c9b919f7453a122e6d35f6df4d71004a0f3fb307a6a0dd22b97eea9c52c9ebc35e782e1e8e81793b356e86877b7dd6831cdb352db62a3c87b1f3098a03e0997ced6ff3a56fa20e714a085716964aa38eac5e77";
    bytes internal _dkgPubkey23 =
        hex"11d7e79762dd465cb6a14136ef2cfd640439eaa14d491f475ff63e20e6af2eab1e20cd5a19e935f347a242cca07a8cb19829d6a25ee6870710f6b4a6a263446d2b9e1b624618d8823e5805546ebbde824b027d2c71834f3f6c318e2acce955132a010223ac67791521a7b201a85f5089163cc00a50861f8b6c6d4c5858b3dd23";
    bytes internal _dkgPubkey24 =
        hex"29b9cbb2bf062d92f9c8890f0b632417d9c4647de626a9db0024fa8ddbca62c82e29b689528668fb5977141c1cea3e6a7e5f01003ef5b7fbfe0d47800252e70e2f9c0645f7ded1a7d3f559e96e171777be887288eebdee7c1f00f108265f9e7322393582da26e44ba3b341a0777509679087fb1566a0b1718bee535740d6705c";
    bytes internal _dkgPubkey25 =
        hex"23ec89e7530e59b1cff12a68fd3810111f7f5d26fd761b9ede362319c09b4fe82dbb1d5d5189f9c4bef5e609d8d611138c8403741697fe8c8ab27e3f86e556d90c507175aa253f40a6fa5524970dd6045db2e5b8a33ab94e236e8d785be0498811f8697477fa498f89cdaac500a8efd58e19d8f305ed74e795b4103d389c4667";
    bytes internal _dkgPubkey26 =
        hex"013f9896d09307269853552158e678bcbabe2bf73a8d6de440814c2bfd004f7725588038570d6bd5f36b11d6bb30ef6afa127736074aead142366681aed0faf3119933b646d6b9a4eace32dde2eec7c99566175451fc7c2fd7969f83352ee77a289cfeb74d77382ce147b1a85c059f2c14cc120f1e36000a9d3f9def18f686e7";
    bytes internal _dkgPubkey27 =
        hex"0e62efae973011fefd625ec0a30b0b21a60e51a6e0fc096d8d0decd96eb41900214c5417c16ee8dd755c5015ee76765f9ddcc786737a7eafc9e71b7464341d253020aba78aff9ec00d5f637541b4a16ef7544faa96d1b9b12bdb941ee57ca5711904981f11f26106edaa28b71b0dd86f6f76b59d4f8d6e87465c43940ef82f4f";
    bytes internal _dkgPubkey28 =
        hex"30616a7c8793d2be94fc779a6fe53fc557eb35cdb7a2728bb6bb742a33fcb3da0b3479542894f9041b67a592e402c09304889ff4bbc91f6551c0574676092de2179aacb88e37db438c78ce251f045301d8257f51e1afcc1c7801ba0db2bbeba10f2e5d1f9f7959a852d723dc8366a5515cc4d6050b65e777aeb8eb687846bbbc";
    bytes internal _dkgPubkey29 =
        hex"05815f165ad9b553ac646dc32d30b6ac091f363627271a0f930e621151a29e7f02b30a5e851f29b7f69cca0c5854e1fcd33d46495e6650397cc3b3f17f8fe73f129228283d5ec9140536ef185a378601b6eaac946432f3168a184a49c6e1d8ff204ca31dee68fb4c9daae9bffd0140f945b41645d0af31eeda233ef953937fdd";
    bytes internal _dkgPubkey30 =
        hex"022221e5b08ce50d1ca6ab0a48a505b5a3bdde0df728046cab70291237579ac8296210e1acf702e24862999f209c11754a73066b228cf1d557e6336606e88b281b1840b659125bf92932447ba8e761fabad95162230bc46fc1f79c173501039000b4ac4cbf623af0a10943ba328d6dcaae3a36a32a6f7eaee41cec1405fcb6d7";
    bytes internal _dkgPubkey31 =
        hex"06a8e68091b66c6e6213fdab7adcaa1c7ef2145aecbad0518de6398ea84175180f55f1c03517e174a95b4317ffee31c4f99bb64b6d0ea179ca7f8e86a54574780678fb7b25f7aa9ef3b7e4108834a5f5517f0d6ce295efb363323e45c434a6162a730a0a82bf1d78eb6ac49c2841f77ed71fc14e5df27625dbeb97de7b78628a";

    function _fulfillRequest(address sender, bytes32 requestId, uint256 sigIndex) internal {
        IAdapter.RequestDetail memory rd = AdapterForTest(address(_adapter)).getPendingRequest(requestId);

        // mock confirmation times and SIGNATURE_TASK_EXCLUSIVE_WINDOW = 10;
        vm.roll(block.number + rd.requestConfirmations + 10);

        // mock fulfillRandomness directly
        IAdapter.PartialSignature[] memory partialSignatures = new IAdapter.PartialSignature[](3);
        partialSignatures[0] = IAdapter.PartialSignature(0, _sig[sigIndex][1]);
        partialSignatures[1] = IAdapter.PartialSignature(1, _sig[sigIndex][2]);
        partialSignatures[2] = IAdapter.PartialSignature(2, _sig[sigIndex][3]);

        vm.prank(sender);
        IAdapter(address(_adapter)).fulfillRandomness(
            0, // fixed group 0
            requestId,
            _sig[sigIndex][0],
            rd,
            partialSignatures
        );
    }

    function _prepareSubscription(address sender, address consumer, uint256 balance) internal returns (uint64) {
        vm.prank(sender);
        uint64 subId = IAdapter(address(_adapter)).createSubscription();
        vm.deal(sender, balance + 1e18);
        vm.prank(sender);
        IAdapter(address(_adapter)).fundSubscription{value: balance}(subId);
        vm.prank(sender);
        IAdapter(address(_adapter)).addConsumer(subId, consumer);
        return subId;
    }

    function _getBalance(uint64 subId) internal view returns (uint256, uint256) {
        (,, uint256 balance, uint256 inflightCost,,,,,) = IAdapter(address(_adapter)).getSubscription(subId);
        return (balance, inflightCost);
    }

    function _prepareStakingContract(address sender, address arpaAddress, address[] memory operators) internal {
        Staking.PoolConstructorParams memory params = Staking.PoolConstructorParams(
            IERC20(arpaAddress),
            _initialMaxPoolSize,
            _initialMaxCommunityStakeAmount,
            _minCommunityStakeAmount,
            _operatorStakeAmount,
            _minInitialOperatorCount,
            _minRewardDuration,
            _delegationRateDenominator,
            _unstakeFreezingDuration
        );
        vm.prank(sender);
        _staking = new Staking(params);

        // add operators
        vm.prank(sender);
        _staking.addOperators(operators);

        // start the _staking pool
        deal(address(_arpa), sender, _rewardAmount);
        vm.prank(sender);
        _arpa.approve(address(_staking), _rewardAmount);
        vm.prank(sender);
        _staking.start(_rewardAmount, 30 days);

        // let a user stake to accumulate some rewards
        _stake(sender);

        for (uint256 i = 0; i < operators.length; i++) {
            _stake(operators[i]);
        }

        // warp to 10 days to earn some delegation rewards for nodes
        vm.warp(10 days);
    }

    function _stake(address sender) internal {
        deal(address(_arpa), sender, _operatorStakeAmount);
        vm.prank(sender);
        _arpa.approve(address(_staking), _operatorStakeAmount);
        vm.prank(sender);
        _staking.stake(_operatorStakeAmount);
    }

    function prepareAnAvailableGroupByKeys(
        address[] memory nodes,
        bytes[] memory dkgPartialPubKeys,
        bytes[] memory pubKeys,
        bytes memory groupPublicKey,
        uint256 groupIndex
    ) internal {
        uint256 groupEpoch = 3;
        address[] memory disqualifiedNodes = new address[](0);
        IController.CommitDkgParams memory params;

        for (uint256 i = 0; i < nodes.length; i++) {
            vm.deal(nodes[i], 1 * 10 ** 18);
            vm.prank(nodes[i]);
            _controller.nodeRegister(pubKeys[i]);
        }

        for (uint256 i = 0; i < nodes.length; i++) {
            params = IController.CommitDkgParams(
                groupIndex, groupEpoch, groupPublicKey, dkgPartialPubKeys[i], disqualifiedNodes
            );
            vm.prank(nodes[i]);
            _controller.commitDkg(params);
        }
    }

    function prepareAnAvailableGroup() public returns (uint256 threshold, uint256 size) {
        threshold = 3;
        size = 5;

        address[] memory _nodesGroup1 = new address[](5);
        _nodesGroup1[0] = _node1;
        _nodesGroup1[1] = _node2;
        _nodesGroup1[2] = _node3;
        _nodesGroup1[3] = _node4;
        _nodesGroup1[4] = _node5;

        bytes[] memory _dkgPartialPubKeysGroup1 = new bytes[](5);
        _dkgPartialPubKeysGroup1[0] = _partialPublicKey1;
        _dkgPartialPubKeysGroup1[1] = _partialPublicKey2;
        _dkgPartialPubKeysGroup1[2] = _partialPublicKey3;
        _dkgPartialPubKeysGroup1[3] = _partialPublicKey4;
        _dkgPartialPubKeysGroup1[4] = _partialPublicKey5;

        bytes[] memory _publicKeys1 = new bytes[](5);
        _publicKeys1[0] = _dkgPubkey1;
        _publicKeys1[1] = _dkgPubkey2;
        _publicKeys1[2] = _dkgPubkey3;
        _publicKeys1[3] = _dkgPubkey4;
        _publicKeys1[4] = _dkgPubkey5;

        prepareAnAvailableGroupByKeys(_nodesGroup1, _dkgPartialPubKeysGroup1, _publicKeys1, _publicKey, 0);
    }

    function printGroupInfo(uint256 groupIndex) public {
        IController.Group memory g = _controller.getGroup(groupIndex);

        uint256 groupCount = _controller.getGroupCount();
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
                    string.concat(
                        "g.members[", Strings.toString(i), "].internal _partialPublicKey[", Strings.toString(j), "]"
                    ),
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
        address coordinatorAddress = _controller.getCoordinator(groupIndex);
        emit log_named_address("\nCoordinator", coordinatorAddress);
    }

    function printNodeInfo(address nodeAddress) public {
        // print node address
        emit log("\n");
        emit log("----------------------------------------");
        emit log_named_address("printing info for node", nodeAddress);
        emit log("----------------------------------------");

        IController.Node memory node = _controller.getNode(nodeAddress);

        emit log_named_address("n.idAddress", node.idAddress);
        emit log_named_bytes("n.dkgPublicKey", node.dkgPublicKey);
        emit log_named_string("n.state", _toText(node.state));
        emit log_named_uint("n.pendingUntilBlock", node.pendingUntilBlock);
    }

    function printMemberInfo(uint256 groupIndex, uint256 memberIndex) public {
        emit log(
            string.concat(
                "\nGroupIndex: ", Strings.toString(groupIndex), " MemberIndex: ", Strings.toString(memberIndex), ":"
            )
        );

        IController.Member memory m = _controller.getMember(groupIndex, memberIndex);

        // emit log_named_uint("m.index", m.index);
        emit log_named_address("m.nodeIdAddress", m.nodeIdAddress);
        for (uint256 i = 0; i < m.partialPublicKey.length; i++) {
            emit log_named_uint(
                string.concat("m.internal _partialPublicKey[", Strings.toString(i), "]"), m.partialPublicKey[i]
            );
        }
    }

    function _toUInt256(bool x) internal pure returns (uint256 r) {
        // solhint-disable-next-line no-inline-assembly
        assembly {
            r := x
        }
    }

    function _toBool(uint256 x) internal pure returns (string memory) {
        // x == 0 ? r = "False" : "True";
        if (x == 0) {
            return "False";
        }
        return "True";
    }

    function _toText(bool x) internal pure returns (string memory r) {
        uint256 inUint = _toUInt256(x);
        string memory inString = _toBool(inUint);
        r = inString;
    }

    function checkIsStrictlyMajorityConsensusReached(uint256 groupIndex) public view returns (bool) {
        IController.Group memory g = _controller.getGroup(groupIndex);
        return g.isStrictlyMajorityConsensusReached;
    }

    function nodeInGroup(address nodeIdAddress, uint256 groupIndex) public view returns (bool) {
        bool nodeFound = false;
        for (uint256 i = 0; i < _controller.getGroup(groupIndex).members.length; i++) {
            if (nodeIdAddress == _controller.getGroup(0).members[i].nodeIdAddress) {
                nodeFound = true;
            }
        }
        return nodeFound;
    }
}
