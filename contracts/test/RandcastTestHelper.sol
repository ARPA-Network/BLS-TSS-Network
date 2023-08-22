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

    // Node Partial Public Keys
    bytes internal _badKey =
        hex"111111550230862eccaa516975047156d5c7cdc299f5fce0aaf04e9246c1ab2122f8c83061984377026e4769de7cc228004221275241ee6a33622043a3c730fc183f7bff0be8b3e21d9d56bc5ed2566ce3193c9df3396bd8cdc457e7c57ecbc010092c9cf423391bff81f73b1b33ac475dbf2b941b23acc7aa26324a57e5951b";

    bytes internal _partialPublicKey1 =
        hex"0151d3c9a72e971cb49a4761bf6b06b23514060b307ea25bbe346d296a1afd9825c2a21b629fde36ff32aa301c5e5e2694268ab39a51c6bb52dcf3c67c2a25961c550e998ca25d10df405636f30220f2e742759091c703886c5398ad91b336ac2635260458dbba229add329ca3d66d9b7f0cbfbf783c56df734e10436505115b";
    bytes internal _partialPublicKey2 =
        hex"0d7e76a7d6940e768baeb96e86921ad0d657800e2244f4d53360d0ed361e0d5b251bc6ded98940562f87394c72258f3934cdb81177da2337f17f11290844ee2b22f72d02afe7956b75d6868273e16550a5c11d9fd54e2086a0b639e18038fd4b051d2dc1377db5cc736924975295d2c8f929d4709a44424d97c9520298dba68f";
    bytes internal _partialPublicKey3 =
        hex"0aa92036050dc1b98b2b92c85426d46d207adb09b27962ae91d7d6e677ec4bc521a88b3ad4cfbaaa4330ae335bfd4dddaf83b2d1f71cf5fb8ed305c99f88debb115a5330a09bd91073b0a3d5f55ac7e74373a685d1b5b4095e15841c1cb943b8224e4acf54b7a5997fdcefd4c6dd93ca82431ba19814fd70a99b76811ca86eb5";
    bytes internal _partialPublicKey4 =
        hex"209266e4e3b1ff01e71b6cda42059649dbd040cbc2c01828e75aa7e270b7306c1779b4007dcbdf2285bbfe475dea1d874a3f3202d5d98e06b4e0fc476797c5c3214ec855c857c9b1b096b41a428ee60818c17c7bd5e17ef5f69e738eb01d4dcb249305183af64e4aeb681a343a7ed4881cf3f1084579a89649bbbe6a14776546";
    bytes internal _partialPublicKey5 =
        hex"0db57ddba86968674167d174546177acd67686142c0ec42f04f399b18375c2422e679ecad2cff16dbf47713c6926a91f30dbf697c0e735b60162880d9870e82f1eb0253b757ea1346ecb85f17b190e96464db415d976889e826b5a37b7a2c54b20353815fa7f5947be653669ed5eccfa550405137af3cee702082ff1b715b401";
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

    // Group Public Key
    bytes internal _publicKey =
        hex"2627cd94cb34ce93b1b308fc2ab9de386b2b5eea81ee14c710f67973ad296fc210dc93d5cbb78b02a195e4e544f091d092bd2a9907ac64a97967be5bdf510e4b12920601981bddf40e0b34d6e3527927c31de543e753ec26f29a12e20998510e22980a24b9da9f2c45ca0cc6e297dbbbc2e71d289b87e62cc658e9585c5714c4";

    // Partial Signatures
    // msg: "381336e809b17f1073ab04bc3ec7f42467eecd3deb2cf2131afc144f3d7e34600000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG1I1 = 0xabbaa4219444d1323279ad58e21d9d7707bf7838fc79794316af551ca223d65c;
    uint256 internal _partialG1I2 = 0x91b3735ba10f8eb8bac31f489d93d23e61df870a0aebf91510e7f60fc1bf51c6;
    uint256 internal _partialG1I3 = 0xb063703bf38d44353bd2b5a86f092a15831b6be171d9bf10b372fd8158122951;
    uint256 internal _partialG1I4 = 0xa9b3353ca044155b4ccf0f9c7628e45efcb79dca40ff78584bd3a8fc1aa73655;
    uint256 internal _partialG1I5 = 0xae69f42760224d5099e87b298ef83fb40fc5fcbb3b62938589c86879f7b3c367;
    uint256 internal _signature1 = 0x26ab307c6d2ade219ee35f1142d0b5d8d36700b7d0242a40200cc16f5d141a4b;
    uint256[] internal _sig1 = [_signature1, _partialG1I1, _partialG1I2, _partialG1I3, _partialG1I4, _partialG1I5];

    // msg: "b21e7b73da061425eaf25194a5948b8eb7de4b48bbb2d2e9288ae5ee6fca7f980000000000000000000000000000000000000000000000000000000000000012"
    uint256 internal _partialG2I1 = 0x871c04fb101bb19e84bb0ef90576413926b97305b8e85ab82c1c84a88364a392;
    uint256 internal _partialG2I2 = 0x07ebc303fb33ac4d3b15de9437c477128594d9c42f96309b7da2e87e08a9fd35;
    uint256 internal _partialG2I3 = 0x0b7cf43352059f00968182c90ce688dcdb279aa892995c555fef0ae79ad2f6c8;
    uint256 internal _partialG2I4 = 0x9b22484f21384f141a3a7db29acbfa925910e3fed493eea261806cbddd329528;
    uint256 internal _partialG2I5 = 0x13c3b1d88c37fa7b44b0bbd4d3702bbe91dd55e29295a44e6d6261746866100b;
    uint256 internal _signature2 = 0x96d6eed5de41a4cab0c45ba13dc6d6f63e5a069bdb3e23a94811db492fd3132b;
    uint256[] internal _sig2 = [_signature2, _partialG2I1, _partialG2I2, _partialG2I3, _partialG2I4, _partialG2I5];

    // msg: "b6341502d010740f87b23ef2726c0e20884c3f32013ba6905b22597c561182420000000000000000000000000000000000000000000000000000000000000023"
    uint256 internal _partialG3I1 = 0x92dc0cf4e9b4a298e202765c48661ad86f79cefd962d8f9b19524fda6b5bb1f7;
    uint256 internal _partialG3I2 = 0x8bea8c68e3e149e9e591884d5b5f6ae13df7d840ed1994a1ea8964053c658f72;
    uint256 internal _partialG3I3 = 0x9f951ba684c453c5720d9484ad5fb968c030e3c49e56047bc3ad9a3a4a85a7ec;
    uint256 internal _partialG3I4 = 0xaa12f0c688cf27bd279cf352d58bef51acd1b8889902cfca9e95bf21e30e4ce8;
    uint256 internal _partialG3I5 = 0x8137c4752da869485e8f65028478930da499f805bdf37e6e981d44430edaf306;
    uint256 internal _signature3 = 0x83061330502402c70ed2b9566f644a17904df2d5fcbc5a7f47dab1375959e241;
    uint256[] internal _sig3 = [_signature3, _partialG3I1, _partialG3I2, _partialG3I3, _partialG3I4, _partialG3I5];

    // msg: "38dde2f18019cfd2160a9efabe48fed1bf6c3224f055da89b50e2ca71c4bbc9a0000000000000000000000000000000000000000000000000000000000000034"
    uint256 internal _partialG4I1 = 0x1ae63f629574794a621929cda7ad8a011640cd57ed7eace7bd6fa344f49eac58;
    uint256 internal _partialG4I2 = 0x889efeffaa0ed1d2c5259e53d5afe9c8dc018dca97e01f019de0db77963f6663;
    uint256 internal _partialG4I3 = 0xac2b9a599c9615a925f76f5a45d160cec2ca65b94f21e5b2d5edca75400aabfb;
    uint256 internal _partialG4I4 = 0x8e164d6a6a4a2710e4f7e361d94ea49d960b8de01d5170deab6aadc497327bd6;
    uint256 internal _partialG4I5 = 0x0cd9d4ec00b9edfa9c058775136e549cc0c191ecae3e0321467163a78608fab5;
    uint256 internal _signature4 = 0x898966133d692d5628b729dba1ef598fdbc17ef16889d153369970d328bc33f5;
    uint256[] internal _sig4 = [_signature4, _partialG4I1, _partialG4I2, _partialG4I3, _partialG4I4, _partialG4I5];

    // msg: "51f4382ec948c5ba7e06815ad3a20a45404f6035b602f5dfa39915765393d1570000000000000000000000000000000000000000000000000000000000000045"
    uint256 internal _partialG5I1 = 0x9f014779f598ac820722931661a723426e103a5436d90b796801edbd31d286cc;
    uint256 internal _partialG5I2 = 0xac1806f26720efbca46d7bcc61f567e399223c5391f63796fc63f67d43ca0afe;
    uint256 internal _partialG5I3 = 0xafcb905bc1183f303971c3471ae9784405f01a2cfa18f637e73e55829e822e7d;
    uint256 internal _partialG5I4 = 0xa62d91bde2b6088fa45cc3f28e87d94b056894e33552138f0b84632b96a038ad;
    uint256 internal _partialG5I5 = 0x1a0bad90a84b3a49ae9ca18ac66fa9b9c030b699f0413eb58e0f2236e56d6993;
    uint256 internal _signature5 = 0x257a0e4f78006498ad0b6306762dba792326e241d2d902a882a41c61027e18f5;
    uint256[] internal _sig5 = [_signature5, _partialG5I1, _partialG5I2, _partialG5I3, _partialG5I4, _partialG5I5];

    // msg: "fe4334e130fd43b7ad1f25d8c62cc345f9b9940c578ceb85421b58fb8d0a78dd0000000000000000000000000000000000000000000000000000000000000056"
    uint256 internal _partialG6I1 = 0x1ccc8855727ff70cd33b1932f3ba9d3a3b738fda3fda6365697f6925ac364a8c;
    uint256 internal _partialG6I2 = 0x84affc9ec80cf225ca71cbdfd18743f0eafeeb752435f07692c6148903eabf76;
    uint256 internal _partialG6I3 = 0x84527e389e3ff097b537b45d41142bc470195bbe5aee76c46ae12a000f06cbfc;
    uint256 internal _partialG6I4 = 0x23beb9de2922389defae02e413a0305c4e1ca237c4dde160b3d7264c42979eb4;
    uint256 internal _partialG6I5 = 0x2a9763319956b00a2f06a34815cc4f4a3ae1bd0ce2c1108a58204b820681e697;
    uint256 internal _signature6 = 0xab442cdaed922492513450172c1faf8dfaa26856135bc845d79753fc8d1ab4c3;
    uint256[] internal _sig6 = [_signature6, _partialG6I1, _partialG6I2, _partialG6I3, _partialG6I4, _partialG6I5];

    // msg: "9a30cc4ff58a5e8a12572ce431dc681f5bb03d9443816093dbd39d98b7a2264e0000000000000000000000000000000000000000000000000000000000000067"
    uint256 internal _partialG7I1 = 0x84f5e756dba97b9efea0eaef46835ad4df47b3b0487bd3c9363ebc7a2c94bd6b;
    uint256 internal _partialG7I2 = 0x89be684723f8db94375026ca09b6d586f43223c4e239f0652ffcbc12ca419b90;
    uint256 internal _partialG7I3 = 0x0e9316e95d8f2970f9896168693a1e3395342e7dc74ab13df871a2abc74372ac;
    uint256 internal _partialG7I4 = 0x0af7f4f25c4900ba23ffc891ee6b0e468370e49fb6a05857844f68b9b26cb8d5;
    uint256 internal _partialG7I5 = 0x2b4b6ad790f340b5adae11aeba6cee83804f8e32da1919d655e24872922cf542;
    uint256 internal _signature7 = 0x16c13efb7a163151b00a4a0968d127c24ecbbc2c15d8ae30f5b7b32aa0d7df60;
    uint256[] internal _sig7 = [_signature7, _partialG7I1, _partialG7I2, _partialG7I3, _partialG7I4, _partialG7I5];

    // msg: "adc74caff81ece16321a09d27b394c0f287db98851edb5219105e30a6b8b919f0000000000000000000000000000000000000000000000000000000000000078"
    uint256 internal _partialG8I1 = 0x9c496e94f26f93f40f82e7bc7b5d2e7b3c443cd08a7c93310d622a44f7466f64;
    uint256 internal _partialG8I2 = 0x993c2746fa8b88eaa69a35243dc97045588667eea4d921998b84c78237fb2a1a;
    uint256 internal _partialG8I3 = 0x162067885550566741fa9bcd2dd3dde1703fd4fc7851598b193616b6b5ddabac;
    uint256 internal _partialG8I4 = 0x2e463bd0d61ca26f5c3df1f540e365d066f4890147a8ea0a50ec7600d1bdf6a3;
    uint256 internal _partialG8I5 = 0x1f30b4c62c22d2cfaec08796fb7f32e8111c06565fdfe123f71d69cb3f0b86be;
    uint256 internal _signature8 = 0x834795279d6cb20e21b1266e3da48d8ec387de55b3826dc52d26db7fd61fbe6d;
    uint256[] internal _sig8 = [_signature8, _partialG8I1, _partialG8I2, _partialG8I3, _partialG8I4, _partialG8I5];

    // msg: "ce4b662b812b4166e4cf522b4322a7492c71222c2548db916c31de45470fce200000000000000000000000000000000000000000000000000000000000000089"
    uint256 internal _partialG9I1 = 0x1dd8e0f5d678fddd9ac39df2db462b4bfa1a72bec8f1c07c229358e407158797;
    uint256 internal _partialG9I2 = 0x2ea9db4db41e4d2572a322a43cd23735c3804e1f6478cb93a1fbd7a7074ecae1;
    uint256 internal _partialG9I3 = 0x0a8c7d79cff0049a663628344b73e3d820cc75e22fcf0a158629fecd95025b70;
    uint256 internal _partialG9I4 = 0x916bf89640bb0ed1fc6d2e7c92a23b59d414f126a5e8a822d393b3b5f6875f1a;
    uint256 internal _partialG9I5 = 0xaa19604168f96526a3a9ade8a4325340c4a0ab725a03a81f782f1369c21dfb19;
    uint256 internal _signature9 = 0x141c7eb7cfa11011d151e8ca1ad968496509a3595267130b0c5d2b446a807c1f;
    uint256[] internal _sig9 = [_signature9, _partialG9I1, _partialG9I2, _partialG9I3, _partialG9I4, _partialG9I5];

    // msg: "d58a2946ea20724bdf25841dcc9f40da9353578fe4c917982d9d414a9306225e000000000000000000000000000000000000000000000000000000000000009a"
    uint256 internal _partialG10I1 = 0x02f4e577cddbfb66c4a97db8f2076b0b1d22ba192948014d5ad2d7b8a0810c7d;
    uint256 internal _partialG10I2 = 0x998cb663e292137cefad5804f6e0d9bc6953e943f35d9518ac91913e4f8a1a4f;
    uint256 internal _partialG10I3 = 0x2883d70e3f7ca6132739c1b3fa793233a08d4482791d2744be5f7fa1c9806d3b;
    uint256 internal _partialG10I4 = 0xa4e482ae740cc2ca962245083a3ff093c2455ee23e488095d0dc8dba42a3743f;
    uint256 internal _partialG10I5 = 0x0d25cc716b2e206ecd06d2eec30aa3c010ab30b37a5de5f75b7ade6a601c031a;
    uint256 internal _signature10 = 0x9d443169c36708bf7095b1ab2780395a942c21d7c4cc4a55b5651989ae31b367;
    uint256[] internal _sig10 =
        [_signature10, _partialG10I1, _partialG10I2, _partialG10I3, _partialG10I4, _partialG10I5];

    // msg: "574d19ba255ab559ce104ba2f58150e9be94f7d40f7a482007b5953b8967629f0000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG11I1 = 0x8368c2c0f984d9b72a17473e25e70a92131f9019feded88c930e05ddc4e04254;
    uint256 internal _partialG11I2 = 0xa1024bbb403bc30a795f9b9ba27bea45eab0cfb892124998d249c4f0e0cdefd6;
    uint256 internal _partialG11I3 = 0x81fe71ad1e13c62ded5df3c80827c0571cd05bea5f31559d5bb745f1b83d7566;
    uint256 internal _partialG11I4 = 0x8f848a57ba5e3cad04373cb6cbe2b0a2a45a8c6ca8dc91b65bce189b2c34f8ef;
    uint256 internal _partialG11I5 = 0x879d1bc3752654b496a2756a0db8dc83372da8363ef0e3525001e0678d2144f9;
    uint256 internal _signature11 = 0x25f6b2dd5279f2bf4ecb0975fa2d528dd563bc80f9de84c971002f3ff4c4eeac;
    uint256[] internal _sig11 =
        [_signature11, _partialG11I1, _partialG11I2, _partialG11I3, _partialG11I4, _partialG11I5];

    // msg: "e90f06a218986b04c8fcabf05d647937bdb828dd443bd70cffde5d0b2dbaa24d0000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG12I1 = 0xa9730f7556588fef722dfc14dac24bdc830985e9744214eb0539249d2dd0b68f;
    uint256 internal _partialG12I2 = 0x1e45e22846a40564b7875307d22575a0bf7fea6a89d48e1d5aa9dcf10f3cd21b;
    uint256 internal _partialG12I3 = 0x0623971723619d78e444144ab9c8943928c2946d98041be71e17a533ae3c8100;
    uint256 internal _partialG12I4 = 0x0454aeeeb6fa97af7dc04ab28206ce37c0816d86b14bff20ff036f14c521e07c;
    uint256 internal _partialG12I5 = 0x900f42689931b6734e6537be929bc320e6bf066e6f73e4dcc7c72d19589741cf;
    uint256 internal _signature12 = 0x9b75c71286cf0e81060e48dfcdb344eb9200ff4801d0ec88c9b0cdbb2da85312;
    uint256[] internal _sig12 =
        [_signature12, _partialG12I1, _partialG12I2, _partialG12I3, _partialG12I4, _partialG12I5];

    // msg: "6bfff7ae4ac5e8c53e806d5377a559d40294301a8e5ae2fa462c176432d96c220000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG13I1 = 0xa8e23f0c9853851b2933a94503a36d5495fd2cab39a2770bc1e0ce059a86e793;
    uint256 internal _partialG13I2 = 0x1e7dc0b0c130c20a34b5f5b658d14fc3040e00d63bb0febd828812e3f5f31b50;
    uint256 internal _partialG13I3 = 0x11e45f4b345898b761720ce8497627e3479999441e22365fdc22b2fdfe03a007;
    uint256 internal _partialG13I4 = 0xa83ead1be41b175d9bdc5de4729c9b53d21a06fe355eab94c620c9fa74bbbe17;
    uint256 internal _partialG13I5 = 0x81893a71ff63084b01d11e976a9ae34bb2452d1466a8d628bad43dab244c1f24;
    uint256 internal _signature13 = 0x1967c949663d1986f86ce056e2dcaaad507f7c8b2783b5623bf6c04bf253ef8f;
    uint256[] internal _sig13 =
        [_signature13, _partialG13I1, _partialG13I2, _partialG13I3, _partialG13I4, _partialG13I5];

    // msg: "381336e809b17f1073ab04bc3ec7f42467eecd3deb2cf2131afc144f3d7e34600000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG14I1 = 0xabbaa4219444d1323279ad58e21d9d7707bf7838fc79794316af551ca223d65c;
    uint256 internal _partialG14I2 = 0x91b3735ba10f8eb8bac31f489d93d23e61df870a0aebf91510e7f60fc1bf51c6;
    uint256 internal _partialG14I3 = 0xb063703bf38d44353bd2b5a86f092a15831b6be171d9bf10b372fd8158122951;
    uint256 internal _partialG14I4 = 0xa9b3353ca044155b4ccf0f9c7628e45efcb79dca40ff78584bd3a8fc1aa73655;
    uint256 internal _partialG14I5 = 0xae69f42760224d5099e87b298ef83fb40fc5fcbb3b62938589c86879f7b3c367;
    uint256 internal _signature14 = 0x26ab307c6d2ade219ee35f1142d0b5d8d36700b7d0242a40200cc16f5d141a4b;
    uint256[] internal _sig14 =
        [_signature14, _partialG14I1, _partialG14I2, _partialG14I3, _partialG14I4, _partialG14I5];

    // msg: "574d19ba255ab559ce104ba2f58150e9be94f7d40f7a482007b5953b8967629f0000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG15I1 = 0x8368c2c0f984d9b72a17473e25e70a92131f9019feded88c930e05ddc4e04254;
    uint256 internal _partialG15I2 = 0xa1024bbb403bc30a795f9b9ba27bea45eab0cfb892124998d249c4f0e0cdefd6;
    uint256 internal _partialG15I3 = 0x81fe71ad1e13c62ded5df3c80827c0571cd05bea5f31559d5bb745f1b83d7566;
    uint256 internal _partialG15I4 = 0x8f848a57ba5e3cad04373cb6cbe2b0a2a45a8c6ca8dc91b65bce189b2c34f8ef;
    uint256 internal _partialG15I5 = 0x879d1bc3752654b496a2756a0db8dc83372da8363ef0e3525001e0678d2144f9;
    uint256 internal _signature15 = 0x25f6b2dd5279f2bf4ecb0975fa2d528dd563bc80f9de84c971002f3ff4c4eeac;
    uint256[] internal _sig15 =
        [_signature15, _partialG15I1, _partialG15I2, _partialG15I3, _partialG15I4, _partialG15I5];

    // msg: "e90f06a218986b04c8fcabf05d647937bdb828dd443bd70cffde5d0b2dbaa24d0000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG16I1 = 0xa9730f7556588fef722dfc14dac24bdc830985e9744214eb0539249d2dd0b68f;
    uint256 internal _partialG16I2 = 0x1e45e22846a40564b7875307d22575a0bf7fea6a89d48e1d5aa9dcf10f3cd21b;
    uint256 internal _partialG16I3 = 0x0623971723619d78e444144ab9c8943928c2946d98041be71e17a533ae3c8100;
    uint256 internal _partialG16I4 = 0x0454aeeeb6fa97af7dc04ab28206ce37c0816d86b14bff20ff036f14c521e07c;
    uint256 internal _partialG16I5 = 0x900f42689931b6734e6537be929bc320e6bf066e6f73e4dcc7c72d19589741cf;
    uint256 internal _signature16 = 0x9b75c71286cf0e81060e48dfcdb344eb9200ff4801d0ec88c9b0cdbb2da85312;
    uint256[] internal _sig16 =
        [_signature16, _partialG16I1, _partialG16I2, _partialG16I3, _partialG16I4, _partialG16I5];

    // msg: "612f9f32f1a2a5ffb5aa3ac2aa34e097bd2e8208940fa8feed3617b28b69a4950000000000000000000000000000000000000000000000000000000000000001"
    uint256 internal _partialG17I1 = 0x86e2cf9ef913c710530b83825b67870dcbaa98f31ecfd03da0336cd2a27418e2;
    uint256 internal _partialG17I2 = 0x2c2f8adad5194ecac35743b510d76d81a0aea879df2bc8cc5b1049bf134f5b5a;
    uint256 internal _partialG17I3 = 0x2aabfb2ff6588aa94f9a293aea3a137c42118bbd17ee6eeab50176f4461644b2;
    uint256 internal _partialG17I4 = 0x869abf2506942764c4076a391ba91f32b7aeccb261b0f3a8e5ba78aa3fbf44da;
    uint256 internal _partialG17I5 = 0x9f477f5a5c54b04d3e53c13e4c97fb15698c4deb64f4facbf84a48d8a458aa25;
    uint256 internal _signature17 = 0x242b48e5c4dbb390480641093db775c358d6627208cf9f77773ff1e637296a80;
    uint256[] internal _sig17 =
        [_signature17, _partialG17I1, _partialG17I2, _partialG17I3, _partialG17I4, _partialG17I5];
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
        _sig17
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
