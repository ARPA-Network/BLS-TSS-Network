pub use coordinator::*;
#[allow(clippy::too_many_arguments, non_camel_case_types)]
pub mod coordinator {
    #![allow(clippy::enum_variant_names)]
    #![allow(dead_code)]
    #![allow(clippy::type_complexity)]
    #![allow(unused_imports)]
    use ethers::contract::{
        builders::{ContractCall, Event},
        Contract, Lazy,
    };
    use ethers::core::{
        abi::{Abi, Detokenize, InvalidOutputType, Token, Tokenizable},
        types::*,
    };
    use ethers::providers::Middleware;
    #[doc = "Coordinator was auto-generated with ethers-rs Abigen. More information at: https://github.com/gakonst/ethers-rs"]
    use std::sync::Arc;
    pub static COORDINATOR_ABI: ethers::contract::Lazy<ethers::core::abi::Abi> =
        ethers::contract::Lazy::new(|| {
            ethers :: core :: utils :: __serde_json :: from_str ("[{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"threshold\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"uint256\",\"name\":\"duration\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"constructor\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"previousOwner\",\"type\":\"address\",\"components\":[],\"indexed\":true},{\"internalType\":\"address\",\"name\":\"newOwner\",\"type\":\"address\",\"components\":[],\"indexed\":true}],\"type\":\"event\",\"name\":\"OwnershipTransferred\",\"outputs\":[],\"anonymous\":false},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"PHASE_DURATION\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"THRESHOLD\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getBlsKeys\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]},{\"internalType\":\"bytes[]\",\"name\":\"\",\"type\":\"bytes[]\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getJustifications\",\"outputs\":[{\"internalType\":\"bytes[]\",\"name\":\"\",\"type\":\"bytes[]\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getParticipants\",\"outputs\":[{\"internalType\":\"address[]\",\"name\":\"\",\"type\":\"address[]\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getResponses\",\"outputs\":[{\"internalType\":\"bytes[]\",\"name\":\"\",\"type\":\"bytes[]\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"getShares\",\"outputs\":[{\"internalType\":\"bytes[]\",\"name\":\"\",\"type\":\"bytes[]\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"inPhase\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"address[]\",\"name\":\"nodes\",\"type\":\"address[]\",\"components\":[]},{\"internalType\":\"bytes[]\",\"name\":\"publicKeys\",\"type\":\"bytes[]\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"initialize\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"justifications\",\"outputs\":[{\"internalType\":\"bytes\",\"name\":\"\",\"type\":\"bytes\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"keys\",\"outputs\":[{\"internalType\":\"bytes\",\"name\":\"\",\"type\":\"bytes\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"owner\",\"outputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"participant_map\",\"outputs\":[{\"internalType\":\"bool\",\"name\":\"\",\"type\":\"bool\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"participants\",\"outputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"bytes\",\"name\":\"value\",\"type\":\"bytes\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"publish\",\"outputs\":[]},{\"inputs\":[],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"renounceOwnership\",\"outputs\":[]},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"responses\",\"outputs\":[{\"internalType\":\"bytes\",\"name\":\"\",\"type\":\"bytes\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\",\"components\":[]}],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"shares\",\"outputs\":[{\"internalType\":\"bytes\",\"name\":\"\",\"type\":\"bytes\",\"components\":[]}]},{\"inputs\":[],\"stateMutability\":\"view\",\"type\":\"function\",\"name\":\"startBlock\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\",\"components\":[]}]},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"newOwner\",\"type\":\"address\",\"components\":[]}],\"stateMutability\":\"nonpayable\",\"type\":\"function\",\"name\":\"transferOwnership\",\"outputs\":[]}]") . expect ("invalid abi")
        });
    #[doc = r" Bytecode of the #name contract"]
    pub static COORDINATOR_BYTECODE: ethers::contract::Lazy<ethers::core::types::Bytes> =
        ethers::contract::Lazy::new(|| {
            "0x60c0604052600060075534801561001557600080fd5b506040516116ea3803806116ea83398101604081905261003491610098565b61003d33610048565b60805260a0526100bc565b600080546001600160a01b038381166001600160a01b0319831681178455604051919092169283917f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e09190a35050565b600080604083850312156100ab57600080fd5b505080516020909101519092909150565b60805160a0516115d8610112600039600081816102150152610b540152600081816101be015281816103c2015281816103f40152818161042d0152818161076b0152818161082d01526108f301526115d86000f3fe608060405234801561001057600080fd5b506004361061012c5760003560e01c80637fd28346116100ad578063cc5ef00911610071578063cc5ef009146102b9578063cd5e3837146102c1578063ce7c2ac2146102d4578063d73fe0aa146102e7578063f2fde38b146102ef57600080fd5b80637fd283461461023757806385edbc1c1461024a5780638da5cb5b1461027d578063a81945961461028e578063b0ef8179146102a457600080fd5b80634ae2b849116100f45780634ae2b849146101b95780635aa68ac0146101e0578063670d14b2146101f5578063715018a614610208578063785ffb371461021057600080fd5b80630ea6564814610131578063221f95111461015a57806335c1d3491461017057806337f8d5ff1461019b57806348cd4cb1146101b0575b600080fd5b61014461013f3660046110de565b610302565b604051610151919061115b565b60405180910390f35b61016261039c565b604051908152602001610151565b61018361017e36600461116e565b61049a565b6040516001600160a01b039091168152602001610151565b6101ae6101a93660046111d3565b6104c4565b005b61016260075481565b6101627f000000000000000000000000000000000000000000000000000000000000000081565b6101e8610669565b604051610151919061123f565b6101446102033660046110de565b6106cb565b6101ae6106e4565b6101627f000000000000000000000000000000000000000000000000000000000000000081565b6101ae61024536600461128c565b6106f8565b61026d6102583660046110de565b60066020526000908152604090205460ff1681565b6040519015158152602001610151565b6000546001600160a01b0316610183565b6102966109f6565b604051610151929190611353565b6102ac610b7b565b6040516101519190611374565b6102ac610cd9565b6101446102cf3660046110de565b610e31565b6101446102e23660046110de565b610e4a565b6102ac610e63565b6101ae6102fd3660046110de565b610fbb565b6003602052600090815260409020805461031b90611387565b80601f016020809104026020016040519081016040528092919081815260200182805461034790611387565b80156103945780601f1061036957610100808354040283529160200191610394565b820191906000526020600020905b81548152906001019060200180831161037757829003601f168201915b505050505081565b60006007546000036103ae5750600090565b6000600754436103be91906113d1565b90507f000000000000000000000000000000000000000000000000000000000000000081116103ef57600191505090565b61041a7f000000000000000000000000000000000000000000000000000000000000000060026113e8565b811161042857600291505090565b6104537f000000000000000000000000000000000000000000000000000000000000000060036113e8565b811161046157600391505090565b60405162461bcd60e51b81526020600482015260096024820152681112d1c8115b99195960ba1b60448201526064015b60405180910390fd5b600581815481106104aa57600080fd5b6000918252602090912001546001600160a01b0316905081565b600754156105145760405162461bcd60e51b815260206004820152601760248201527f444b472068617320616c726561647920737461727465640000000000000000006044820152606401610491565b61051c611034565b60005b8381101561065e5760016006600087878581811061053f5761053f611407565b905060200201602081019061055491906110de565b6001600160a01b031681526020810191909152604001600020805460ff1916911515919091179055600585858381811061059057610590611407565b90506020020160208101906105a591906110de565b81546001810183556000928352602090922090910180546001600160a01b0319166001600160a01b039092169190911790558282828181106105e9576105e9611407565b90506020028101906105fb919061141d565b6001600088888681811061061157610611611407565b905060200201602081019061062691906110de565b6001600160a01b0316815260208101919091526040016000209161064b9190836114c8565b508061065681611589565b91505061051f565b505043600755505050565b606060058054806020026020016040519081016040528092919081815260200182805480156106c157602002820191906000526020600020905b81546001600160a01b031681526001909101906020018083116106a3575b5050505050905090565b6001602052600090815260409020805461031b90611387565b6106ec611034565b6106f6600061108e565b565b3360009081526006602052604090205460ff166107575760405162461bcd60e51b815260206004820152601760248201527f796f7520617265206e6f742072656769737465726564210000000000000000006044820152606401610491565b60006007544361076791906113d1565b90507f000000000000000000000000000000000000000000000000000000000000000081116108285733600090815260026020526040902080546107aa90611387565b1590506108085760405162461bcd60e51b815260206004820152602660248201527f796f75206861766520616c7265616479207075626c697368656420796f75722060448201526573686172657360d01b6064820152608401610491565b3360009081526002602052604090206108228385836114c8565b50505050565b6108537f000000000000000000000000000000000000000000000000000000000000000060026113e8565b81116108ee57336000908152600360205260409020805461087390611387565b1590506108d45760405162461bcd60e51b815260206004820152602960248201527f796f75206861766520616c7265616479207075626c697368656420796f757220604482015268726573706f6e73657360b81b6064820152608401610491565b3360009081526003602052604090206108228385836114c8565b6109197f000000000000000000000000000000000000000000000000000000000000000060036113e8565b81116109b957336000908152600460205260409020805461093990611387565b15905061099f5760405162461bcd60e51b815260206004820152602e60248201527f796f75206861766520616c7265616479207075626c697368656420796f75722060448201526d6a757374696669636174696f6e7360901b6064820152608401610491565b3360009081526004602052604090206108228385836114c8565b60405162461bcd60e51b815260206004820152600d60248201526c1112d1c81a185cc8195b991959609a1b6044820152606401610491565b505050565b60006060600060058054905067ffffffffffffffff811115610a1a57610a1a611464565b604051908082528060200260200182016040528015610a4d57816020015b6060815260200190600190039081610a385790505b50905060005b600554811015610b51576001600060058381548110610a7457610a74611407565b60009182526020808320909101546001600160a01b0316835282019290925260400190208054610aa390611387565b80601f0160208091040260200160405190810160405280929190818152602001828054610acf90611387565b8015610b1c5780601f10610af157610100808354040283529160200191610b1c565b820191906000526020600020905b815481529060010190602001808311610aff57829003601f168201915b5050505050828281518110610b3357610b33611407565b60200260200101819052508080610b4990611589565b915050610a53565b507f0000000000000000000000000000000000000000000000000000000000000000939092509050565b60055460609060009067ffffffffffffffff811115610b9c57610b9c611464565b604051908082528060200260200182016040528015610bcf57816020015b6060815260200190600190039081610bba5790505b50905060005b600554811015610cd3576004600060058381548110610bf657610bf6611407565b60009182526020808320909101546001600160a01b0316835282019290925260400190208054610c2590611387565b80601f0160208091040260200160405190810160405280929190818152602001828054610c5190611387565b8015610c9e5780601f10610c7357610100808354040283529160200191610c9e565b820191906000526020600020905b815481529060010190602001808311610c8157829003601f168201915b5050505050828281518110610cb557610cb5611407565b60200260200101819052508080610ccb90611589565b915050610bd5565b50919050565b60055460609060009067ffffffffffffffff811115610cfa57610cfa611464565b604051908082528060200260200182016040528015610d2d57816020015b6060815260200190600190039081610d185790505b50905060005b600554811015610cd3576003600060058381548110610d5457610d54611407565b60009182526020808320909101546001600160a01b0316835282019290925260400190208054610d8390611387565b80601f0160208091040260200160405190810160405280929190818152602001828054610daf90611387565b8015610dfc5780601f10610dd157610100808354040283529160200191610dfc565b820191906000526020600020905b815481529060010190602001808311610ddf57829003601f168201915b5050505050828281518110610e1357610e13611407565b60200260200101819052508080610e2990611589565b915050610d33565b6004602052600090815260409020805461031b90611387565b6002602052600090815260409020805461031b90611387565b60055460609060009067ffffffffffffffff811115610e8457610e84611464565b604051908082528060200260200182016040528015610eb757816020015b6060815260200190600190039081610ea25790505b50905060005b600554811015610cd3576002600060058381548110610ede57610ede611407565b60009182526020808320909101546001600160a01b0316835282019290925260400190208054610f0d90611387565b80601f0160208091040260200160405190810160405280929190818152602001828054610f3990611387565b8015610f865780601f10610f5b57610100808354040283529160200191610f86565b820191906000526020600020905b815481529060010190602001808311610f6957829003601f168201915b5050505050828281518110610f9d57610f9d611407565b60200260200101819052508080610fb390611589565b915050610ebd565b610fc3611034565b6001600160a01b0381166110285760405162461bcd60e51b815260206004820152602660248201527f4f776e61626c653a206e6577206f776e657220697320746865207a65726f206160448201526564647265737360d01b6064820152608401610491565b6110318161108e565b50565b6000546001600160a01b031633146106f65760405162461bcd60e51b815260206004820181905260248201527f4f776e61626c653a2063616c6c6572206973206e6f7420746865206f776e65726044820152606401610491565b600080546001600160a01b038381166001600160a01b0319831681178455604051919092169283917f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e09190a35050565b6000602082840312156110f057600080fd5b81356001600160a01b038116811461110757600080fd5b9392505050565b6000815180845260005b8181101561113457602081850181015186830182015201611118565b81811115611146576000602083870101525b50601f01601f19169290920160200192915050565b602081526000611107602083018461110e565b60006020828403121561118057600080fd5b5035919050565b60008083601f84011261119957600080fd5b50813567ffffffffffffffff8111156111b157600080fd5b6020830191508360208260051b85010111156111cc57600080fd5b9250929050565b600080600080604085870312156111e957600080fd5b843567ffffffffffffffff8082111561120157600080fd5b61120d88838901611187565b9096509450602087013591508082111561122657600080fd5b5061123387828801611187565b95989497509550505050565b6020808252825182820181905260009190848201906040850190845b818110156112805783516001600160a01b03168352928401929184019160010161125b565b50909695505050505050565b6000806020838503121561129f57600080fd5b823567ffffffffffffffff808211156112b757600080fd5b818501915085601f8301126112cb57600080fd5b8135818111156112da57600080fd5b8660208285010111156112ec57600080fd5b60209290920196919550909350505050565b600081518084526020808501808196508360051b8101915082860160005b8581101561134657828403895261133484835161110e565b9885019893509084019060010161131c565b5091979650505050505050565b82815260406020820152600061136c60408301846112fe565b949350505050565b60208152600061110760208301846112fe565b600181811c9082168061139b57607f821691505b602082108103610cd357634e487b7160e01b600052602260045260246000fd5b634e487b7160e01b600052601160045260246000fd5b6000828210156113e3576113e36113bb565b500390565b6000816000190483118215151615611402576114026113bb565b500290565b634e487b7160e01b600052603260045260246000fd5b6000808335601e1984360301811261143457600080fd5b83018035915067ffffffffffffffff82111561144f57600080fd5b6020019150368190038213156111cc57600080fd5b634e487b7160e01b600052604160045260246000fd5b601f8211156109f157600081815260208120601f850160051c810160208610156114a15750805b601f850160051c820191505b818110156114c0578281556001016114ad565b505050505050565b67ffffffffffffffff8311156114e0576114e0611464565b6114f4836114ee8354611387565b8361147a565b6000601f84116001811461152857600085156115105750838201355b600019600387901b1c1916600186901b178355611582565b600083815260209020601f19861690835b828110156115595786850135825560209485019460019092019101611539565b50868210156115765760001960f88860031b161c19848701351681555b505060018560011b0183555b5050505050565b60006001820161159b5761159b6113bb565b506001019056fea2646970667358221220372aee54ce2f79f1feddc4890490d69f34e5e5473b212389af21f9f25d3e28a664736f6c634300080f0033" . parse () . expect ("invalid bytecode")
        });
    pub struct Coordinator<M>(ethers::contract::Contract<M>);
    impl<M> Clone for Coordinator<M> {
        fn clone(&self) -> Self {
            Coordinator(self.0.clone())
        }
    }
    impl<M> std::ops::Deref for Coordinator<M> {
        type Target = ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M: ethers::providers::Middleware> std::fmt::Debug for Coordinator<M> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.debug_tuple(stringify!(Coordinator))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ethers::providers::Middleware> Coordinator<M> {
        #[doc = r" Creates a new contract instance with the specified `ethers`"]
        #[doc = r" client at the given `Address`. The contract derefs to a `ethers::Contract`"]
        #[doc = r" object"]
        pub fn new<T: Into<ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            ethers::contract::Contract::new(address.into(), COORDINATOR_ABI.clone(), client).into()
        }
        #[doc = r" Constructs the general purpose `Deployer` instance based on the provided constructor arguments and sends it."]
        #[doc = r" Returns a new instance of a deployer that returns an instance of this contract after sending the transaction"]
        #[doc = r""]
        #[doc = r" Notes:"]
        #[doc = r" 1. If there are no constructor arguments, you should pass `()` as the argument."]
        #[doc = r" 1. The default poll duration is 7 seconds."]
        #[doc = r" 1. The default number of confirmations is 1 block."]
        #[doc = r""]
        #[doc = r""]
        #[doc = r" # Example"]
        #[doc = r""]
        #[doc = r" Generate contract bindings with `abigen!` and deploy a new contract instance."]
        #[doc = r""]
        #[doc = r" *Note*: this requires a `bytecode` and `abi` object in the `greeter.json` artifact."]
        #[doc = r""]
        #[doc = r" ```ignore"]
        #[doc = r" # async fn deploy<M: ethers::providers::Middleware>(client: ::std::sync::Arc<M>) {"]
        #[doc = r#"     abigen!(Greeter,"../greeter.json");"#]
        #[doc = r""]
        #[doc = r#"    let greeter_contract = Greeter::deploy(client, "Hello world!".to_string()).unwrap().send().await.unwrap();"#]
        #[doc = r"    let msg = greeter_contract.greet().call().await.unwrap();"]
        #[doc = r" # }"]
        #[doc = r" ```"]
        pub fn deploy<T: ethers::core::abi::Tokenize>(
            client: ::std::sync::Arc<M>,
            constructor_args: T,
        ) -> ::std::result::Result<
            ethers::contract::builders::ContractDeployer<M, Self>,
            ethers::contract::ContractError<M>,
        > {
            let factory = ethers::contract::ContractFactory::new(
                COORDINATOR_ABI.clone(),
                COORDINATOR_BYTECODE.clone().into(),
                client,
            );
            let deployer = factory.deploy(constructor_args)?;
            let deployer = ethers::contract::ContractDeployer::new(deployer);
            Ok(deployer)
        }
        #[doc = "Calls the contract's `PHASE_DURATION` (0x4ae2b849) function"]
        pub fn phase_duration(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([74, 226, 184, 73], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `THRESHOLD` (0x785ffb37) function"]
        pub fn threshold(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([120, 95, 251, 55], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getBlsKeys` (0xa8194596) function"]
        pub fn get_bls_keys(
            &self,
        ) -> ethers::contract::builders::ContractCall<
            M,
            (
                ethers::core::types::U256,
                ::std::vec::Vec<ethers::core::types::Bytes>,
            ),
        > {
            self.0
                .method_hash([168, 25, 69, 150], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getJustifications` (0xb0ef8179) function"]
        pub fn get_justifications(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ::std::vec::Vec<ethers::core::types::Bytes>>
        {
            self.0
                .method_hash([176, 239, 129, 121], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getParticipants` (0x5aa68ac0) function"]
        pub fn get_participants(
            &self,
        ) -> ethers::contract::builders::ContractCall<
            M,
            ::std::vec::Vec<ethers::core::types::Address>,
        > {
            self.0
                .method_hash([90, 166, 138, 192], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getResponses` (0xcc5ef009) function"]
        pub fn get_responses(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ::std::vec::Vec<ethers::core::types::Bytes>>
        {
            self.0
                .method_hash([204, 94, 240, 9], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getShares` (0xd73fe0aa) function"]
        pub fn get_shares(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ::std::vec::Vec<ethers::core::types::Bytes>>
        {
            self.0
                .method_hash([215, 63, 224, 170], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `inPhase` (0x221f9511) function"]
        pub fn in_phase(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([34, 31, 149, 17], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `initialize` (0x37f8d5ff) function"]
        pub fn initialize(
            &self,
            nodes: ::std::vec::Vec<ethers::core::types::Address>,
            public_keys: ::std::vec::Vec<ethers::core::types::Bytes>,
        ) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([55, 248, 213, 255], (nodes, public_keys))
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `justifications` (0xcd5e3837) function"]
        pub fn justifications(
            &self,
            p0: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::Bytes> {
            self.0
                .method_hash([205, 94, 56, 55], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `keys` (0x670d14b2) function"]
        pub fn keys(
            &self,
            p0: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::Bytes> {
            self.0
                .method_hash([103, 13, 20, 178], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `owner` (0x8da5cb5b) function"]
        pub fn owner(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::Address> {
            self.0
                .method_hash([141, 165, 203, 91], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `participant_map` (0x85edbc1c) function"]
        pub fn participant_map(
            &self,
            p0: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, bool> {
            self.0
                .method_hash([133, 237, 188, 28], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `participants` (0x35c1d349) function"]
        pub fn participants(
            &self,
            p0: ethers::core::types::U256,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::Address> {
            self.0
                .method_hash([53, 193, 211, 73], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `publish` (0x7fd28346) function"]
        pub fn publish(
            &self,
            value: ethers::core::types::Bytes,
        ) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([127, 210, 131, 70], value)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `renounceOwnership` (0x715018a6) function"]
        pub fn renounce_ownership(&self) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([113, 80, 24, 166], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `responses` (0x0ea65648) function"]
        pub fn responses(
            &self,
            p0: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::Bytes> {
            self.0
                .method_hash([14, 166, 86, 72], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `shares` (0xce7c2ac2) function"]
        pub fn shares(
            &self,
            p0: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::Bytes> {
            self.0
                .method_hash([206, 124, 42, 194], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `startBlock` (0x48cd4cb1) function"]
        pub fn start_block(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([72, 205, 76, 177], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `transferOwnership` (0xf2fde38b) function"]
        pub fn transfer_ownership(
            &self,
            new_owner: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([242, 253, 227, 139], new_owner)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Gets the contract's `OwnershipTransferred` event"]
        pub fn ownership_transferred_filter(
            &self,
        ) -> ethers::contract::builders::Event<M, OwnershipTransferredFilter> {
            self.0.event()
        }
        #[doc = r" Returns an [`Event`](#ethers_contract::builders::Event) builder for all events of this contract"]
        pub fn events(&self) -> ethers::contract::builders::Event<M, OwnershipTransferredFilter> {
            self.0.event_with_filter(Default::default())
        }
    }
    impl<M: ethers::providers::Middleware> From<ethers::contract::Contract<M>> for Coordinator<M> {
        fn from(contract: ethers::contract::Contract<M>) -> Self {
            Self(contract)
        }
    }
    #[derive(
        Clone,
        Debug,
        Default,
        Eq,
        PartialEq,
        ethers :: contract :: EthEvent,
        ethers :: contract :: EthDisplay,
    )]
    #[ethevent(
        name = "OwnershipTransferred",
        abi = "OwnershipTransferred(address,address)"
    )]
    pub struct OwnershipTransferredFilter {
        #[ethevent(indexed)]
        pub previous_owner: ethers::core::types::Address,
        #[ethevent(indexed)]
        pub new_owner: ethers::core::types::Address,
    }
    #[doc = "Container type for all input parameters for the `PHASE_DURATION` function with signature `PHASE_DURATION()` and selector `[74, 226, 184, 73]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "PHASE_DURATION", abi = "PHASE_DURATION()")]
    pub struct PhaseDurationCall;
    #[doc = "Container type for all input parameters for the `THRESHOLD` function with signature `THRESHOLD()` and selector `[120, 95, 251, 55]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "THRESHOLD", abi = "THRESHOLD()")]
    pub struct ThresholdCall;
    #[doc = "Container type for all input parameters for the `getBlsKeys` function with signature `getBlsKeys()` and selector `[168, 25, 69, 150]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "getBlsKeys", abi = "getBlsKeys()")]
    pub struct GetBlsKeysCall;
    #[doc = "Container type for all input parameters for the `getJustifications` function with signature `getJustifications()` and selector `[176, 239, 129, 121]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "getJustifications", abi = "getJustifications()")]
    pub struct GetJustificationsCall;
    #[doc = "Container type for all input parameters for the `getParticipants` function with signature `getParticipants()` and selector `[90, 166, 138, 192]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "getParticipants", abi = "getParticipants()")]
    pub struct GetParticipantsCall;
    #[doc = "Container type for all input parameters for the `getResponses` function with signature `getResponses()` and selector `[204, 94, 240, 9]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "getResponses", abi = "getResponses()")]
    pub struct GetResponsesCall;
    #[doc = "Container type for all input parameters for the `getShares` function with signature `getShares()` and selector `[215, 63, 224, 170]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "getShares", abi = "getShares()")]
    pub struct GetSharesCall;
    #[doc = "Container type for all input parameters for the `inPhase` function with signature `inPhase()` and selector `[34, 31, 149, 17]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "inPhase", abi = "inPhase()")]
    pub struct InPhaseCall;
    #[doc = "Container type for all input parameters for the `initialize` function with signature `initialize(address[],bytes[])` and selector `[55, 248, 213, 255]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "initialize", abi = "initialize(address[],bytes[])")]
    pub struct InitializeCall {
        pub nodes: ::std::vec::Vec<ethers::core::types::Address>,
        pub public_keys: ::std::vec::Vec<ethers::core::types::Bytes>,
    }
    #[doc = "Container type for all input parameters for the `justifications` function with signature `justifications(address)` and selector `[205, 94, 56, 55]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "justifications", abi = "justifications(address)")]
    pub struct JustificationsCall(pub ethers::core::types::Address);
    #[doc = "Container type for all input parameters for the `keys` function with signature `keys(address)` and selector `[103, 13, 20, 178]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "keys", abi = "keys(address)")]
    pub struct KeysCall(pub ethers::core::types::Address);
    #[doc = "Container type for all input parameters for the `owner` function with signature `owner()` and selector `[141, 165, 203, 91]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "owner", abi = "owner()")]
    pub struct OwnerCall;
    #[doc = "Container type for all input parameters for the `participant_map` function with signature `participant_map(address)` and selector `[133, 237, 188, 28]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "participant_map", abi = "participant_map(address)")]
    pub struct ParticipantMapCall(pub ethers::core::types::Address);
    #[doc = "Container type for all input parameters for the `participants` function with signature `participants(uint256)` and selector `[53, 193, 211, 73]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "participants", abi = "participants(uint256)")]
    pub struct ParticipantsCall(pub ethers::core::types::U256);
    #[doc = "Container type for all input parameters for the `publish` function with signature `publish(bytes)` and selector `[127, 210, 131, 70]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "publish", abi = "publish(bytes)")]
    pub struct PublishCall {
        pub value: ethers::core::types::Bytes,
    }
    #[doc = "Container type for all input parameters for the `renounceOwnership` function with signature `renounceOwnership()` and selector `[113, 80, 24, 166]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "renounceOwnership", abi = "renounceOwnership()")]
    pub struct RenounceOwnershipCall;
    #[doc = "Container type for all input parameters for the `responses` function with signature `responses(address)` and selector `[14, 166, 86, 72]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "responses", abi = "responses(address)")]
    pub struct ResponsesCall(pub ethers::core::types::Address);
    #[doc = "Container type for all input parameters for the `shares` function with signature `shares(address)` and selector `[206, 124, 42, 194]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "shares", abi = "shares(address)")]
    pub struct SharesCall(pub ethers::core::types::Address);
    #[doc = "Container type for all input parameters for the `startBlock` function with signature `startBlock()` and selector `[72, 205, 76, 177]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "startBlock", abi = "startBlock()")]
    pub struct StartBlockCall;
    #[doc = "Container type for all input parameters for the `transferOwnership` function with signature `transferOwnership(address)` and selector `[242, 253, 227, 139]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthCall,
        ethers :: contract :: EthDisplay,
        Default,
    )]
    #[ethcall(name = "transferOwnership", abi = "transferOwnership(address)")]
    pub struct TransferOwnershipCall {
        pub new_owner: ethers::core::types::Address,
    }
    #[derive(Debug, Clone, PartialEq, Eq, ethers :: contract :: EthAbiType)]
    pub enum CoordinatorCalls {
        PhaseDuration(PhaseDurationCall),
        Threshold(ThresholdCall),
        GetBlsKeys(GetBlsKeysCall),
        GetJustifications(GetJustificationsCall),
        GetParticipants(GetParticipantsCall),
        GetResponses(GetResponsesCall),
        GetShares(GetSharesCall),
        InPhase(InPhaseCall),
        Initialize(InitializeCall),
        Justifications(JustificationsCall),
        Keys(KeysCall),
        Owner(OwnerCall),
        ParticipantMap(ParticipantMapCall),
        Participants(ParticipantsCall),
        Publish(PublishCall),
        RenounceOwnership(RenounceOwnershipCall),
        Responses(ResponsesCall),
        Shares(SharesCall),
        StartBlock(StartBlockCall),
        TransferOwnership(TransferOwnershipCall),
    }
    impl ethers::core::abi::AbiDecode for CoordinatorCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::std::result::Result<Self, ethers::core::abi::AbiError> {
            if let Ok(decoded) =
                <PhaseDurationCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::PhaseDuration(decoded));
            }
            if let Ok(decoded) =
                <ThresholdCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::Threshold(decoded));
            }
            if let Ok(decoded) =
                <GetBlsKeysCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::GetBlsKeys(decoded));
            }
            if let Ok(decoded) =
                <GetJustificationsCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::GetJustifications(decoded));
            }
            if let Ok(decoded) =
                <GetParticipantsCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::GetParticipants(decoded));
            }
            if let Ok(decoded) =
                <GetResponsesCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::GetResponses(decoded));
            }
            if let Ok(decoded) =
                <GetSharesCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::GetShares(decoded));
            }
            if let Ok(decoded) =
                <InPhaseCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::InPhase(decoded));
            }
            if let Ok(decoded) =
                <InitializeCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::Initialize(decoded));
            }
            if let Ok(decoded) =
                <JustificationsCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::Justifications(decoded));
            }
            if let Ok(decoded) = <KeysCall as ethers::core::abi::AbiDecode>::decode(data.as_ref()) {
                return Ok(CoordinatorCalls::Keys(decoded));
            }
            if let Ok(decoded) = <OwnerCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::Owner(decoded));
            }
            if let Ok(decoded) =
                <ParticipantMapCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::ParticipantMap(decoded));
            }
            if let Ok(decoded) =
                <ParticipantsCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::Participants(decoded));
            }
            if let Ok(decoded) =
                <PublishCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::Publish(decoded));
            }
            if let Ok(decoded) =
                <RenounceOwnershipCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::RenounceOwnership(decoded));
            }
            if let Ok(decoded) =
                <ResponsesCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::Responses(decoded));
            }
            if let Ok(decoded) = <SharesCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::Shares(decoded));
            }
            if let Ok(decoded) =
                <StartBlockCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::StartBlock(decoded));
            }
            if let Ok(decoded) =
                <TransferOwnershipCall as ethers::core::abi::AbiDecode>::decode(data.as_ref())
            {
                return Ok(CoordinatorCalls::TransferOwnership(decoded));
            }
            Err(ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ethers::core::abi::AbiEncode for CoordinatorCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                CoordinatorCalls::PhaseDuration(element) => element.encode(),
                CoordinatorCalls::Threshold(element) => element.encode(),
                CoordinatorCalls::GetBlsKeys(element) => element.encode(),
                CoordinatorCalls::GetJustifications(element) => element.encode(),
                CoordinatorCalls::GetParticipants(element) => element.encode(),
                CoordinatorCalls::GetResponses(element) => element.encode(),
                CoordinatorCalls::GetShares(element) => element.encode(),
                CoordinatorCalls::InPhase(element) => element.encode(),
                CoordinatorCalls::Initialize(element) => element.encode(),
                CoordinatorCalls::Justifications(element) => element.encode(),
                CoordinatorCalls::Keys(element) => element.encode(),
                CoordinatorCalls::Owner(element) => element.encode(),
                CoordinatorCalls::ParticipantMap(element) => element.encode(),
                CoordinatorCalls::Participants(element) => element.encode(),
                CoordinatorCalls::Publish(element) => element.encode(),
                CoordinatorCalls::RenounceOwnership(element) => element.encode(),
                CoordinatorCalls::Responses(element) => element.encode(),
                CoordinatorCalls::Shares(element) => element.encode(),
                CoordinatorCalls::StartBlock(element) => element.encode(),
                CoordinatorCalls::TransferOwnership(element) => element.encode(),
            }
        }
    }
    impl ::std::fmt::Display for CoordinatorCalls {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match self {
                CoordinatorCalls::PhaseDuration(element) => element.fmt(f),
                CoordinatorCalls::Threshold(element) => element.fmt(f),
                CoordinatorCalls::GetBlsKeys(element) => element.fmt(f),
                CoordinatorCalls::GetJustifications(element) => element.fmt(f),
                CoordinatorCalls::GetParticipants(element) => element.fmt(f),
                CoordinatorCalls::GetResponses(element) => element.fmt(f),
                CoordinatorCalls::GetShares(element) => element.fmt(f),
                CoordinatorCalls::InPhase(element) => element.fmt(f),
                CoordinatorCalls::Initialize(element) => element.fmt(f),
                CoordinatorCalls::Justifications(element) => element.fmt(f),
                CoordinatorCalls::Keys(element) => element.fmt(f),
                CoordinatorCalls::Owner(element) => element.fmt(f),
                CoordinatorCalls::ParticipantMap(element) => element.fmt(f),
                CoordinatorCalls::Participants(element) => element.fmt(f),
                CoordinatorCalls::Publish(element) => element.fmt(f),
                CoordinatorCalls::RenounceOwnership(element) => element.fmt(f),
                CoordinatorCalls::Responses(element) => element.fmt(f),
                CoordinatorCalls::Shares(element) => element.fmt(f),
                CoordinatorCalls::StartBlock(element) => element.fmt(f),
                CoordinatorCalls::TransferOwnership(element) => element.fmt(f),
            }
        }
    }
    impl ::std::convert::From<PhaseDurationCall> for CoordinatorCalls {
        fn from(var: PhaseDurationCall) -> Self {
            CoordinatorCalls::PhaseDuration(var)
        }
    }
    impl ::std::convert::From<ThresholdCall> for CoordinatorCalls {
        fn from(var: ThresholdCall) -> Self {
            CoordinatorCalls::Threshold(var)
        }
    }
    impl ::std::convert::From<GetBlsKeysCall> for CoordinatorCalls {
        fn from(var: GetBlsKeysCall) -> Self {
            CoordinatorCalls::GetBlsKeys(var)
        }
    }
    impl ::std::convert::From<GetJustificationsCall> for CoordinatorCalls {
        fn from(var: GetJustificationsCall) -> Self {
            CoordinatorCalls::GetJustifications(var)
        }
    }
    impl ::std::convert::From<GetParticipantsCall> for CoordinatorCalls {
        fn from(var: GetParticipantsCall) -> Self {
            CoordinatorCalls::GetParticipants(var)
        }
    }
    impl ::std::convert::From<GetResponsesCall> for CoordinatorCalls {
        fn from(var: GetResponsesCall) -> Self {
            CoordinatorCalls::GetResponses(var)
        }
    }
    impl ::std::convert::From<GetSharesCall> for CoordinatorCalls {
        fn from(var: GetSharesCall) -> Self {
            CoordinatorCalls::GetShares(var)
        }
    }
    impl ::std::convert::From<InPhaseCall> for CoordinatorCalls {
        fn from(var: InPhaseCall) -> Self {
            CoordinatorCalls::InPhase(var)
        }
    }
    impl ::std::convert::From<InitializeCall> for CoordinatorCalls {
        fn from(var: InitializeCall) -> Self {
            CoordinatorCalls::Initialize(var)
        }
    }
    impl ::std::convert::From<JustificationsCall> for CoordinatorCalls {
        fn from(var: JustificationsCall) -> Self {
            CoordinatorCalls::Justifications(var)
        }
    }
    impl ::std::convert::From<KeysCall> for CoordinatorCalls {
        fn from(var: KeysCall) -> Self {
            CoordinatorCalls::Keys(var)
        }
    }
    impl ::std::convert::From<OwnerCall> for CoordinatorCalls {
        fn from(var: OwnerCall) -> Self {
            CoordinatorCalls::Owner(var)
        }
    }
    impl ::std::convert::From<ParticipantMapCall> for CoordinatorCalls {
        fn from(var: ParticipantMapCall) -> Self {
            CoordinatorCalls::ParticipantMap(var)
        }
    }
    impl ::std::convert::From<ParticipantsCall> for CoordinatorCalls {
        fn from(var: ParticipantsCall) -> Self {
            CoordinatorCalls::Participants(var)
        }
    }
    impl ::std::convert::From<PublishCall> for CoordinatorCalls {
        fn from(var: PublishCall) -> Self {
            CoordinatorCalls::Publish(var)
        }
    }
    impl ::std::convert::From<RenounceOwnershipCall> for CoordinatorCalls {
        fn from(var: RenounceOwnershipCall) -> Self {
            CoordinatorCalls::RenounceOwnership(var)
        }
    }
    impl ::std::convert::From<ResponsesCall> for CoordinatorCalls {
        fn from(var: ResponsesCall) -> Self {
            CoordinatorCalls::Responses(var)
        }
    }
    impl ::std::convert::From<SharesCall> for CoordinatorCalls {
        fn from(var: SharesCall) -> Self {
            CoordinatorCalls::Shares(var)
        }
    }
    impl ::std::convert::From<StartBlockCall> for CoordinatorCalls {
        fn from(var: StartBlockCall) -> Self {
            CoordinatorCalls::StartBlock(var)
        }
    }
    impl ::std::convert::From<TransferOwnershipCall> for CoordinatorCalls {
        fn from(var: TransferOwnershipCall) -> Self {
            CoordinatorCalls::TransferOwnership(var)
        }
    }
    #[doc = "Container type for all return fields from the `PHASE_DURATION` function with signature `PHASE_DURATION()` and selector `[74, 226, 184, 73]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct PhaseDurationReturn(pub ethers::core::types::U256);
    #[doc = "Container type for all return fields from the `THRESHOLD` function with signature `THRESHOLD()` and selector `[120, 95, 251, 55]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct ThresholdReturn(pub ethers::core::types::U256);
    #[doc = "Container type for all return fields from the `getBlsKeys` function with signature `getBlsKeys()` and selector `[168, 25, 69, 150]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct GetBlsKeysReturn(
        pub ethers::core::types::U256,
        pub ::std::vec::Vec<ethers::core::types::Bytes>,
    );
    #[doc = "Container type for all return fields from the `getJustifications` function with signature `getJustifications()` and selector `[176, 239, 129, 121]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct GetJustificationsReturn(pub ::std::vec::Vec<ethers::core::types::Bytes>);
    #[doc = "Container type for all return fields from the `getParticipants` function with signature `getParticipants()` and selector `[90, 166, 138, 192]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct GetParticipantsReturn(pub ::std::vec::Vec<ethers::core::types::Address>);
    #[doc = "Container type for all return fields from the `getResponses` function with signature `getResponses()` and selector `[204, 94, 240, 9]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct GetResponsesReturn(pub ::std::vec::Vec<ethers::core::types::Bytes>);
    #[doc = "Container type for all return fields from the `getShares` function with signature `getShares()` and selector `[215, 63, 224, 170]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct GetSharesReturn(pub ::std::vec::Vec<ethers::core::types::Bytes>);
    #[doc = "Container type for all return fields from the `inPhase` function with signature `inPhase()` and selector `[34, 31, 149, 17]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct InPhaseReturn(pub ethers::core::types::U256);
    #[doc = "Container type for all return fields from the `justifications` function with signature `justifications(address)` and selector `[205, 94, 56, 55]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct JustificationsReturn(pub ethers::core::types::Bytes);
    #[doc = "Container type for all return fields from the `keys` function with signature `keys(address)` and selector `[103, 13, 20, 178]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct KeysReturn(pub ethers::core::types::Bytes);
    #[doc = "Container type for all return fields from the `owner` function with signature `owner()` and selector `[141, 165, 203, 91]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct OwnerReturn(pub ethers::core::types::Address);
    #[doc = "Container type for all return fields from the `participant_map` function with signature `participant_map(address)` and selector `[133, 237, 188, 28]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct ParticipantMapReturn(pub bool);
    #[doc = "Container type for all return fields from the `participants` function with signature `participants(uint256)` and selector `[53, 193, 211, 73]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct ParticipantsReturn(pub ethers::core::types::Address);
    #[doc = "Container type for all return fields from the `responses` function with signature `responses(address)` and selector `[14, 166, 86, 72]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct ResponsesReturn(pub ethers::core::types::Bytes);
    #[doc = "Container type for all return fields from the `shares` function with signature `shares(address)` and selector `[206, 124, 42, 194]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct SharesReturn(pub ethers::core::types::Bytes);
    #[doc = "Container type for all return fields from the `startBlock` function with signature `startBlock()` and selector `[72, 205, 76, 177]`"]
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        ethers :: contract :: EthAbiType,
        ethers :: contract :: EthAbiCodec,
        Default,
    )]
    pub struct StartBlockReturn(pub ethers::core::types::U256);
}
