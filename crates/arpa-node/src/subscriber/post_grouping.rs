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
use log::{debug, error, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PostGroupingSubscriber<PC: Curve> {
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    supported_relayed_chains: Vec<u64>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
}

impl<PC: Curve> PostGroupingSubscriber<PC> {
    pub fn new(
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        supported_relayed_chains: Vec<u64>,
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
    supported_relayed_chains: Vec<u64>,
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
                            receipt.gas_used,
                            receipt.effective_gas_price
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
                                    receipt.gas_used,
                                    receipt.effective_gas_price
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
    async fn notify(&self, topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::{dkg_post_process::DKGPostProcess, types::Topic},
        queue::event_queue::EventQueue,
        scheduler::dynamic::SimpleDynamicTaskScheduler,
    };
    use alloy::node_bindings::{Anvil, AnvilInstance};
    use alloy::primitives::{Address, U256};
    use alloy::providers::WsConnect;
    use alloy::signers::local::PrivateKeySigner;
    use alloy::sol;
    use arpa_core::{
        build_client, Config, DKGStatus, GeneralMainChainIdentity, Group, Member, 
        ProviderClientWithSigner, PLACEHOLDER_ADDRESS
    };
    use arpa_dal::{cache::InMemoryGroupInfoCache, GroupInfoHandler};
    use std::{collections::BTreeMap, sync::Arc};
    use threshold_bls::schemes::bn254::G2Curve;
    use tokio::sync::RwLock;

    const DEFAULT_GROUP_INDEX: usize = 1;
    const DEFAULT_EPOCH: usize = 1;
    const DEFAULT_GROUP_SIZE: usize = 3;
    const DEFAULT_THRESHOLD: usize = 2;
    const DEFAULT_BLOCK_HEIGHT: usize = 100;
    const DEFAULT_RPC_ENDPOINT: &str = "http://localhost:8545";
    const DEFAULT_SUPPORTED_CHAINS: [u64; 2] = [2, 3];

    sol! {
        #[sol(ignore_unlinked)]
        #[sol(rpc)]
        MockController,
        "test-contract/MockController.json"
    }

    sol! {
        #[sol(ignore_unlinked)]
        #[sol(rpc)]
        MockControllerRelayer,
        "test-contract/MockControllerRelayer.json"
    }

    struct TestEnvironment {
        _anvil: AnvilInstance,
        client: ProviderClientWithSigner,
        wallet: PrivateKeySigner,
        chain_id: u64,
        ws_endpoint: String,
    }

    impl TestEnvironment {
        async fn new() -> Self {
            let anvil = Anvil::new().spawn();
            let ws_endpoint = anvil.ws_endpoint();
            let ws_connect = WsConnect::new(&ws_endpoint);
            let wallet: PrivateKeySigner = anvil.keys()[0].clone().into();
            let chain_id = anvil.chain_id();
            let client = build_client(wallet.clone(), chain_id, ws_connect)
                .await
                .unwrap();

            TestEnvironment {
                _anvil: anvil,
                client,
                wallet,
                chain_id,
                ws_endpoint,
            }
        }

        async fn deploy_mock_controller(&self) -> (Address, MockController::MockControllerInstance<ProviderClientWithSigner>) {
            let contract = MockController::deploy(self.client.clone(), Address::ZERO)
                .await
                .unwrap();
            (*contract.address(), contract)
        }

        async fn deploy_mock_controller_relayer(&self) -> (Address, MockControllerRelayer::MockControllerRelayerInstance<ProviderClientWithSigner>) {
            let contract = MockControllerRelayer::deploy(self.client.clone())
                .await
                .unwrap();
            (*contract.address(), contract)
        }

        async fn create_chain_identity(
            &self,
            controller_address: Address,
            relayer_address: Address,
        ) -> Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> {
            let config = Config::default();
            let ws_connect = WsConnect::new(&self.ws_endpoint);
            let general_chain_identity = GeneralMainChainIdentity::new(
                self.chain_id.try_into().unwrap(),
                self.wallet.clone(),
                ws_connect,
                self.client.clone(),
                self.ws_endpoint.clone(),
                controller_address,
                relayer_address,
                Address::ZERO,
                config
                    .get_time_limits()
                    .contract_transaction_retry_descriptor,
                config.get_time_limits().contract_view_retry_descriptor,
                None,
            );
            Arc::new(RwLock::new(Box::new(general_chain_identity)))
        }
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
                coordinator_address: Address::ZERO,
            };

            group_cache_write.save_task_info(0, dkg_task).await?;
            group_cache_write
                .update_dkg_status(group_index, epoch, dkg_status)
                .await?;

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
                group.members.insert(
                    *addr,
                    Member {
                        index: i,
                        dkg_index: Some(i),
                        id_address: *addr,
                        rpc_endpoint: Some(DEFAULT_RPC_ENDPOINT.to_string()),
                        partial_public_key: None,
                    },
                );
            }

            group_cache_write
                .sync_up_members(group_index, epoch, group.members)
                .await?;
        }

        Ok(group_cache)
    }

    fn create_schedulers() -> (
        Arc<RwLock<EventQueue>>,
        Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) {
        (
            Arc::new(RwLock::new(EventQueue::new())),
            Arc::new(RwLock::new(SimpleDynamicTaskScheduler::new())),
        )
    }

    fn create_test_group(
        group_index: usize,
        epoch: usize,
        state: bool,
        member_addresses: Vec<Address>,
    ) -> Group<G2Curve> {
        let mut members = BTreeMap::new();
        for (i, addr) in member_addresses.iter().enumerate() {
            members.insert(
                *addr,
                Member {
                    index: i,
                    dkg_index: Some(i),
                    id_address: *addr,
                    rpc_endpoint: Some(DEFAULT_RPC_ENDPOINT.to_string()),
                    partial_public_key: None,
                },
            );
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
        let env = TestEnvironment::new().await;
        let (controller_address, _controller) = env.deploy_mock_controller().await;
        let (relayer_address, _relayer) = env.deploy_mock_controller_relayer().await;

        let chain_identity = env
            .create_chain_identity(controller_address, relayer_address)
            .await;

        let id_address = Address::ZERO;
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::ZERO, Address::ZERO],
            DKGStatus::WaitForPostProcess,
            true,
        )
        .await
        .unwrap();

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
        let env = TestEnvironment::new().await;
        let (controller_address, controller) = env.deploy_mock_controller().await;
        let (relayer_address, _relayer) = env.deploy_mock_controller_relayer().await;

        controller
            .setCoordinator(U256::from(DEFAULT_GROUP_INDEX), PLACEHOLDER_ADDRESS)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        let chain_identity = env
            .create_chain_identity(controller_address, relayer_address)
            .await;

        let id_address = Address::ZERO;
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::ZERO, Address::ZERO],
            DKGStatus::WaitForPostProcess,
            true,
        )
        .await
        .unwrap();

        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: DEFAULT_SUPPORTED_CHAINS.to_vec(),
            group_cache,
            c: PhantomData,
        };

        let group = create_test_group(
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            true,
            vec![id_address, Address::ZERO, Address::ZERO],
        );
        let result = handler
            .handle(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, group)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_process_handler_with_coordinator_success() {
        let env = TestEnvironment::new().await;
        let (controller_address, controller) = env.deploy_mock_controller().await;
        let (relayer_address, relayer) = env.deploy_mock_controller_relayer().await;

        let coordinator_addr = Address::ZERO;
        controller
            .setCoordinator(U256::from(DEFAULT_GROUP_INDEX), coordinator_addr)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        controller
            .setShouldSucceed(true)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        relayer
            .setShouldSucceed(true)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        let chain_identity = env
            .create_chain_identity(controller_address, relayer_address)
            .await;

        let id_address = Address::ZERO;
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::ZERO, Address::ZERO],
            DKGStatus::WaitForPostProcess,
            true,
        )
        .await
        .unwrap();

        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: DEFAULT_SUPPORTED_CHAINS.to_vec(),
            group_cache,
            c: PhantomData,
        };

        let group = create_test_group(
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            true,
            vec![id_address, Address::ZERO, Address::ZERO],
        );
        let result = handler
            .handle(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, group)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_process_handler_with_inactive_group() {
        let env = TestEnvironment::new().await;
        let (controller_address, controller) = env.deploy_mock_controller().await;
        let (relayer_address, relayer) = env.deploy_mock_controller_relayer().await;

        let coordinator_addr = Address::ZERO;
        controller
            .setCoordinator(U256::from(DEFAULT_GROUP_INDEX), coordinator_addr)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        controller
            .setShouldSucceed(true)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        relayer
            .setShouldSucceed(true)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        let chain_identity = env
            .create_chain_identity(controller_address, relayer_address)
            .await;

        let id_address = Address::ZERO;
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::ZERO, Address::ZERO],
            DKGStatus::WaitForPostProcess,
            false,
        )
        .await
        .unwrap();

        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: DEFAULT_SUPPORTED_CHAINS.to_vec(),
            group_cache,
            c: PhantomData,
        };

        let group = create_test_group(
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            false,
            vec![id_address, Address::ZERO, Address::ZERO],
        );
        let result = handler
            .handle(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, group)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_process_handler_with_contract_failure() {
        let env = TestEnvironment::new().await;
        let (controller_address, controller) = env.deploy_mock_controller().await;
        let (relayer_address, relayer) = env.deploy_mock_controller_relayer().await;

        let coordinator_addr = Address::ZERO;
        controller
            .setCoordinator(U256::from(DEFAULT_GROUP_INDEX), coordinator_addr)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        controller
            .setShouldSucceed(false)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        controller
            .setFailureMessage("Controller failure".to_string())
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        relayer
            .setShouldSucceed(false)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        relayer
            .setFailureMessage("Relayer failure".to_string())
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        let chain_identity = env
            .create_chain_identity(controller_address, relayer_address)
            .await;

        let id_address = Address::ZERO;
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::ZERO, Address::ZERO],
            DKGStatus::WaitForPostProcess,
            true,
        )
        .await
        .unwrap();

        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: DEFAULT_SUPPORTED_CHAINS.to_vec(),
            group_cache,
            c: PhantomData,
        };

        let group = create_test_group(
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            true,
            vec![id_address, Address::ZERO, Address::ZERO],
        );
        let result = handler
            .handle(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, group)
            .await;
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

        let group = create_test_group(
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            true,
            vec![id_address, Address::ZERO, Address::ZERO],
        );
        let post_process_event = DKGPostProcess {
            group_index: DEFAULT_GROUP_INDEX,
            group_epoch: DEFAULT_EPOCH,
            group,
        };

        let result = subscriber
            .notify(Topic::DKGPostProcess, &post_process_event)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscriber_subscribe() {
        let id_address = Address::ZERO;
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::ZERO, Address::ZERO],
            DKGStatus::WaitForPostProcess,
            true,
        )
        .await
        .unwrap();

        let (eq, ts) = create_schedulers();
        let env = TestEnvironment::new().await;
        let chain_identity = env
            .create_chain_identity(Address::ZERO, Address::ZERO)
            .await;

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
        let env = TestEnvironment::new().await;
        let (controller_address, controller) = env.deploy_mock_controller().await;
        let (relayer_address, relayer) = env.deploy_mock_controller_relayer().await;

        let coordinator_addr = Address::ZERO;
        controller
            .setCoordinator(U256::from(DEFAULT_GROUP_INDEX), coordinator_addr)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        controller
            .setShouldSucceed(true)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        relayer
            .setShouldSucceed(true)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        let chain_identity = env
            .create_chain_identity(controller_address, relayer_address)
            .await;

        let id_address = Address::ZERO;
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::ZERO, Address::ZERO],
            DKGStatus::WaitForPostProcess,
            true,
        )
        .await
        .unwrap();

        let multiple_chains = vec![2, 3, 4, 5, 6];
        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: multiple_chains.clone(),
            group_cache,
            c: PhantomData,
        };

        let group = create_test_group(
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            true,
            vec![id_address, Address::ZERO, Address::ZERO],
        );
        let result = handler
            .handle(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, group)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dkg_status_update_failure() {
        let env = TestEnvironment::new().await;
        let (controller_address, _controller) = env.deploy_mock_controller().await;
        let (relayer_address, _relayer) = env.deploy_mock_controller_relayer().await;

        let chain_identity = env
            .create_chain_identity(controller_address, relayer_address)
            .await;

        let id_address = Address::ZERO;
        let group_cache = setup_group_cache(
            id_address,
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::ZERO, Address::ZERO],
            DKGStatus::None,
            true,
        )
        .await
        .unwrap();

        let handler = GeneralDKGPostProcessHandler {
            chain_identity,
            supported_relayed_chains: DEFAULT_SUPPORTED_CHAINS.to_vec(),
            group_cache,
            c: PhantomData,
        };

        let group = create_test_group(
            DEFAULT_GROUP_INDEX,
            DEFAULT_EPOCH,
            true,
            vec![id_address, Address::ZERO, Address::ZERO],
        );
        let result = handler
            .handle(DEFAULT_GROUP_INDEX, DEFAULT_EPOCH, group)
            .await;
        assert!(result.is_ok());
    }
}