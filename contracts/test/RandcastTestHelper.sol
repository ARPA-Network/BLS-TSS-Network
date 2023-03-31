// SPDX-License-Identifier: MIT
pragma solidity >=0.8.10;

import "forge-std/Test.sol";
import "../src/interfaces/IAdapter.sol";
import "../src/Controller.sol";
import "./MockArpaEthOracle.sol";
import "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
import "openzeppelin-contracts/contracts/utils/Strings.sol";

abstract contract RandcastTestHelper is Test {
    Controller controller;
    MockArpaEthOracle oracle;
    IERC20 arpa;

    address public admin = address(0xABCD);
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

    uint256 t = 3;
    uint256 n = 5;

    // Node Partial Public Keys
    bytes partialPublicKey1 =
        hex"25b985d7e013aa556364b7723541ece7e92d4cf461446204aa72e37aeef7f2582b10412b61190143a48dc65628a6566c7af39483a45dcca44742c53a5ef14e6f2875a8a2580d1888129c85235d8d1b5ef8087423ad9aeb93c7cc060a53dab24226b90fdda6385b283a995448b4089787186c930ee8085430802e3df3764b4740";
    bytes partialPublicKey2 =
        hex"1849985fba9099e6d8d1863a7c6c9c71483782b5d9a30d803bdec60f204b4fd42f3356e30b927030fd4d00d045fefa8eb93b29f254b8433f7ff64467f54b52d8217c5760bc64e70ab51f9bf8ee21e76daca13efcbfb9fbb35d5f405f974bfeee12dffc41ce6ec13f52cfd7ca911fb302ad5c64158b8079b626fd65b51b663a81";
    bytes partialPublicKey3 =
        hex"0e1f951f32aba23599d09a6523c861a41a3666b0a41f9cf86a6e6c360a26304d2cead9504e3b7d1adbc775f9a6b1301bfbcf0a5aa4372326c00f91af7863b53b05ca366f0dc79e9e34c8966d33a3d77ee107e6e7e9ad86738b4d7a278fe81919110f586e639a4ac0823bde5be65a531a37fa45b9584dd1fca1d1aa42efdeffd9";
    bytes partialPublicKey4 =
        hex"0b205d275db7e7254691803f189dd40e203f57f21f585b2f43924ba3189d8f8c0fccc061ce49df93900deadd2cdc4604807a8f4b99bccd4f0b14df4845473253166b778cee833e52268379d4c49a88729711f4e16a51fbc2c4086070c944baef212c93bda6b9ee0bf7419e6c0256569f33eecf345daffd01523ce9e6aa4fa43c";
    bytes partialPublicKey5 =
        hex"1ab24e550230862eccaa516975047156d5c7cdc299f5fce0aaf04e9246c1ab2122f8c83061984377026e4769de7cc228004221275241ee6a33622043a3c730fc183f7bff0be8b3e21d9d56bc5ed2566ce3193c9df3396bd8cdc457e7c57ecbc010092c9cf423391bff81f73b1b33ac475dbf2b941b23acc7aa26324a57e5951b";
    bytes badKey =
        hex"111111550230862eccaa516975047156d5c7cdc299f5fce0aaf04e9246c1ab2122f8c83061984377026e4769de7cc228004221275241ee6a33622043a3c730fc183f7bff0be8b3e21d9d56bc5ed2566ce3193c9df3396bd8cdc457e7c57ecbc010092c9cf423391bff81f73b1b33ac475dbf2b941b23acc7aa26324a57e5951b";

    // Group Public Key
    bytes publicKey =
        hex"0f9c97d632d73c59e8d32ff08c51d11375bfb72d63239b6d7a4a9dfdb3f918be1622dd347dc1da12c8be8cf589e76216fe4cbd2ad696eb2134aa46c3705b5b3b2e7077789272b985a85fbb012b448564a65a07b7c7bd68fbd7f67bb3032cd7731e4506b2e2566b36ce57871ebc2a931b9ba51f1172271ce34be32816118a95c3";

    // Partial Signatures
    // msg: "2bdda4dd3ed74cb9ed050c6b6144174040215e9b997cc13ef7430487090270440000000000000000000000000000000000000000000000000000000000000001";
    uint256 partial1_1 = 0x946b508624231c85e295a6af69a2f774c30912c24b68d03389bca6579ca4f7ff;
    uint256 partial1_2 = 0x0b1dff99b5e89e5d6cf8ee13f03530fb365bef81f210a93bec1bd7966f0e813c;
    uint256 partial1_3 = 0xa129d28a7acff9f841d36a7ea7449be909bd3a6df0db0d32038c407f33d7225e;
    uint256 partial1_4 = 0xad7d8338f0a9f4a545088ca631830a4850233e973c6af021e7f6aa33196b9ce7;
    uint256 partial1_5 = 0x2e783af9d4a26e543b269c5b1251588fc7428e406ad3110576dbbbc93df3de17;
    uint256 signature1 = 0x021428f7a0c463c9ec48228219dcd5bd6b03ecdc734ca331541aacf97afeddb1;
    uint256[] sig1 = [signature1, partial1_1, partial1_2, partial1_3, partial1_4, partial1_5];

    // msg: "adba1e0e95a816030c7ab73a0ac0d519e51bb207562b225c169ba4fca9708aba0000000000000000000000000000000000000000000000000000000000000012";

    uint256 partial2_1 = 0xa760fcc4bf51cf5a2060e997cda96c3f234ae116072804d293284c5eae33961e;
    uint256 partial2_2 = 0x9ff58b0ac4814713b0b410c7d1ebce225f56bb77b33660b77b1561a2216d838d;
    uint256 partial2_3 = 0x9f478ef2ff9ceb771e5dadc82d6342104aa7fd55dc5d201897123db3d8127b2b;
    uint256 partial2_4 = 0xa89a494479ddcf5c15d8cc7eaf24b82e779a6e4cce4f8731994ea636070d93d1;
    uint256 partial2_5 = 0x94dc5a968688f9ae05557061d3f1baf7767bf874d4df7ec8d9cc1adf27315b58;
    uint256 signature2 = 0x010fd071a6ef7d41302c2c3bc570b53d6a5ca9c3e9fa31a25b618a44c944e1f5;
    uint256[] sig2 = [signature2, partial2_1, partial2_2, partial2_3, partial2_4, partial2_5];

    // msg: "2aba4fa6a8d715baf9a8e4965940c548f9d7a715093459ab3746a5516a7b83170000000000000000000000000000000000000000000000000000000000000023";

    uint256 partial3_1 = 0x1e0da7d48c2b81ae036bd7ef6d421ad0b78daa558fb60db7bda43cf677934e4e;
    uint256 partial3_2 = 0x8dcf2f3eacfee6459f4dbd9d750b6528edb47e2f7e7d39a8158a43f2af5a4cf7;
    uint256 partial3_3 = 0x0c48d0f5884341404b884284a89bf31565497101f9fb264c0ad48f08f2b1d12a;
    uint256 partial3_4 = 0x0406144835cb6ff3d6f0cc8f3b59004c00ad8e7d8d1a0c783fd7e6f62dcfd72f;
    uint256 partial3_5 = 0x9e6385262bdfe12c2b9b56f68ecdea66ae7478ee48e248b4d5f27d7d81ec3f62;
    uint256 signature3 = 0xabdd81d633ecd280e7e1e8c9facc2d009da0597068bbc7b2c3c846946f49014f;
    uint256[] sig3 = [signature3, partial3_1, partial3_2, partial3_3, partial3_4, partial3_5];

    // msg: "8c766583fa9c65789080aee687f24be92eaa68775d583eccbc54468f21b7fd190000000000000000000000000000000000000000000000000000000000000034";

    uint256 partial4_1 = 0x01691bb7ea7907b8838444a335b10aa1560c6fb411fc11bb46e00af5bc5f801e;
    uint256 partial4_2 = 0xa9fe69939f727a69c0c1bdb1fdb72874f8169d74e48c9e1b6462eb405f936652;
    uint256 partial4_3 = 0x9216fc553ca2570069eb117f9f13b07d60542d187abd4baa870bb149f76e754f;
    uint256 partial4_4 = 0x1165658dfd6f787ae1da9e109bb299cf34ed2efc6aea9516228af1fd5fe58319;
    uint256 partial4_5 = 0x2b5fbf4c9f10815e112127527d54939ec50608e8aeb321453e4585704778becf;
    uint256 signature4 = 0xa9ffc3d292b69c14a1c5426b9d8d32324bf8b960e3d7cc1608ae67eafbfeaa99;
    uint256[] sig4 = [signature4, partial4_1, partial4_2, partial4_3, partial4_4, partial4_5];

    // msg: "772e33591d14ca86452b2fa0b6c1f50e9a89eebc9d806aaebb6f825169f656390000000000000000000000000000000000000000000000000000000000000045";

    uint256 partial5_1 = 0x08ca74445e3a5e674fc65f3252811ba60aa9f6b2a2497ea89ce30caf2157384b;
    uint256 partial5_2 = 0xad61d8dfaa2d7803cc7dfedd0288ef276a26854647934cce2b4a861a304fd870;
    uint256 partial5_3 = 0x8774d72c680a8bfd6fbb40ffe271d7d7178be5c3436e96f1346db8fa31ecd855;
    uint256 partial5_4 = 0x85281a9afcbffe0eebbe6254571ed25d613bc389813ac3f29608b9c781d7b1b7;
    uint256 partial5_5 = 0x2aeff23f7c46b806d25ac6fb4b736daa4d690b78f28182758ec826a340cc0289;
    uint256 signature5 = 0x2d30b04946f771b974d91268530d8b62918405d949eb193f538351c1b14e79a1;
    uint256[] sig5 = [signature5, partial5_1, partial5_2, partial5_3, partial5_4, partial5_5];

    // msg: "8a767bde8c14db8d024b7b6dd98163398f60d86bc9a4ac814a213db6f489975d0000000000000000000000000000000000000000000000000000000000000056";

    uint256 partial6_1 = 0x9acb515c7bc899987c3ec9284a854e2865cb997be40d3303aebd820cf3674ba7;
    uint256 partial6_2 = 0x304d1593ce934f03dac1a200cfdda4b170b54079bcc6d844b632daca4f12cb13;
    uint256 partial6_3 = 0x219960faa5f120ea6df81b087300a41f94e2abd6f47eabeae41ec3dc589067c2;
    uint256 partial6_4 = 0x1c7edaa28abd9307f3fbe68102c182df5fc67aaef3ed663cd679827fd37545b1;
    uint256 partial6_5 = 0xa0e0a8007443479fcb6965c6e09509487d45be04c0e317e77bf6dd05e1566f1c;
    uint256 signature6 = 0x050ac9abe886d104fdc80d48dd305d1478bf7281b95ed5f84f89002639605f73;
    uint256[] sig6 = [signature6, partial6_1, partial6_2, partial6_3, partial6_4, partial6_5];

    // msg: "13c4fe7022314f4e35623d61d30f71de31192dca0a19898ff36af659e969e7a10000000000000000000000000000000000000000000000000000000000000067";

    uint256 partial7_1 = 0xa433e8373f9b41ff18b8e31efa2ae22109dc6f93945739b46862de2b793bde8e;
    uint256 partial7_2 = 0x8e4b1ad76fb2caa8d798c7a4d9e0418e97e8665b8b5cd878bf312a5ade26e7ca;
    uint256 partial7_3 = 0x226233a323d687d07184a86888446b6151a259af47ae859f4b63d704f3ffcd57;
    uint256 partial7_4 = 0x1eaa6c8cfd49a4b538fd6a36dffaacccdc35277ea7ca78c4e97573d31ebfa485;
    uint256 partial7_5 = 0x9bbdecbe04675f32fdb0abb6a1c8049b0fc8357e8210ba99aab424144884b6f1;
    uint256 signature7 = 0xad6f2ff184eca21d93f93b8fdd21a0c793f2cc8d8659ef1a77468aaa322b6fba;
    uint256[] sig7 = [signature7, partial7_1, partial7_2, partial7_3, partial7_4, partial7_5];

    // msg: "249b38981df5bcd673f3bd50631aba97c5cc5b0dc7a85c94f11705ba950f5b0e0000000000000000000000000000000000000000000000000000000000000078";

    uint256 partial8_1 = 0xad5d31d24d4ff1b9613c39da68d9b5900d512568f8376156778cdc659b79c48d;
    uint256 partial8_2 = 0xa40e928c9845ea0b6e05c6c381677cc21988046e7fba297e17a134a8d9da0484;
    uint256 partial8_3 = 0x0e4239ac002a032e9856ddac866f296114421f8788b080fdcfe425639bbbbfa1;
    uint256 partial8_4 = 0x99dee2aac304d5b4a1432f8277060c1f8825224356e50bfea555b6d16df1a167;
    uint256 partial8_5 = 0xa9140778f6822d579fc427267783717af64ff73a8b81d45acd19b2e06627f69d;
    uint256 signature8 = 0x0fb13e9be1e8632d7d7bbe085ad0c1dd912e508200e52aa998238578763d7832;
    uint256[] sig8 = [signature8, partial8_1, partial8_2, partial8_3, partial8_4, partial8_5];

    // msg: "ee37dd8f5b0b97804d115493672502eb373eca189306b49126816808ac2adcb80000000000000000000000000000000000000000000000000000000000000089";

    uint256 partial9_1 = 0xadb7ace362880cf2081a2c0741d8fd04418e0395956318ff9ba1790824763599;
    uint256 partial9_2 = 0xa3ab309eebf232311c7674d6d4036b4a81d7c2f89f3f56573833a17ae1d16629;
    uint256 partial9_3 = 0x012f18a8f21bd54bf7a522dc4546323f738f410a67d503f29fb4ff6cca09092f;
    uint256 partial9_4 = 0xa72ae610bfd641ab908365a98c8ba80a062b6e0b54e6ae39911da7da400ffdeb;
    uint256 partial9_5 = 0x97e07d101266bc41703cfd31c38b207b9b10dcc49f0aa76b822bbc3b4521aa18;
    uint256 signature9 = 0xa831de13b261538d545e5ed5801b3118663ce3b1420bf4cbdd60950124d5c248;
    uint256[] sig9 = [signature9, partial9_1, partial9_2, partial9_3, partial9_4, partial9_5];

    // msg: "ccee934dfb6e63a7d154fb8cbbe6444659cf6d493b125bba2a931ce22414ddf8000000000000000000000000000000000000000000000000000000000000009a";

    uint256 partial10_1 = 0x9a11790d94c2a0c455692773a2d0dc6641d9f55b04e7b999853a06689a24e9af;
    uint256 partial10_2 = 0x8fe21a6669dbe5bec92fac15e83664acc6e16bd3d48e8846ed070ea6592400c2;
    uint256 partial10_3 = 0x18ebb32d72009a6368a8cc54c7e10aa7e8acd5b326ebc0da81effd1b1f79ed92;
    uint256 partial10_4 = 0x28367a9a90fcfdce26106591ce5ccd8ff3f4388f3836547e485e8c4471188a71;
    uint256 partial10_5 = 0x0cbe7969ca650e88f8208034f33d71f14f536f80f345276652a473effe403f92;
    uint256 signature10 = 0x87494bde9eec7bb33163c94cc9ac729ab3975e82c2b196826bb76d3d120fe415;
    uint256[] sig10 = [signature10, partial10_1, partial10_2, partial10_3, partial10_4, partial10_5];

    // msg: "c8bc8d6c91c5384f0cf6a93dceec05e9580951bf07d8d4839242ac4a566e0e020000000000000000000000000000000000000000000000000000000000000001";

    uint256 partial11_1 = 0xa0b7d6ae339f7a614dfa6d1286629fcdd3ed562cbff70ef4fb4293fc2bfbbb54;
    uint256 partial11_2 = 0x9986c8af2136f0bd7eb7389d6bd17e58e2fb93ea4669dd9d42886d8e43167852;
    uint256 partial11_3 = 0x9eb90655f4b93fd01bdf927a547a7a406c4f66a6e8c3982664f4433046918758;
    uint256 partial11_4 = 0x88da0490707952b32dff64d54154f0f035d6dc8262190f4f902105d0cade9737;
    uint256 partial11_5 = 0x11a424e93c0bf9c6320706462a4329cce7d5642d2cdb18bb84613da7441f4564;
    uint256 signature11 = 0xa0ae19551797db366eaa443a6e82fb06dc0bf67c249817a6c4c2c03fa893fb3c;
    uint256[] sig11 = [signature11, partial11_1, partial11_2, partial11_3, partial11_4, partial11_5];

    // msg: "d1c2a1c7ae81259aae2c94abf016febc938fdda8fd917fb88c50b3c9613c2b3a0000000000000000000000000000000000000000000000000000000000000001";

    uint256 partial12_1 = 0x134f9bcc39bfcfcf0916561cde41193068710c8b9320d71d13e9228445d0c369;
    uint256 partial12_2 = 0x0383e198d44f61d4d6186546869e6de81332b40996caa0542ab7100df1ec4119;
    uint256 partial12_3 = 0x16ac23a9a79c0da4b6348c961bc715aeb437127e741aebec365da9850de8bd8a;
    uint256 partial12_4 = 0x29f1ccde3d31c6d2c825e8aafa2460f07def34acebb47d014c47ed584dbe5d87;
    uint256 partial12_5 = 0xad35d01a2ea8522995b05b2bb2babfc18635749037c0ee3f51614d9c347d56fd;
    uint256 signature12 = 0x954110abb3e2546d46b31f1b1c7cad4d6b6e0c279f242597c84c2a3a64e41367;
    uint256[] sig12 = [signature12, partial12_1, partial12_2, partial12_3, partial12_4, partial12_5];

    // msg: "f249c2d5758f9d37d62f3efc5e26a2509dd97a154b0daf661a4831b5b339fa1c0000000000000000000000000000000000000000000000000000000000000001";

    uint256 partial13_1 = 0x8c2594f9df9aeb9c7bc58257e4cd0fc37a554afd8b2efd6922638458eedf1401;
    uint256 partial13_2 = 0x2cdca76dae09bcc67a2a07f8c32727db5d0243a70199196ddc098148ccfdda6d;
    uint256 partial13_3 = 0x03cf976c1a5618d77274d0631599f44f07d093765467efffed6b265e3153ee54;
    uint256 partial13_4 = 0x89a4afd8bb8b1a14ada33ace850f99c73a11c89e8b0a0c336032ee235ee95835;
    uint256 partial13_5 = 0x9c4fc6c66d44d7b4592727704508f902caa1415d51c796f3460f9a6fc01341fe;
    uint256 signature13 = 0xa3ff1e6e94f55aac94c04430aac0b1a14b3d0de0af426e28cb8f129bc186524e;
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
        Controller.Callback memory callback = controller.getPendingRequest(requestId);
        // mock confirmation times and SIGNATURE_TASK_EXCLUSIVE_WINDOW = 10;
        vm.roll(block.number + callback.requestConfirmations + 10);

        // mock fulfillRandomness directly
        IAdapter.PartialSignature[] memory partialSignatures = new IAdapter.PartialSignature[](3);
        partialSignatures[0] = IAdapter.PartialSignature(0, sig[sigIndex][1]);
        partialSignatures[1] = IAdapter.PartialSignature(1, sig[sigIndex][2]);
        partialSignatures[2] = IAdapter.PartialSignature(2, sig[sigIndex][3]);
        controller.fulfillRandomness(
            0, // fixed group 0
            requestId,
            sig[sigIndex][0],
            partialSignatures
        );
    }

    function prepareSubscription(address consumer, uint96 balance) internal returns (uint64) {
        uint64 subId = controller.createSubscription();
        controller.fundSubscription(subId, balance);
        controller.addConsumer(subId, consumer);
        return subId;
    }

    function getBalance(uint64 subId) internal view returns (uint96, uint96) {
        (uint96 balance, uint96 inflightCost,,,) = controller.getSubscription(subId);
        return (balance, inflightCost);
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
        Controller.CommitDkgParams memory params;

        // Succesful Commit: Node 1
        params = Controller.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey1, disqualifiedNodes);
        vm.prank(node1);
        controller.commitDkg(params);

        // Succesful Commit: Node 2
        params = Controller.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey2, disqualifiedNodes);
        vm.prank(node2);
        controller.commitDkg(params);

        // Succesful Commit: Node 3
        params = Controller.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey3, disqualifiedNodes);
        vm.prank(node3);
        controller.commitDkg(params);

        // Succesful Commit: Node 4
        params = Controller.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey4, disqualifiedNodes);
        vm.prank(node4);
        controller.commitDkg(params);

        // Succesful Commit: Node 5
        params = Controller.CommitDkgParams(groupIndex, groupEpoch, publicKey, partialPublicKey5, disqualifiedNodes);
        vm.prank(node5);
        controller.commitDkg(params);
    }

    function printGroupInfo(uint256 groupIndex) public {
        Controller.Group memory g = controller.getGroup(groupIndex);

        uint256 groupCount = controller.groupCount();
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
        emit log_named_uint("n.staking", node.staking);
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
