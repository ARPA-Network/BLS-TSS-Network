use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::node_activation::NodeActivation,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::{controller::ControllerViews, node_registry::NodeRegistryViews};
use arpa_core::ListenerDescriptor;
use async_trait::async_trait;
use ethers::{providers::Middleware, types::Address};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct NodeActivationListener<PC: Curve> {
    listener_descriptor: ListenerDescriptor,
    is_eigenlayer: bool,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    node_registry_address: Option<Address>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> std::fmt::Display for NodeActivationListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeActivationListener")
    }
}

impl<PC: Curve> NodeActivationListener<PC> {
    pub fn new(
        listener_descriptor: ListenerDescriptor,
        is_eigenlayer: bool,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        NodeActivationListener {
            listener_descriptor,
            is_eigenlayer,
            chain_identity,
            node_registry_address: None,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> EventPublisher<NodeActivation> for NodeActivationListener<PC> {
    async fn publish(&self, event: NodeActivation) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for NodeActivationListener<PC> {
    async fn initialize(&mut self) -> NodeResult<()> {
        if self.node_registry_address.is_none() {
            let controller_client = self.chain_identity.read().await.build_controller_client();

            let node_registry_address =
                ControllerViews::<PC>::get_node_registry_address(&controller_client).await?;

            self.node_registry_address = Some(node_registry_address);
        }

        Ok(())
    }

    async fn listen(&self) -> NodeResult<()> {
        let self_address = self.chain_identity.read().await.get_id_address();

        let node_registry_address = self.node_registry_address.unwrap();

        let node_registry_client = self
            .chain_identity
            .read()
            .await
            .build_node_registry_client(node_registry_address);

        let node = node_registry_client.get_node(self_address).await?;

        if node.id_address == self_address && !node.state {
            self.publish(NodeActivation {
                chain_id: self.listener_descriptor.chain_id,
                is_eigenlayer: self.is_eigenlayer,
                node_registry_address,
            })
            .await;
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

    fn chain_id(&self) -> usize {
        self.listener_descriptor.chain_id
    }

    fn listener_descriptor(&self) -> ListenerDescriptor {
        self.listener_descriptor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::signers::{LocalWallet, Signer};
    use ethers_middleware::SignerMiddleware;
    use ethers::contract::{ContractFactory, abigen};
    use threshold_bls::schemes::bn254::G2Curve;
    use crate::queue::EventSubscriber;
    use crate::event::types::Topic;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use arpa_core::{Config, GeneralMainChainIdentity};
    use ethers::{
        providers::{Provider, Ws, Http},
        types::{Address, Bytes},
        utils::Anvil,
    };
    use std::time::Duration;
    use tokio::time::timeout;
    use anyhow::anyhow;
    use std::sync::Arc;

    abigen!(
        MockController,
        r#"[
            function getControllerConfig() external view returns (address nodeRegistryAddress,address adapterContractAddress, uint256 disqualifiedNodePenaltyAmount, uint256 defaultNumberOfCommitters,uint256 defaultDkgPhaseDuration, uint256 groupMaxCapacity,uint256 idealNumberOfGroups, uint256 dkgPostProcessReward)
        ]"#,
    );

    abigen!(
        MockNodeRegistry,
        r#"[
            function getNode(address idAddress) external view returns (tuple(address idAddress, bytes dkgPublicKey, bool isEigenlayerNode, bool state, uint256 pendingUntilBlock))
            function setNodeState(address idAddress, bool state) external
            function registerNode(address idAddress, bytes memory dkgPublicKey, bool isEigenlayerNode) external
        ]"#,
    );

    async fn deploy_mock_contracts(
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
        node_address: Address,
        is_node_active: bool
    ) -> Result<(Address, Address), Box<dyn std::error::Error>> {
        println!("Deploying mock contracts...");

        const NODE_REGISTRY_ABI: &str = r#"[{"inputs":[{"internalType":"address","name":"nodeAddress","type":"address"}],"name":"getNode","outputs":[{"components":[{"internalType":"address","name":"idAddress","type":"address"},{"internalType":"bytes","name":"dkgPublicKey","type":"bytes"},{"internalType":"bool","name":"isEigenlayerNode","type":"bool"},{"internalType":"bool","name":"state","type":"bool"},{"internalType":"uint256","name":"pendingUntilBlock","type":"uint256"}],"internalType":"struct INodeRegistry.Node","name":"","type":"tuple"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"address","name":"","type":"address"}],"name":"nodes","outputs":[{"internalType":"address","name":"idAddress","type":"address"},{"internalType":"bytes","name":"dkgPublicKey","type":"bytes"},{"internalType":"bool","name":"isEigenlayerNode","type":"bool"},{"internalType":"bool","name":"state","type":"bool"},{"internalType":"uint256","name":"pendingUntilBlock","type":"uint256"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"address","name":"idAddress","type":"address"},{"internalType":"bytes","name":"dkgPublicKey","type":"bytes"},{"internalType":"bool","name":"isEigenlayerNode","type":"bool"}],"name":"registerNode","outputs":[],"stateMutability":"nonpayable","type":"function"},{"inputs":[{"internalType":"address","name":"idAddress","type":"address"},{"internalType":"bool","name":"state","type":"bool"}],"name":"setNodeState","outputs":[],"stateMutability":"nonpayable","type":"function"}]"#;
        const NODE_REGISTRY_BYTECODE: &str = "6080604052348015600e575f5ffd5b506107178061001c5f395ff3fe608060405234801561000f575f5ffd5b506004361061004a575f3560e01c8063189a5a171461004e578063933ff7b51461007b5780639d20904814610090578063f56f705f146100b0575b5f5ffd5b61006161005c366004610392565b6100f1565b6040516100729594939291906103e0565b60405180910390f35b61008e610089366004610443565b6101b6565b005b6100a361009e366004610392565b610261565b6040516100729190610516565b61008e6100be366004610571565b6001600160a01b039091165f90815260208190526040902060020180549115156101000261ff0019909216919091179055565b5f60208190529081526040902080546001820180546001600160a01b03909216929161011c906105a2565b80601f0160208091040260200160405190810160405280929190818152602001828054610148906105a2565b80156101935780601f1061016a57610100808354040283529160200191610193565b820191905f5260205f20905b81548152906001019060200180831161017657829003601f168201915b505050506002830154600390930154919260ff8082169361010090920416915085565b6040805160a0810182526001600160a01b038581168083526020808401878152861515858701525f606086018190526080860181905292835290829052939020825181546001600160a01b0319169216919091178155915190919060018201906102209082610626565b506040820151600282018054606085015115156101000261ff00199315159390931661ffff1990911617919091179055608090910151600390910155505050565b6040805160a080820183525f80835260606020808501829052848601839052908401829052608084018290526001600160a01b0386811683528282529185902085519384019095528454909116825260018401805493949293918401916102c7906105a2565b80601f01602080910402602001604051908101604052809291908181526020018280546102f3906105a2565b801561033e5780601f106103155761010080835404028352916020019161033e565b820191905f5260205f20905b81548152906001019060200180831161032157829003601f168201915b5050509183525050600282015460ff80821615156020840152610100909104161515604082015260039091015460609091015292915050565b80356001600160a01b038116811461038d575f5ffd5b919050565b5f602082840312156103a2575f5ffd5b6103ab82610377565b9392505050565b5f81518084528060208401602086015e5f602082860101526020601f19601f83011685010191505092915050565b6001600160a01b038616815260a0602082018190525f90610403908301876103b2565b941515604083015250911515606083015260809091015292915050565b634e487b7160e01b5f52604160045260245ffd5b8035801515811461038d575f5ffd5b5f5f5f60608486031215610455575f5ffd5b61045e84610377565b9250602084013567ffffffffffffffff811115610479575f5ffd5b8401601f81018613610489575f5ffd5b803567ffffffffffffffff8111156104a3576104a3610420565b604051601f8201601f19908116603f0116810167ffffffffffffffff811182821017156104d2576104d2610420565b6040528181528282016020018810156104e9575f5ffd5b816020840160208301375f6020838301015280945050505061050d60408501610434565b90509250925092565b602080825282516001600160a01b03168282015282015160a060408301525f9061054360c08401826103b2565b9050604084015115156060840152606084015115156080840152608084015160a08401528091505092915050565b5f5f60408385031215610582575f5ffd5b61058b83610377565b915061059960208401610434565b90509250929050565b600181811c908216806105b657607f821691505b6020821081036105d457634e487b7160e01b5f52602260045260245ffd5b50919050565b601f82111561062157805f5260205f20601f840160051c810160208510156105ff5750805b601f840160051c820191505b8181101561061e575f815560010161060b565b50505b505050565b815167ffffffffffffffff81111561064057610640610420565b6106548161064e84546105a2565b846105da565b6020601f821160018114610686575f831561066f5750848201515b5f19600385901b1c1916600184901b17845561061e565b5f84815260208120601f198516915b828110156106b55787850151825560209485019460019092019101610695565b50848210156106d257868401515f19600387901b60f8161c191681555b50505050600190811b0190555056fea2646970667358221220fb71010de0b2c73e60fab5dfe9230f2e003333956bcf1c2394d35a91cb89ebd064736f6c634300081b0033";

        const CONTROLLER_ABI: &str = r#"[{"inputs":[{"internalType":"address","name":"_nodeRegistryAddress","type":"address"}],"stateMutability":"nonpayable","type":"constructor"},{"inputs":[],"name":"adapterAddress","outputs":[{"internalType":"address","name":"","type":"address"}],"stateMutability":"view","type":"function"},{"inputs":[],"name":"getControllerConfig","outputs":[{"internalType":"address","name":"nodeRegistryContractAddress","type":"address"},{"internalType":"address","name":"adapterContractAddress","type":"address"},{"internalType":"uint256","name":"disqualifiedNodePenaltyAmount","type":"uint256"},{"internalType":"uint256","name":"defaultNumberOfCommitters","type":"uint256"},{"internalType":"uint256","name":"defaultDkgPhaseDuration","type":"uint256"},{"internalType":"uint256","name":"groupMaxCapacity","type":"uint256"},{"internalType":"uint256","name":"idealNumberOfGroups","type":"uint256"},{"internalType":"uint256","name":"dkgPostProcessReward","type":"uint256"}],"stateMutability":"view","type":"function"},{"inputs":[],"name":"nodeRegistryAddress","outputs":[{"internalType":"address","name":"","type":"address"}],"stateMutability":"view","type":"function"}]"#;
        const CONTROLLER_BYTECODE: &str = "6080604052348015600e575f5ffd5b50604051610192380380610192833981016040819052602b916055565b5f80546001600160a01b039092166001600160a01b03199283161790556001805490911690556080565b5f602082840312156064575f5ffd5b81516001600160a01b03811681146079575f5ffd5b9392505050565b6101058061008d5f395ff3fe6080604052348015600e575f5ffd5b5060043610603a575f3560e01c806366c9aba614603e578063d11b8e6814606d578063fec10aa91460be575b5f5ffd5b6001546050906001600160a01b031681565b6040516001600160a01b0390911681526020015b60405180910390f35b5f54600154604080516001600160a01b0393841681529290911660208301526103e89082015260056060820152606460808201819052600a60a0830152600360c083015260e0820152610100016064565b5f546050906001600160a01b03168156fea2646970667358221220d976986fa9df26b68453eb9d7bfd8ec83f87ee1fb8b9b555235bd54082edf5e264736f6c634300081b0033";

        let node_registry_factory = ContractFactory::new(
            serde_json::from_str(NODE_REGISTRY_ABI).expect("Invalid NODE_REGISTRY_ABI"),
            NODE_REGISTRY_BYTECODE.parse::<Bytes>().expect("Invalid NODE_REGISTRY_BYTECODE"),
            client.clone(),
        );
        
        let node_registry_contract_deployed = node_registry_factory.deploy(())?.send().await?;
        let node_registry_address = node_registry_contract_deployed.address();
        println!("Node Registry contract deployed at: {}", node_registry_address);

        let controller_factory = ContractFactory::new(
            serde_json::from_str(CONTROLLER_ABI).expect("Invalid CONTROLLER_ABI"),
            CONTROLLER_BYTECODE.parse::<Bytes>().expect("Invalid CONTROLLER_BYTECODE"),
            client.clone(),
        );
        
        let controller_contract_deployed = controller_factory.deploy(node_registry_address)?.send().await?;
        let controller_address = controller_contract_deployed.address();
        println!("Controller contract deployed at: {}", controller_address);

        let node_registry = MockNodeRegistry::new(node_registry_address, client.clone());
        
        node_registry.register_node(node_address, Bytes::from(vec![1, 2, 3]), false).send().await?.await?;
        println!("Node registered: {}", node_address);

        if is_node_active {
            node_registry.set_node_state(node_address, true).send().await?.await?;
            println!("Node state set to active");
        } else {
            println!("Node state set to inactive");
        }

        Ok((controller_address, node_registry_address))
    }
    
    async fn mock_subscribe_to_events(
        eq: &mut EventQueue,
        subscriber_name: &str,
    ) -> tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>> {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        
        struct TestSubscriber {
            name: String,
            sender: tokio::sync::mpsc::Sender<Box<dyn std::any::Any + Send>>,
        }
        
        impl std::fmt::Debug for TestSubscriber {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "TestSubscriber({})", self.name)
            }
        }
        
        #[async_trait]
        impl Subscriber for TestSubscriber {
            async fn notify(&self, _topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()> {
                println!("TestSubscriber received notification");
                if let Some(event) = payload.as_any().downcast_ref::<NodeActivation>() {
                    println!("Received NodeActivation event");
                    let cloned_event = NodeActivation {
                        chain_id: event.chain_id,
                        is_eigenlayer: event.is_eigenlayer,
                        node_registry_address: event.node_registry_address,
                    };
                    let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                    self.sender.send(boxed).await.map_err(|e| {
                        println!("Failed to send event: {}", e);
                        let err: crate::error::NodeError = anyhow!("Failed to send event: {}", e).into();
                        err
                    })?;
                    println!("Event sent to receiver");
                } else {
                    println!("Payload is not a NodeActivation event");
                }
                Ok(())
            }
            
            async fn subscribe(self) {
                println!("TestSubscriber subscribed");
            }
        }
        
        impl DebuggableSubscriber for TestSubscriber {}
        
        let subscriber = TestSubscriber {
            name: subscriber_name.to_string(),
            sender,
        };

        let topic = Topic::NodeActivation;
        println!("Subscribing to topic: {:?}", topic);
        
        eq.subscribe(topic, Box::new(subscriber));
        println!("Subscribed to event queue");
        
        receiver
    }
    
    #[tokio::test]
    async fn test_node_activation_listener() -> NodeResult<()> {
        println!("Starting test_node_activation_listener");
        
        let anvil = Anvil::new().spawn();
        println!("Anvil instance started at {}", anvil.endpoint());
        
        let http_provider = Provider::<Http>::try_from(anvil.endpoint())
        .map_err(|e| anyhow!("Failed to create HTTP provider: {}", e))?;
        
        let ws_provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await?);
        println!("Connected to Anvil WebSocket at {}", anvil.ws_endpoint());
        
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let id_address = wallet.address();
        println!("Using wallet address: {}", id_address);
        
        let chain_id = anvil.chain_id() as usize;
        println!("Chain ID: {}", chain_id);
        
        let client = Arc::new(SignerMiddleware::new(
            http_provider,
            wallet.clone().with_chain_id(anvil.chain_id()),
        ));
        
        let (controller_address, node_registry_address) = deploy_mock_contracts(
            client.clone(), 
            id_address,
            false,
        ).await.map_err(|e| anyhow!("Failed to deploy mock contracts: {}", e))?;
        
        println!("Controller deployed at: {}", controller_address);
        println!("Node Registry deployed at: {}", node_registry_address);
        
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
        println!("Chain identity created");
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        println!("Event queue created");
        
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            println!("Setting up test subscriber");
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };
        
        let is_eigenlayer = false;
        let mut listener = NodeActivationListener::<G2Curve>::new(
            chain_id,
            is_eigenlayer,
            chain_identity_arc.clone(),
            event_queue.clone(),
        );
        println!("Listener created");
        
        println!("Testing initialize method");
        let init_result = listener.initialize().await;
        assert!(init_result.is_ok(), "Initialize method should not fail");
        assert!(listener.node_registry_address.is_some(), "Node registry address should be set after initialization");
        assert_eq!(listener.node_registry_address.unwrap(), node_registry_address, "Node registry address does not match");
        
        println!("Testing direct publishing through listener");
        listener.publish(NodeActivation {
            chain_id,
            is_eigenlayer,
            node_registry_address,
        }).await;
        
        println!("Waiting for published event...");
        let received_event = timeout(Duration::from_secs(5), event_receiver.recv()).await
            .map_err(|_| {
                println!("Timeout: No event received after 5 seconds");
                anyhow!("Timeout: No event received")
            })?
            .ok_or_else(|| {
                println!("Error: Event channel closed");
                anyhow!("Error: Event channel closed")
            })?;
        
        println!("Event received!");
        
        if let Some(event) = received_event.downcast_ref::<NodeActivation>() {
            println!("Received NodeActivation event");
            assert_eq!(event.chain_id, chain_id);
            assert_eq!(event.is_eigenlayer, is_eigenlayer);
            assert_eq!(event.node_registry_address, node_registry_address);
        } else {
            println!("Received unexpected event type");
            return Err(anyhow!("Received unexpected event type").into());
        }
        
        println!("Testing listen method - should generate event since node state is false");
        let listen_result = listener.listen().await;
        assert!(listen_result.is_ok(), "Listen method should not fail");
        
        println!("Waiting for listen-triggered event...");
        let listen_triggered_event = timeout(Duration::from_secs(5), event_receiver.recv()).await
            .map_err(|_| {
                println!("Timeout: No listen-triggered event received");
                anyhow!("Timeout: No listen-triggered event received")
            })?
            .ok_or_else(|| {
                println!("Error: Event channel closed");
                anyhow!("Error: Event channel closed")
            })?;
            
        if let Some(event) = listen_triggered_event.downcast_ref::<NodeActivation>() {
            println!("Received NodeActivation event from listen method");
            assert_eq!(event.chain_id, chain_id);
            assert_eq!(event.is_eigenlayer, is_eigenlayer);
            assert_eq!(event.node_registry_address, node_registry_address);
        } else {
            println!("Received unexpected event type from listen method");
            return Err(anyhow!("Received unexpected event type from listen method").into());
        }
        
        println!("Testing handle_interruption method");
        let interruption_result = listener.handle_interruption().await;
        assert!(interruption_result.is_ok(), "Handle interruption failed");
        
        println!("Testing chain_id method");
        assert_eq!(listener.chain_id().await, chain_id);
        
        let display_string = format!("{}", listener);
        assert_eq!(display_string, "NodeActivationListener");
        
        println!("Test completed successfully");
        Ok(())
    }
}