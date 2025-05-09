use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::dkg_success::DKGSuccess,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::controller::ControllerViews;
use arpa_core::DKGStatus;
use arpa_dal::GroupInfoHandler;
use async_trait::async_trait;
use ethers::providers::Middleware;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PostCommitGroupingListener<PC: Curve> {
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> std::fmt::Display for PostCommitGroupingListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PostCommitGroupingListener")
    }
}

impl<PC: Curve> PostCommitGroupingListener<PC> {
    pub fn new(
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PostCommitGroupingListener {
            chain_identity,
            group_cache,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Send + Sync + 'static> EventPublisher<DKGSuccess<PC>>
    for PostCommitGroupingListener<PC>
{
    async fn publish(&self, event: DKGSuccess<PC>) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send + 'static> Listener for PostCommitGroupingListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        let dkg_status = self.group_cache.read().await.get_dkg_status();

        if let Ok(DKGStatus::CommitSuccess) = dkg_status {
            let chain_id = self.chain_id().await;

            let group_index = self.group_cache.read().await.get_index()?;

            let client = self.chain_identity.read().await.build_controller_client();

            let id_address = self.chain_identity.read().await.get_id_address();

            if let Ok(group) = client.get_group(group_index).await {
                if group.state {
                    self.publish(DKGSuccess {
                        chain_id,
                        id_address,
                        group,
                    })
                    .await;
                }
            }
        }

        Ok(())
    }

    async fn handle_interruption(&self) -> NodeResult<()> {
        self.chain_identity
            .read()
            .await
            .get_provider()
            .get_net_version()
            .await?;

        Ok(())
    }

    async fn chain_id(&self) -> usize {
        self.chain_identity.read().await.get_chain_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use ethers::signers::{LocalWallet, Signer};
    use ethers_middleware::SignerMiddleware;
    use ethers::contract::{ContractFactory, abigen};
    use threshold_bls::schemes::bn254::G2Curve;
    use crate::queue::EventSubscriber;
    use crate::event::types::Topic;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use arpa_core::{
        GeneralMainChainIdentity, Config, DKGStatus, Group, Member
    };
    use arpa_dal::{
        cache::InMemoryGroupInfoCache,
        GroupInfoHandler
    };
    use ethers::{
        providers::{Provider, Ws, Http},
        types::{Address, Bytes, U256},
        utils::Anvil,
    };
    use std::time::Duration;
    use tokio::time::timeout;
    use anyhow::anyhow;
    use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

    abigen!(
        MockController,
        r#"[
            function getControllerConfig() external view returns (address nodeRegistryAddress,address adapterContractAddress, uint256 disqualifiedNodePenaltyAmount, uint256 defaultNumberOfCommitters,uint256 defaultDkgPhaseDuration, uint256 groupMaxCapacity,uint256 idealNumberOfGroups, uint256 dkgPostProcessReward)
            function getGroup(uint256 groupIndex) external view returns (tuple(uint256 index, uint256 epoch, uint256 size, uint256 threshold, tuple(address nodeIdAddress, uint256[4] memberPartialPublicKey)[] members, address[] committers, tuple(address[] nodeIdAddress, tuple(uint256 groupEpoch, uint256[4] commitResultPublicKey, address[] disqualifiedNodes) commitResult)[] commitCacheList, bool isStrictlyMajorityConsensusReached, uint256[4] groupPublicKey))
            function setGroup(uint256 groupIndex, uint256 epoch, uint256 size, uint256 threshold, bool isStrictlyMajorityConsensusReached, uint256[4] memory groupPublicKey, address[] memory memberAddresses) external
            function setCommitters(uint256 groupIndex, address[] memory committerAddresses) external
            function setMemberPartialPublicKey(uint256 groupIndex, uint256 memberIndex, uint256[4] memory memberPartialPublicKey) external
        ]"#,
    );
    async fn deploy_mock_controller(
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
        node_registry_address: Address
    ) -> Result<Address, Box<dyn std::error::Error>> {
        const CONTROLLER_ABI: &str = r#"[{"inputs":[{"internalType":"address","name":"_nodeRegistryAddress","type":"address"}],"stateMutability":"nonpayable","type":"constructor"},{"inputs":[],"name":"getControllerConfig","outputs":[{"internalType":"address","name":"nodeRegistryContractAddress","type":"address"},{"internalType":"address","name":"adapterContractAddress","type":"address"},{"internalType":"uint256","name":"disqualifiedNodePenaltyAmount","type":"uint256"},{"internalType":"uint256","name":"defaultNumberOfCommitters","type":"uint256"},{"internalType":"uint256","name":"defaultDkgPhaseDuration","type":"uint256"},{"internalType":"uint256","name":"groupMaxCapacity","type":"uint256"},{"internalType":"uint256","name":"idealNumberOfGroups","type":"uint256"},{"internalType":"uint256","name":"dkgPostProcessReward","type":"uint256"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"uint256","name":"groupIndex","type":"uint256"}],"name":"getGroup","outputs":[{"components":[{"internalType":"uint256","name":"index","type":"uint256"},{"internalType":"uint256","name":"epoch","type":"uint256"},{"internalType":"uint256","name":"size","type":"uint256"},{"internalType":"uint256","name":"threshold","type":"uint256"},{"components":[{"internalType":"address","name":"nodeIdAddress","type":"address"},{"internalType":"uint256[4]","name":"partialPublicKey","type":"uint256[4]"}],"internalType":"struct IController.Member[]","name":"members","type":"tuple[]"},{"internalType":"address[]","name":"committers","type":"address[]"},{"components":[{"internalType":"address[]","name":"nodeIdAddress","type":"address[]"},{"components":[{"internalType":"uint256","name":"groupEpoch","type":"uint256"},{"internalType":"uint256[4]","name":"publicKey","type":"uint256[4]"},{"internalType":"address[]","name":"disqualifiedNodes","type":"address[]"}],"internalType":"struct IController.CommitResult","name":"commitResult","type":"tuple"}],"internalType":"struct IController.CommitCache[]","name":"commitCacheList","type":"tuple[]"},{"internalType":"bool","name":"isStrictlyMajorityConsensusReached","type":"bool"},{"internalType":"uint256[4]","name":"publicKey","type":"uint256[4]"}],"internalType":"struct IController.Group","name":"","type":"tuple"}],"stateMutability":"view","type":"function"},{"inputs":[],"name":"nodeRegistryAddress","outputs":[{"internalType":"address","name":"","type":"address"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"uint256","name":"groupIndex","type":"uint256"},{"internalType":"address[]","name":"committerAddresses","type":"address[]"}],"name":"setCommitters","outputs":[],"stateMutability":"nonpayable","type":"function"},{"inputs":[{"internalType":"uint256","name":"groupIndex","type":"uint256"},{"internalType":"uint256","name":"epoch","type":"uint256"},{"internalType":"uint256","name":"size","type":"uint256"},{"internalType":"uint256","name":"threshold","type":"uint256"},{"internalType":"bool","name":"isStrictlyMajorityConsensusReached","type":"bool"},{"internalType":"uint256[4]","name":"publicKey","type":"uint256[4]"},{"internalType":"address[]","name":"memberAddresses","type":"address[]"}],"name":"setGroup","outputs":[],"stateMutability":"nonpayable","type":"function"},{"inputs":[{"internalType":"uint256","name":"groupIndex","type":"uint256"},{"internalType":"uint256","name":"memberIndex","type":"uint256"},{"internalType":"uint256[4]","name":"partialPublicKey","type":"uint256[4]"}],"name":"setMemberPartialPublicKey","outputs":[],"stateMutability":"nonpayable","type":"function"}]"#;
        const CONTROLLER_BYTECODE: &str = "6080604052348015600e575f5ffd5b50604051610c72380380610c72833981016040819052602b91604f565b600180546001600160a01b0319166001600160a01b0392909216919091179055607a565b5f60208284031215605e575f5ffd5b81516001600160a01b03811681146073575f5ffd5b9392505050565b610beb806100875f395ff3fe608060405234801561000f575f5ffd5b5060043610610060575f3560e01c80631bb1fd281461006457806326a93abe146100795780636971f0961461008c578063ceb606541461009f578063d11b8e68146100c8578063fec10aa914610115575b5f5ffd5b61007761007236600461080b565b610140565b005b6100776100873660046108ba565b610167565b61007761009a366004610942565b61027e565b6100b26100ad366004610975565b6102c6565b6040516100bf9190610aed565b60405180910390f35b600154604080516001600160a01b0390921682525f60208301526103e89082015260056060820152606460808201819052600a60a0830152600360c083015260e0820152610100016100bf565b600154610128906001600160a01b031681565b6040516001600160a01b0390911681526020016100bf565b5f828152602081815260409091208251610162926005909201918401906105c7565b505050565b5f87815260208190526040902087815560018101879055600281018690556003810185905560078101805460ff19168515151790556101ab6008820184600461062a565b506101b9600482015f610658565b5f5b825181101561025d576101cc610679565b8260040160405180604001604052808685815181106101ed576101ed610ba1565b6020908102919091018101516001600160a01b0390811683529181018590528354600180820186555f95865294829020845160059092020180546001600160a01b031916919093161782558201519192909161024d91830190600461062a565b5050600190920191506101bb9050565b50815161027390600583019060208501906105c7565b505050505050505050565b805f5f8581526020019081526020015f2060040183815481106102a3576102a3610ba1565b905f5260205f2090600502016001019060046102c092919061062a565b50505050565b6102ce610697565b5f828152602081815260408083208151610120810183528154815260018201548185015260028201548184015260038201546060820152600482018054845181870281018701909552808552919592946080870194939192919084015b828210156103a3575f8481526020908190206040805180820182526005860290920180546001600160a01b0316835281516080810190925291928301906001830160048282826020028201915b815481526020019060010190808311610378575050505050815250508152602001906001019061032b565b5050505081526020016005820180548060200260200160405190810160405280929190818152602001828054801561040257602002820191905f5260205f20905b81546001600160a01b031681526001909101906020018083116103e4575b5050505050815260200160068201805480602002602001604051908101604052809291908181526020015f905b82821015610571578382905f5260205f2090600702016040518060400160405290815f82018054806020026020016040519081016040528092919081815260200182805480156104a657602002820191905f5260205f20905b81546001600160a01b03168152600190910190602001808311610488575b50505091835250506040805160608101825260018401805482528251608081019384905260209485019492939192840191600287019060049082845b8154815260200190600101908083116104e257505050505081526020016005820180548060200260200160405190810160405280929190818152602001828054801561055557602002820191905f5260205f20905b81546001600160a01b03168152600190910190602001808311610537575b505050505081525050815250508152602001906001019061042f565b50505090825250600782015460ff1615156020820152604080516080810182529101906008830160048282826020028201915b8154815260200190600101908083116105a4575050505050815250509050919050565b828054828255905f5260205f2090810192821561061a579160200282015b8281111561061a57825182546001600160a01b0319166001600160a01b039091161782556020909201916001909101906105e5565b506106269291506106e5565b5090565b826004810192821561061a579160200282015b8281111561061a57825182559160200191906001019061063d565b5080545f8255600502905f5260205f209081019061067691906106f9565b50565b60405180608001604052806004906020820280368337509192915050565b6040518061012001604052805f81526020015f81526020015f81526020015f81526020016060815260200160608152602001606081526020015f151581526020016106e0610679565b905290565b5b80821115610626575f81556001016106e6565b808211156106265780546001600160a01b03191681555f60018201819055600282018190556003820181905560048201556005016106f9565b634e487b7160e01b5f52604160045260245ffd5b604051601f8201601f1916810167ffffffffffffffff8111828210171561076f5761076f610732565b604052919050565b5f82601f830112610786575f5ffd5b813567ffffffffffffffff8111156107a0576107a0610732565b8060051b6107b060208201610746565b918252602081850181019290810190868411156107cb575f5ffd5b6020860192505b838310156108015782356001600160a01b03811681146107f0575f5ffd5b8252602092830192909101906107d2565b9695505050505050565b5f5f6040838503121561081c575f5ffd5b82359150602083013567ffffffffffffffff811115610839575f5ffd5b61084585828601610777565b9150509250929050565b5f82601f83011261085e575f5ffd5b6040516080810167ffffffffffffffff8111828210171561088157610881610732565b604052806080840185811115610895575f5ffd5b845b818110156108af578035835260209283019201610897565b509195945050505050565b5f5f5f5f5f5f5f610140888a0312156108d1575f5ffd5b87359650602088013595506040880135945060608801359350608088013580151581146108fc575f5ffd5b925061090b8960a08a0161084f565b915061012088013567ffffffffffffffff811115610927575f5ffd5b6109338a828b01610777565b91505092959891949750929550565b5f5f5f60c08486031215610954575f5ffd5b833592506020840135915061096c856040860161084f565b90509250925092565b5f60208284031215610985575f5ffd5b5035919050565b805f5b60048110156102c057815184526020938401939091019060010161098f565b5f8151808452602084019350602083015f5b82811015610a0157815180516001600160a01b03168752602090810151906109ea9088018261098c565b5060a09590950194602091909101906001016109c0565b5093949350505050565b5f8151808452602084019350602083015f5b82811015610a015781516001600160a01b0316865260209586019590910190600101610a1d565b5f82825180855260208501945060208160051b830101602085015f5b83811015610ae157601f198584030188528151805160408552610a866040860182610a0b565b9050602082015191508481036020860152815181526020820151610aad602083018261098c565b506040820151915060c060a0820152610ac960c0820183610a0b565b60209a8b019a90955093909301925050600101610a60565b50909695505050505050565b60208152815160208201526020820151604082015260408201516060820152606082015160808201525f608083015161018060a0840152610b326101a08401826109ae565b905060a0840151601f198483030160c0850152610b4f8282610a0b565b91505060c0840151601f198483030160e0850152610b6d8282610a44565b91505060e0840151610b8461010085018215159052565b50610100840151610b9961012085018261098c565b509392505050565b634e487b7160e01b5f52603260045260245ffdfea26469706673582212201d5cb75c79769b26d2acac3cb7a048910adaa08ad09c80e0089e3957ba91d9dd64736f6c634300081b0033";

        let controller_factory = ContractFactory::new(
            serde_json::from_str(CONTROLLER_ABI).expect("Invalid CONTROLLER_ABI"),
            CONTROLLER_BYTECODE.parse::<Bytes>().expect("Invalid CONTROLLER_BYTECODE"),
            client.clone(),
        );
        
        let controller_contract_deployed = controller_factory.deploy(node_registry_address)?.send().await?;
        let controller_address = controller_contract_deployed.address();

        Ok(controller_address)
    }

    async fn mock_subscribe_to_events<PC: Curve + Send + Sync + 'static>(
        eq: &mut EventQueue,
        subscriber_name: &str,
    ) -> tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>> {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        
        struct TestSubscriber<PC: Curve + Send + Sync + 'static> {
            name: String,
            sender: tokio::sync::mpsc::Sender<Box<dyn std::any::Any + Send>>,
            pc: PhantomData<PC>,
        }
        
        impl<PC: Curve + Send + Sync + 'static> std::fmt::Debug for TestSubscriber<PC> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "TestSubscriber({})", self.name)
            }
        }
        
        #[async_trait]
        impl<PC: Curve + Send + Sync + 'static> Subscriber for TestSubscriber<PC> {
            async fn notify(&self, _topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()> {
                if let Some(success_event) = payload.as_any().downcast_ref::<DKGSuccess<PC>>() {
                    let cloned_event = DKGSuccess {
                        chain_id: success_event.chain_id,
                        id_address: success_event.id_address,
                        group: success_event.group.clone(),
                    };
                    let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                    self.sender.send(boxed).await.map_err(|e| {
                        let err: crate::error::NodeError = anyhow!("Failed to send event: {}", e).into();
                        err
                    })?;
                }
                Ok(())
            }
            
            async fn subscribe(self) {
            }
        }
        
        impl<PC: Curve + Send + Sync + 'static> DebuggableSubscriber for TestSubscriber<PC> {}
        
        let subscriber: TestSubscriber<PC> = TestSubscriber {
            name: subscriber_name.to_string(),
            sender,
            pc: PhantomData,
        };

        let dummy_id_address = Address::random();
        let dummy_group = Group::<PC> {
            index: 1,
            epoch: 1,
            size: 3,
            threshold: 2,
            state: true,
            public_key: None,
            members: BTreeMap::new(),
            committers: vec![Address::random()],
            c: PhantomData,
        };
        
        let dummy_event = DKGSuccess::<PC> { 
            chain_id: 0, 
            id_address: dummy_id_address,
            group: dummy_group
        };
        
        let topic = dummy_event.topic();
        
        eq.subscribe(topic, Box::new(subscriber));
        
        receiver
    }
    
    #[tokio::test]
    async fn test_post_commit_grouping_listener() -> NodeResult<()> {
        let anvil = Anvil::new().spawn();
        
        let http_provider = Provider::<Http>::try_from(anvil.endpoint())
            .map_err(|e| anyhow!("Failed to create HTTP provider: {}", e))?;
        
        let ws_provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await?);
        
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let id_address = wallet.address();
        
        let chain_id = anvil.chain_id() as usize;
        
        let client = Arc::new(SignerMiddleware::new(
            http_provider,
            wallet.clone().with_chain_id(anvil.chain_id()),
        ));
        
        let node_registry_address = Address::random();
        
        let controller_address = deploy_mock_controller(
            client.clone(), 
            node_registry_address,
        ).await.map_err(|e| anyhow!("Failed to deploy mock controller: {}", e))?;
        
        let config = Config::default();
        
        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet.clone(),
            ws_provider.clone(),
            anvil.ws_endpoint(),
            controller_address,
            Address::random(),
            node_registry_address,
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
        );
        
        let controller = MockController::new(controller_address, client.clone());
        
        let group_index = 1;
        let epoch = 1;
        let size = 3;
        let threshold = 2;
        let group_state = true;
        
        let mut member_addresses = Vec::new();
        member_addresses.push(id_address);
        member_addresses.push(Address::random());
        member_addresses.push(Address::random());
        
        let empty_public_key: [U256; 4] = [U256::zero(), U256::zero(), U256::zero(), U256::zero()];
        
        controller.set_group(
            group_index.into(),
            epoch.into(),
            size.into(),
            threshold.into(),
            group_state,
            empty_public_key,
            member_addresses.clone(),
        ).send().await
        .map_err(|e| anyhow!("Failed to send transaction: {}", e))?
        .await
        .map_err(|e| anyhow!("Transaction failed: {}", e))?;
        
        let mut committer_addresses = Vec::new();
        committer_addresses.push(id_address);
        
        controller.set_committers(
            group_index.into(),
            committer_addresses.clone(),
        ).send().await
        .map_err(|e| anyhow!("Failed to send transaction: {}", e))?
        .await
        .map_err(|e| anyhow!("Transaction failed: {}", e))?;
        
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address)),
        ));
        
        {
            let mut group_cache_write = group_cache.write().await;
            let dkg_task = arpa_core::DKGTask {
                group_index,
                epoch,
                size,
                threshold,
                assignment_block_height: 100,
                members: member_addresses.clone(),
                coordinator_address: Address::random()
            };
            
            group_cache_write.save_task_info(chain_id, dkg_task).await?;
            group_cache_write.update_dkg_status(group_index, epoch, DKGStatus::CommitSuccess).await?;
            assert_eq!(group_cache_write.get_index().unwrap(), group_index);
            assert_eq!(group_cache_write.get_dkg_start_block_height().unwrap(), 100);
        }
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        let listener = PostCommitGroupingListener::<G2Curve>::new(
            chain_identity_arc.clone(),
            group_cache.clone(),
            event_queue.clone(),
        );
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscribe_to_events::<G2Curve>(&mut *eq_write, "test_subscriber").await
        };
        
        let mut members = BTreeMap::new();
        members.insert(id_address, Member {
            index: 0,
            dkg_index: Some(1),
            id_address,
            rpc_endpoint: None,
            partial_public_key: None,
        });
        
        let test_group = Group::<G2Curve> {
            index: group_index,
            epoch,
            size,
            threshold,
            state: group_state,
            public_key: None,
            members,
            committers: committer_addresses.clone(),
            c: PhantomData,
        };
        
        listener.publish(DKGSuccess {
            chain_id,
            id_address,
            group: test_group.clone(),
        }).await;
        
        let received_event = timeout(Duration::from_secs(5), event_receiver.recv()).await
            .map_err(|_| anyhow!("Timeout: No event received"))?
            .ok_or_else(|| anyhow!("Error: Event channel closed"))?;
        
        if let Some(success_event) = received_event.downcast_ref::<DKGSuccess<G2Curve>>() {
            assert_eq!(success_event.chain_id, chain_id);
            assert_eq!(success_event.id_address, id_address);
            assert_eq!(success_event.group.index, group_index);
            assert_eq!(success_event.group.state, group_state);
        } else {
            return Err(anyhow!("Received unexpected event type").into());
        }
        
        let listen_result = listener.listen().await;
        assert!(listen_result.is_ok(), "Listen method should not fail");
        
        let listen_triggered_event = timeout(Duration::from_secs(5), event_receiver.recv()).await
            .map_err(|_| anyhow!("Timeout: No listen-triggered event received"))?
            .ok_or_else(|| anyhow!("Error: Event channel closed"))?;
            
        if let Some(success_event) = listen_triggered_event.downcast_ref::<DKGSuccess<G2Curve>>() {
            assert_eq!(success_event.chain_id, chain_id);
            assert_eq!(success_event.id_address, id_address);
            assert_eq!(success_event.group.index, group_index);
            assert_eq!(success_event.group.epoch, epoch);
            assert_eq!(success_event.group.size, size);
            assert_eq!(success_event.group.threshold, threshold);
            assert_eq!(success_event.group.state, group_state);
        } else {
            return Err(anyhow!("Received unexpected event type from listen method").into());
        }
        
        {
            let mut group_cache_write = group_cache.write().await;
            group_cache_write.update_dkg_status(group_index, epoch, DKGStatus::None).await?;
        }
        
        let new_event_queue = Arc::new(RwLock::new(EventQueue::new()));
        let listener_for_not_ready = PostCommitGroupingListener::<G2Curve>::new(
            chain_identity_arc.clone(),
            group_cache.clone(),
            new_event_queue.clone(),
        );
        
        let mut event_receiver = {
            let mut eq_write = new_event_queue.write().await;
            mock_subscribe_to_events::<G2Curve>(&mut *eq_write, "test_subscriber").await
        };
        
        listener_for_not_ready.listen().await?;
        
        let timeout_result = timeout(Duration::from_millis(500), event_receiver.recv()).await;
        assert!(timeout_result.is_err(), "Unexpectedly received an event when DKG status is not CommitSuccess");
        
        {
            let mut group_cache_write = group_cache.write().await;
            group_cache_write.update_dkg_status(group_index, epoch, DKGStatus::CommitSuccess).await?;
        }
        
        controller.set_group(
            group_index.into(),
            epoch.into(),
            size.into(),
            threshold.into(),
            false,
            empty_public_key,
            member_addresses.clone(),
        ).send().await
        .map_err(|e| anyhow!("Failed to send transaction: {}", e))?
        .await
        .map_err(|e| anyhow!("Transaction failed: {}", e))?;
        
        let new_event_queue2 = Arc::new(RwLock::new(EventQueue::new()));
        let listener_for_inactive_group = PostCommitGroupingListener::<G2Curve>::new(
            chain_identity_arc.clone(),
            group_cache.clone(),
            new_event_queue2.clone(),
        );
        
        let mut event_receiver = {
            let mut eq_write = new_event_queue2.write().await;
            mock_subscribe_to_events::<G2Curve>(&mut *eq_write, "test_subscriber").await
        };
        
        listener_for_inactive_group.listen().await?;
        
        let timeout_result = timeout(Duration::from_millis(500), event_receiver.recv()).await;
        assert!(timeout_result.is_err(), "Unexpectedly received an event when group state is false");
        
        let interruption_result = listener.handle_interruption().await;
        assert!(interruption_result.is_ok(), "Handle interruption failed");
        
        assert_eq!(listener.chain_id().await, chain_id);
        
        let display_string = format!("{}", listener);
        assert_eq!(display_string, "PostCommitGroupingListener");
        
        Ok(())
    }
}