use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::{dkg_post_process::DKGPostProcess, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use arpa_contract_client::{
    controller::{ControllerTransactions, ControllerViews},
    controller_relayer::ControllerRelayerTransactions,
};
use arpa_core::{
    log::{build_group_related_payload, build_group_related_transaction_receipt_payload, LogType},
    ComponentTaskType, DKGStatus, Group, SubscriberType, PLACEHOLDER_ADDRESS,
};
use arpa_dal::GroupInfoHandler;
use arpa_log::*;
use async_trait::async_trait;
use ethers::types::U256;
use log::{debug, error, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PostGroupingSubscriber<PC: Curve> {
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    supported_relayed_chains: Vec<usize>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
}

impl<PC: Curve> PostGroupingSubscriber<PC> {
    pub fn new(
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        supported_relayed_chains: Vec<usize>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        PostGroupingSubscriber {
            chain_identity,
            supported_relayed_chains,
            group_cache,
            eq,
            ts,
            c: PhantomData,
        }
    }
}

#[async_trait]
pub trait DKGPostProcessHandler<PC: Curve> {
    async fn handle(
        &self,
        group_index: usize,
        group_epoch: usize,
        group: Group<PC>,
    ) -> NodeResult<()>;
}

pub struct GeneralDKGPostProcessHandler<PC: Curve> {
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    supported_relayed_chains: Vec<usize>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    c: PhantomData<PC>,
}

#[async_trait]
impl<PC: Curve + Sync + Send + 'static> DKGPostProcessHandler<PC>
    for GeneralDKGPostProcessHandler<PC>
{
    #[log_function]
    async fn handle(
        &self,
        group_index: usize,
        group_epoch: usize,
        group: Group<PC>,
    ) -> NodeResult<()> {
        if self
            .group_cache
            .write()
            .await
            .update_dkg_status(group_index, group_epoch, DKGStatus::None)
            .await?
        {
            info!(
                "DKG status updated to None for group {} epoch {}",
                group_index, group_epoch
            );

            let chain_id = self.chain_identity.read().await.get_chain_id();

            // sync up the members in the group
            if !self
                .group_cache
                .write()
                .await
                .sync_up_members(group.index, group.epoch, group.members)
                .await?
            {
                error!(
                    "{}",
                    build_group_related_payload(
                        LogType::DKGGroupingMemberMisMatch,
                        "After the DKG process, group members are not matched, attempt to run with contract records.",
                        chain_id,
                        self.group_cache.read().await.get_group()?
                    )
                );
            }

            let controller_client = self.chain_identity.read().await.build_controller_client();

            let controller_relayer_client = self
                .chain_identity
                .read()
                .await
                .build_controller_relayer_client();

            if PLACEHOLDER_ADDRESS
                != ControllerViews::<PC>::get_coordinator(&controller_client, group_index).await?
            {
                if let Ok(receipt) = controller_client
                    .post_process_dkg(group_index, group_epoch)
                    .await
                {
                    info!(
                        "{}",
                        build_group_related_transaction_receipt_payload(
                            LogType::DKGPostProcessFinished,
                            "DKG post process finished.",
                            chain_id,
                            self.group_cache.read().await.get_group()?,
                            None,
                            receipt.transaction_hash,
                            receipt.gas_used.unwrap_or(U256::zero()),
                            receipt.effective_gas_price.unwrap_or(U256::zero())
                        )
                    );
                }

                if self.group_cache.read().await.get_group()?.state {
                    for relayed_chain_id in self.supported_relayed_chains.iter() {
                        if let Ok(receipt) = controller_relayer_client
                            .relay_group(*relayed_chain_id, group_index)
                            .await
                        {
                            info!(
                                "{}",
                                build_group_related_transaction_receipt_payload(
                                    LogType::DKGPostProcessGroupRelayFinished,
                                    "DKG post process group relay finished.",
                                    chain_id,
                                    self.group_cache.read().await.get_group()?,
                                    Some(*relayed_chain_id),
                                    receipt.transaction_hash,
                                    receipt.gas_used.unwrap_or(U256::zero()),
                                    receipt.effective_gas_price.unwrap_or(U256::zero())
                                )
                            );
                        }
                    }
                }
            };
        }

        Ok(())
    }
}

#[async_trait]
impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> Subscriber
    for PostGroupingSubscriber<PC>
{
    #[log_function]
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let DKGPostProcess {
            group_index,
            group_epoch,
            group,
        } = payload
            .as_any()
            .downcast_ref::<DKGPostProcess<PC>>()
            .unwrap()
            .clone();

        let chain_identity = self.chain_identity.clone();
        let supported_relayed_chains = self.supported_relayed_chains.clone();
        let group_cache = self.group_cache.clone();

        self.ts.write().await.add_task(ComponentTaskType::Subscriber(self.chain_identity.read().await.get_chain_id(), SubscriberType::PostGrouping),async move {
                let handler = GeneralDKGPostProcessHandler {
                    chain_identity,
                    supported_relayed_chains,
                    group_cache,
                    c: PhantomData,
                };

                if let Err(e) = handler.handle(group_index, group_epoch, group).await {
                    error!("{:?}", e);
                } else {
                    info!("-------------------------call post process successfully-------------------------");
                }
            })?;

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write()
            .await
            .subscribe(Topic::DKGPostProcess, subscriber);
    }
}

impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> DebuggableSubscriber
    for PostGroupingSubscriber<PC>
{
}

#[cfg(feature = "unittest")]
mod tests {
    use super::*;
    use crate::{
        event::{dkg_post_process::DKGPostProcess, types::Topic},
        queue::event_queue::EventQueue,
        scheduler::dynamic::SimpleDynamicTaskScheduler,
        test_contracts::{
            mockcontroller::{deploy_mock_controller_with_args, get_mock_controller_at}, 
            mockcontrollerrelayer::{deploy_mock_controller_relayer, get_mock_controller_relayer_at}
        },
    };
    use arpa_core::{
        Config, DKGStatus, GeneralMainChainIdentity, Group, Member, PLACEHOLDER_ADDRESS
    };
    use crate::test_contracts::{
        mockcontroller::MockController,
        mockcontrollerrelayer::MockControllerRelayer
    };
    use arpa_dal::{
        cache::InMemoryGroupInfoCache,
        GroupInfoHandler,
    };
    use ethers::prelude::*;
    use std::{collections::BTreeMap, sync::Arc};
    use threshold_bls::schemes::bn254::G2Curve;
    use tokio::sync::RwLock;

    const DEFAULT_CHAIN_ID: u64 = 1;
    const DEFAULT_GROUP_INDEX: usize = 1;
    const DEFAULT_EPOCH: usize = 1;
    const DEFAULT_GROUP_SIZE: usize = 3;
    const DEFAULT_THRESHOLD: usize = 2;
    const DEFAULT_BLOCK_HEIGHT: usize = 100;
    const DEFAULT_RPC_ENDPOINT: &str = "http://localhost:8545";
    const DEFAULT_WS_ENDPOINT: &str = "ws://localhost:8545";
    const DEFAULT_SUPPORTED_CHAINS: [usize; 2] = [2, 3];

    async fn setup_anvil() -> (ethers::utils::AnvilInstance, Arc<Provider<Ws>>, LocalWallet) {
        let anvil = ethers::utils::Anvil::new().spawn();
        let ws_provider = Arc::new(
            Provider::<Ws>::connect(anvil.ws_endpoint())
                .await
                .expect("Failed to connect to anvil")
        );
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let wallet = wallet.with_chain_id(anvil.chain_id());
        (anvil, ws_provider, wallet)
    }

    async fn deploy_controller(
        ws_provider: Arc<Provider<Ws>>, 
        wallet: LocalWallet
    ) -> (Address, MockController<SignerMiddleware<Provider<Ws>, LocalWallet>>) {
        let client = Arc::new(SignerMiddleware::new((*ws_provider).clone(), wallet.clone()));
        let controller_address = deploy_mock_controller_with_args(client.clone(), Address::random()).await.unwrap();
        let controller_contract = get_mock_controller_at(controller_address, client.clone());
        (controller_address, controller_contract)
    }

    async fn deploy_controller_relayer(
        ws_provider: Arc<Provider<Ws>>, 
        wallet: LocalWallet
    ) -> (Address, MockControllerRelayer<SignerMiddleware<Provider<Ws>, LocalWallet>>) {
        let client = Arc::new(SignerMiddleware::new((*ws_provider).clone(), wallet.clone()));
        let relayer_address = deploy_mock_controller_relayer(client.clone()).await.unwrap();
        let relayer_contract = get_mock_controller_relayer_at(relayer_address, client.clone());
        (relayer_address, relayer_contract)
    }

    async fn create_chain_identity(
        chain_id: u64,
        wallet: LocalWallet,
        ws_provider: Arc<Provider<Ws>>,
        ws_endpoint: String,
        controller_address: Address,
        relayer_address: Address,
    ) -> Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> {
        let config = Config::default();
        let general_chain_identity = GeneralMainChainIdentity::new(
            chain_id.try_into().unwrap(),
            wallet.clone(),
            ws_provider.clone(),
            ws_endpoint,
            controller_address,
            relayer_address,
            Address::random(), 
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );
        Arc::new(RwLock::new(Box::new(general_chain_identity)))
    }

    async fn setup_group_cache(
        id_address: Address,
        group_index: usize,
        epoch: usize,
        size: usize,
        threshold: usize,
        member_addresses: Vec<Address>,
        dkg_status: DKGStatus,
        state: bool,
    ) -> NodeResult<Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>>> {
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
                assignment_block_height: DEFAULT_BLOCK_HEIGHT,
                members: member_addresses.clone(),
                coordinator_address: Address::random(),
            };

            group_cache_write.save_task_info(0, dkg_task).await?;
            group_cache_write.update_dkg_status(group_index, epoch, dkg_status).await?;

            let mut group = Group::<G2Curve> {
                index: group_index,
                epoch,
                size,
                threshold,
                state,
                public_key: None,
                members: BTreeMap::new(),
                committers: vec![],
                c: std::marker::PhantomData,
            };

            for (i, addr) in member_addresses.iter().enumerate() {
                group.members.insert(*addr, Member {
                    index: i,
                    dkg_index: Some(i),
                    id_address: *addr,
                    rpc_endpoint: Some(DEFAULT_RPC_ENDPOINT.to_string()),
                    partial_public_key: None,
                });
            }

            group_cache_write.sync_up_members(group_index, epoch, group.members).await?;
        }

        Ok(group_cache)
    }

    fn create_schedulers() -> (Arc<RwLock<EventQueue>>, Arc<RwLock<SimpleDynamicTaskScheduler>>) {
        (
            Arc::new(RwLock::new(EventQueue::new())),
            Arc::new(RwLock::new(SimpleDynamicTaskScheduler::new()))
        )
    }

    fn create_test_group(
        group_index: usize, 
        epoch: usize, 
        state: bool, 
        member_addresses: Vec<Address>
    ) -> Group<G2Curve> {
        let mut members = BTreeMap::new();
        for (i, addr) in member_addresses.iter().enumerate() {
            members.insert(*addr, Member {
                index: i,
                dkg_index: Some(i),
                id_address: *addr,
                rpc_endpoint: Some(DEFAULT_RPC_ENDPOINT.to_string()),
                partial_public_key: None,
            });
        }

        Group {
            index: group_index,
            epoch,
            size: member_addresses.len(),
            threshold: (member_addresses.len() * 2 / 3) + 1,
            state,
            public_key: None,
            members,
            committers: member_addresses,
            c: std::marker::PhantomData,
        }
    }

    async fn create_test_setup() -> (
        Arc<RwLock<ChainIdentityHandlerType<G2Curve>>>,
        Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>>,
        Address,
    ) {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let (controller_address, _controller) = deploy_controller(ws_provider.clone(), wallet.clone()).await;
        let (relayer_address, _relayer) = deploy_controller_relayer(ws_provider.clone(), wallet.clone()).await;
        
        let chain_identity = create_chain_identity(
            DEFAULT_CHAIN_ID,
            wallet,
            ws_provider,
            DEFAULT_WS_ENDPOINT.to_string(),
            controller_address,
            relayer_address,
        ).await;

        let id_address = Address::random();
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::random(), Address::random()],
            DKGStatus::WaitForPostProcess,
            true,
        ).await.unwrap();

        (chain_identity, group_cache, id_address)
    }

    #[tokio::test]
    async fn test_post_grouping_subscriber_creation() {
        let (chain_identity, group_cache, _id_address) = create_test_setup().await;
        let (eq, ts) = create_schedulers();
        let supported_chains = DEFAULT_SUPPORTED_CHAINS.to_vec();

        let subscriber = PostGroupingSubscriber::new(
            chain_identity,
            supported_chains.clone(),
            group_cache,
            eq,
            ts,
        );

        assert_eq!(subscriber.supported_relayed_chains, supported_chains);
    }

    #[tokio::test]
    async fn test_dkg_post_process_handler_creation() {
        let (chain_identity, group_cache, _id_address) = create_test_setup().await;
        let supported_chains = DEFAULT_SUPPORTED_CHAINS.to_vec();

        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: supported_chains.clone(),
            group_cache,
            c: PhantomData,
        };

        assert_eq!(handler.supported_relayed_chains, supported_chains);
    }

    #[tokio::test]
    async fn test_post_process_handler_with_no_coordinator() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let (controller_address, controller) = deploy_controller(ws_provider.clone(), wallet.clone()).await;
        let (relayer_address, _relayer) = deploy_controller_relayer(ws_provider.clone(), wallet.clone()).await;
        
        controller.set_coordinator(U256::from(DEFAULT_GROUP_INDEX), PLACEHOLDER_ADDRESS).send().await.unwrap().await.unwrap();

        let chain_identity = create_chain_identity(
            DEFAULT_CHAIN_ID,
            wallet,
            ws_provider,
            DEFAULT_WS_ENDPOINT.to_string(),
            controller_address,
            relayer_address,
        ).await;

        let id_address = Address::random();
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::random(), Address::random()],
            DKGStatus::WaitForPostProcess,
            true,
        ).await.unwrap();

        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: DEFAULT_SUPPORTED_CHAINS.to_vec(),
            group_cache,
            c: PhantomData,
        };

        let group = create_test_group(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, true, vec![id_address, Address::random(), Address::random()]);
        let result = handler.handle(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, group).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_process_handler_with_coordinator_success() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let (controller_address, controller) = deploy_controller(ws_provider.clone(), wallet.clone()).await;
        let (relayer_address, relayer) = deploy_controller_relayer(ws_provider.clone(), wallet.clone()).await;
        
        let coordinator_addr = Address::random();
        controller.set_coordinator(U256::from(DEFAULT_GROUP_INDEX), coordinator_addr).send().await.unwrap().await.unwrap();
        controller.set_should_succeed(true).send().await.unwrap().await.unwrap();
        relayer.set_should_succeed(true).send().await.unwrap().await.unwrap();

        let chain_identity = create_chain_identity(
            DEFAULT_CHAIN_ID,
            wallet,
            ws_provider,
            DEFAULT_WS_ENDPOINT.to_string(),
            controller_address,
            relayer_address,
        ).await;

        let id_address = Address::random();
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::random(), Address::random()],
            DKGStatus::WaitForPostProcess,
            true,
        ).await.unwrap();

        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: DEFAULT_SUPPORTED_CHAINS.to_vec(),
            group_cache,
            c: PhantomData,
        };

        let group = create_test_group(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, true, vec![id_address, Address::random(), Address::random()]);
        let result = handler.handle(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, group).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_process_handler_with_inactive_group() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let (controller_address, controller) = deploy_controller(ws_provider.clone(), wallet.clone()).await;
        let (relayer_address, relayer) = deploy_controller_relayer(ws_provider.clone(), wallet.clone()).await;
        
        let coordinator_addr = Address::random();
        controller.set_coordinator(U256::from(DEFAULT_GROUP_INDEX), coordinator_addr).send().await.unwrap().await.unwrap();
        controller.set_should_succeed(true).send().await.unwrap().await.unwrap();
        relayer.set_should_succeed(true).send().await.unwrap().await.unwrap();

        let chain_identity = create_chain_identity(
            DEFAULT_CHAIN_ID,
            wallet,
            ws_provider,
            DEFAULT_WS_ENDPOINT.to_string(),
            controller_address,
            relayer_address,
        ).await;

        let id_address = Address::random();
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::random(), Address::random()],
            DKGStatus::WaitForPostProcess,
            false,
        ).await.unwrap();

        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: DEFAULT_SUPPORTED_CHAINS.to_vec(),
            group_cache,
            c: PhantomData,
        };

        let group = create_test_group(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, false, vec![id_address, Address::random(), Address::random()]);
        let result = handler.handle(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, group).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_process_handler_with_contract_failure() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let (controller_address, controller) = deploy_controller(ws_provider.clone(), wallet.clone()).await;
        let (relayer_address, relayer) = deploy_controller_relayer(ws_provider.clone(), wallet.clone()).await;
        
        let coordinator_addr = Address::random();
        controller.set_coordinator(U256::from(DEFAULT_GROUP_INDEX), coordinator_addr).send().await.unwrap().await.unwrap();
        controller.set_should_succeed(false).send().await.unwrap().await.unwrap();
        controller.set_failure_message("Controller failure".to_string()).send().await.unwrap().await.unwrap();
        relayer.set_should_succeed(false).send().await.unwrap().await.unwrap();
        relayer.set_failure_message("Relayer failure".to_string()).send().await.unwrap().await.unwrap();

        let chain_identity = create_chain_identity(
            DEFAULT_CHAIN_ID,
            wallet,
            ws_provider,
            DEFAULT_WS_ENDPOINT.to_string(),
            controller_address,
            relayer_address,
        ).await;

        let id_address = Address::random();
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::random(), Address::random()],
            DKGStatus::WaitForPostProcess,
            true,
        ).await.unwrap();

        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: DEFAULT_SUPPORTED_CHAINS.to_vec(),
            group_cache,
            c: PhantomData,
        };

        let group = create_test_group(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, true, vec![id_address, Address::random(), Address::random()]);
        let result = handler.handle(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, group).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscriber_notify() {
        let (chain_identity, group_cache, id_address) = create_test_setup().await;
        let (eq, ts) = create_schedulers();

        let subscriber = PostGroupingSubscriber::new(
            chain_identity,
            DEFAULT_SUPPORTED_CHAINS.to_vec(),
            group_cache,
            eq,
            ts,
        );

        let group = create_test_group(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, true, vec![id_address, Address::random(), Address::random()]);
        let post_process_event = DKGPostProcess {
            group_index: DEFAULT_GROUP_INDEX,
            group_epoch: DEFAULT_EPOCH,
            group,
        };

        let result = subscriber.notify(Topic::DKGPostProcess, &post_process_event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscriber_subscribe() {
        let id_address = Address::random();
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::random(), Address::random()],
            DKGStatus::WaitForPostProcess,
            true,
        ).await.unwrap();

        let (eq, ts) = create_schedulers();
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let chain_identity = create_chain_identity(
            DEFAULT_CHAIN_ID,
            wallet,
            ws_provider,
            DEFAULT_WS_ENDPOINT.to_string(),
            Address::random(),
            Address::random(),
        ).await;

        let subscriber = PostGroupingSubscriber::new(
            chain_identity,
            DEFAULT_SUPPORTED_CHAINS.to_vec(),
            group_cache,
            eq.clone(),
            ts,
        );

        subscriber.subscribe().await;
    }

    #[tokio::test]
    async fn test_multiple_relayed_chains() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let (controller_address, controller) = deploy_controller(ws_provider.clone(), wallet.clone()).await;
        let (relayer_address, relayer) = deploy_controller_relayer(ws_provider.clone(), wallet.clone()).await;
        
        let coordinator_addr = Address::random();
        controller.set_coordinator(U256::from(DEFAULT_GROUP_INDEX), coordinator_addr).send().await.unwrap().await.unwrap();
        controller.set_should_succeed(true).send().await.unwrap().await.unwrap();
        relayer.set_should_succeed(true).send().await.unwrap().await.unwrap();

        let chain_identity = create_chain_identity(
            DEFAULT_CHAIN_ID,
            wallet,
            ws_provider,
            DEFAULT_WS_ENDPOINT.to_string(),
            controller_address,
            relayer_address,
        ).await;

        let id_address = Address::random();
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::random(), Address::random()],
            DKGStatus::WaitForPostProcess,
            true,
        ).await.unwrap();

        let multiple_chains = vec![2, 3, 4, 5, 6];
        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: multiple_chains.clone(),
            group_cache,
            c: PhantomData,
        };

        let group = create_test_group(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, true, vec![id_address, Address::random(), Address::random()]);
        let result = handler.handle(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, group).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dkg_status_update_failure() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let (controller_address, _controller) = deploy_controller(ws_provider.clone(), wallet.clone()).await;
        let (relayer_address, _relayer) = deploy_controller_relayer(ws_provider.clone(), wallet.clone()).await;
        
        let chain_identity = create_chain_identity(
            DEFAULT_CHAIN_ID,
            wallet,
            ws_provider,
            DEFAULT_WS_ENDPOINT.to_string(),
            controller_address,
            relayer_address,
        ).await;

        let id_address = Address::random();
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::random(), Address::random()],
            DKGStatus::None,
            true,
        ).await.unwrap();

        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: DEFAULT_SUPPORTED_CHAINS.to_vec(),
            group_cache,
            c: PhantomData,
        };

        let group = create_test_group(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, true, vec![id_address, Address::random(), Address::random()]);
        let result = handler.handle(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, group).await;
        assert!(result.is_ok());
    }
}