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
    address public node10 = address(0xA);
    address public node11 = address(0xB);
    address public node12 = address(0xC);

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
