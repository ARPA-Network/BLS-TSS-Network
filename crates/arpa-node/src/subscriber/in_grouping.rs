use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    algorithm::dkg::{AllPhasesDKGCore, DKGCore},
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::{run_dkg::RunDKG, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, DynamicTaskScheduler},
};
use arpa_contract_client::{controller::ControllerTransactions, error::ContractClientError};
use arpa_core::{
    log::{build_group_related_payload, build_group_related_transaction_receipt_payload, LogType},
    DKGStatus, DKGTask,
};
use arpa_dal::{GroupInfoHandler, NodeInfoHandler};
use async_trait::async_trait;
use core::fmt::Debug;
use log::{debug, error, info};
use rand::{prelude::ThreadRng, RngCore};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct InGroupingSubscriber<PC: Curve> {
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<PC>>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
    dkg_wait_for_phase_interval_millis: u64,
}

impl<PC: Curve> InGroupingSubscriber<PC> {
    pub fn new(
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<PC>>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
        dkg_wait_for_phase_interval_millis: u64,
    ) -> Self {
        InGroupingSubscriber {
            chain_identity,
            node_cache,
            group_cache,
            eq,
            ts,
            c: PhantomData,
            dkg_wait_for_phase_interval_millis,
        }
    }
}

pub struct AllInOneDKGHandler<F: Fn() -> R, R: RngCore, PC: Curve> {
    rng: F,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<PC>>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    c: PhantomData<PC>,
    dkg_wait_for_phase_interval_millis: u64,
}

impl<F: Fn() -> R, R: RngCore, PC: Curve> AllInOneDKGHandler<F, R, PC> {
    pub fn new(
        rng: F,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<PC>>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        dkg_wait_for_phase_interval_millis: u64,
    ) -> Self {
        AllInOneDKGHandler {
            rng,
            chain_identity,
            node_cache,
            group_cache,
            c: PhantomData,
            dkg_wait_for_phase_interval_millis,
        }
    }
}

#[async_trait]
pub trait DKGHandler<F, R> {
    async fn handle(&mut self, task: DKGTask) -> NodeResult<()>
    where
        R: RngCore,
        F: Fn() -> R + 'static;
}

#[async_trait]
impl<
        F: Fn() -> R + Debug + Send + Sync + Copy + 'static,
        R: RngCore + 'static,
        PC: Curve + Sync + Send + 'static,
    > DKGHandler<F, R> for AllInOneDKGHandler<F, R, PC>
{
    async fn handle(&mut self, task: DKGTask) -> NodeResult<()>
    where
        R: RngCore,
        F: Fn() -> R + Send + Debug + 'async_trait,
    {
        let node_rpc_endpoint = self
            .node_cache
            .read()
            .await
            .get_node_rpc_endpoint()?
            .to_string();

        let chain_id = self.chain_identity.read().await.get_chain_id();

        let controller_client = self.chain_identity.read().await.build_controller_client();

        let dkg_private_key = self.node_cache.read().await.get_dkg_private_key()?.clone();

        let task_group_index = task.group_index;

        let task_epoch = task.epoch;

        let coordinator_client = self
            .chain_identity
            .read()
            .await
            .build_coordinator_client(task.coordinator_address);

        let mut dkg_core =
            AllPhasesDKGCore::new(coordinator_client, self.dkg_wait_for_phase_interval_millis);

        match dkg_core
            .run_dkg(dkg_private_key, node_rpc_endpoint, self.rng)
            .await
        {
            Ok(output) => match output.disqualified_node_indices.len() {
                0 => {
                    let (public_key, partial_public_key, disqualified_nodes) = self
                        .group_cache
                        .write()
                        .await
                        .save_successful_output(task_group_index, task_epoch, output)
                        .await?;

                    info!(
                        "{}",
                        build_group_related_payload(
                            LogType::DKGGroupingFinished,
                            "DKG grouping finished.",
                            chain_id,
                            self.group_cache.read().await.get_group()?
                        )
                    );

                    match controller_client
                        .commit_dkg(
                            task_group_index,
                            task_epoch,
                            bincode::serialize(&public_key).unwrap(),
                            bincode::serialize(&partial_public_key).unwrap(),
                            disqualified_nodes,
                        )
                        .await
                    {
                        Ok(receipt) => {
                            info!(
                                "{}",
                                build_group_related_transaction_receipt_payload(
                                    LogType::DKGGroupingCommitted,
                                    "DKG grouping result committed.",
                                    chain_id,
                                    self.group_cache.read().await.get_group()?,
                                    None,
                                    receipt.transaction_hash,
                                    receipt.gas_used,
                                    receipt.effective_gas_price
                                )
                            );
                        }
                        Err(e) => match e {
                            ContractClientError::TransactionFailed(receipt) => {
                                error!(
                                    "{}",
                                    build_group_related_transaction_receipt_payload(
                                        LogType::DKGGroupingCommitFailed,
                                        "DKG grouping commit failed.",
                                        chain_id,
                                        self.group_cache.read().await.get_group()?,
                                        None,
                                        receipt.transaction_hash,
                                        receipt.gas_used,
                                        receipt.effective_gas_price
                                    )
                                );
                            }
                            _ => {
                                error!(
                                    "{}",
                                    build_group_related_payload(
                                        LogType::DKGGroupingCommitFailed,
                                        &format!("DKG grouping commit failed with error: {:?}", e),
                                        chain_id,
                                        self.group_cache.read().await.get_group()?
                                    )
                                );
                            }
                        },
                    }
                }
                _ => {
                    info!(
                        "Disqualified node indices: {:?}",
                        output.disqualified_node_indices
                    );

                    let disqualified_nodes = self
                        .group_cache
                        .write()
                        .await
                        .save_failed_output(
                            task_group_index,
                            task_epoch,
                            output.disqualified_node_indices,
                        )
                        .await?;

                    info!("Disqualified node addresses: {:?}", disqualified_nodes);

                    info!(
                        "{}",
                        build_group_related_payload(
                            LogType::DKGGroupingAborted,
                            "DKG grouping aborted due to disqualified nodes.",
                            chain_id,
                            self.group_cache.read().await.get_group()?
                        )
                    );

                    let g_public_key = PC::point();
                    let g_partial_public_key = PC::point();

                    match controller_client
                        .commit_dkg(
                            task_group_index,
                            task_epoch,
                            bincode::serialize(&g_public_key).unwrap(),
                            bincode::serialize(&g_partial_public_key).unwrap(),
                            disqualified_nodes,
                        )
                        .await
                    {
                        Ok(receipt) => {
                            info!(
                                "{}",
                                build_group_related_transaction_receipt_payload(
                                    LogType::DKGGroupingCommitted,
                                    "DKG grouping result committed.",
                                    chain_id,
                                    self.group_cache.read().await.get_group()?,
                                    None,
                                    receipt.transaction_hash,
                                    receipt.gas_used,
                                    receipt.effective_gas_price
                                )
                            );
                        }
                        Err(e) => match e {
                            ContractClientError::TransactionFailed(receipt) => {
                                error!(
                                    "{}",
                                    build_group_related_transaction_receipt_payload(
                                        LogType::DKGGroupingCommitFailed,
                                        "DKG grouping commit failed.",
                                        chain_id,
                                        self.group_cache.read().await.get_group()?,
                                        None,
                                        receipt.transaction_hash,
                                        receipt.gas_used,
                                        receipt.effective_gas_price
                                    )
                                );
                            }
                            _ => {
                                error!(
                                    "{}",
                                    build_group_related_payload(
                                        LogType::DKGGroupingCommitFailed,
                                        &format!("DKG grouping commit failed with error: {:?}", e),
                                        chain_id,
                                        self.group_cache.read().await.get_group()?
                                    )
                                );
                            }
                        },
                    }
                }
            },
            Err(e) => {
                error!(
                    "{}",
                    build_group_related_payload(
                        LogType::DKGGroupingFailed,
                        &format!("DKG grouping failed with error: {:?}", e),
                        chain_id,
                        self.group_cache.read().await.get_group()?
                    )
                );
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> Subscriber for InGroupingSubscriber<PC> {
    async fn notify(&self, topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()> {
        debug!("{:?}", topic);

        let RunDKG { dkg_task: task, .. } =
            payload.as_any().downcast_ref::<RunDKG>().unwrap().clone();

        static RNG_FN: fn() -> ThreadRng = rand::thread_rng;

        let chain_identity = self.chain_identity.clone();

        let group_cache_for_handler = self.group_cache.clone();

        let group_cache_for_handler_shutdown_signal = self.group_cache.clone();

        let task_group_index = task.group_index;

        let task_epoch = task.epoch;

        let mut handler = AllInOneDKGHandler::new(
            RNG_FN,
            chain_identity,
            self.node_cache.clone(),
            self.group_cache.clone(),
            self.dkg_wait_for_phase_interval_millis,
        );

        self.ts.write().await.add_task_with_shutdown_signal(
            async move {
                if let Err(e) = handler.handle(task).await {
                    error!("{:?}", e);
                } else if let Err(e) = group_cache_for_handler
                    .write()
                    .await
                    .update_dkg_status(task_group_index, task_epoch, DKGStatus::CommitSuccess)
                    .await
                {
                    error!("{:?}", e);
                }
            },
            move || {
                let group_cache = group_cache_for_handler_shutdown_signal.clone();
                async move {
                    let cache_index = group_cache.clone().read().await.get_index().unwrap_or(0);

                    let cache_epoch = group_cache.clone().read().await.get_epoch().unwrap_or(0);

                    cache_index != task_group_index || cache_epoch != task_epoch
                    //NodeError::GroupIndexObsolete(cache_index)
                    //NodeError::GroupEpochObsolete(cache_epoch)
                }
            },
            2000,
        );

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write().await.subscribe(Topic::RunDKG, subscriber);
    }
}

impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> DebuggableSubscriber
    for InGroupingSubscriber<PC>
{
}

#[cfg(feature = "unittest")]
mod tests {
    use super::*;
    use crate::{
        event::{run_dkg::RunDKG, types::Topic},
        queue::event_queue::EventQueue,
        scheduler::dynamic::SimpleDynamicTaskScheduler, 
        test_contracts::{
            mockcontroller::{deploy_mock_controller_with_args, get_mock_controller_at}, 
            mockcoordinator::deploy_mock_coordinator_with_args
        },
    };
    use arpa_core::{Config, DKGStatus, GeneralMainChainIdentity, DKGTask};
    use crate::test_contracts::mockcontroller::MockController;
    use arpa_dal::{
        cache::{InMemoryGroupInfoCache, InMemoryNodeInfoCache},
        GroupInfoHandler, NodeInfoHandler,
    };
    use ethers::prelude::*;
    use rand::thread_rng;
    use std::sync::Arc;
    use threshold_bls::{schemes::bn254::G2Curve, group::Element};
    use tokio::sync::RwLock;

    const WS_ENDPOINT: &str = "ws://localhost:8545";
    const HTTP_ENDPOINT: &str = "http://localhost:8545";

    async fn setup_anvil() -> (ethers::utils::AnvilInstance, Arc<Provider<Ws>>, LocalWallet) {
        let anvil = ethers::utils::Anvil::new().spawn();
        let ws_provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await.expect("Failed to connect to anvil"));
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let wallet = wallet.with_chain_id(anvil.chain_id());
        (anvil, ws_provider, wallet)
    }

    async fn deploy_controller(ws_provider: Arc<Provider<Ws>>, wallet: LocalWallet) -> Address {
        let client = Arc::new(SignerMiddleware::new((*ws_provider).clone(), wallet.clone()));
        let controller_address = deploy_mock_controller_with_args(client.clone(), Address::random()).await.unwrap();
        let controller_contract: MockController<SignerMiddleware<Provider<Ws>, LocalWallet>> = 
            get_mock_controller_at(controller_address, client.clone());
        controller_contract.set_should_succeed(true).send().await.unwrap().await.unwrap();
        controller_address
    }

    async fn deploy_coordinator(ws_provider: Arc<Provider<Ws>>, wallet: LocalWallet, threshold: u64) -> Address {
        let client = Arc::new(SignerMiddleware::new((*ws_provider).clone(), wallet.clone()));
        deploy_mock_coordinator_with_args(client.clone(), U256::from(threshold)).await.unwrap()
    }

    async fn create_chain_identity(
        chain_id: u64,
        wallet: LocalWallet,
        ws_provider: Arc<Provider<Ws>>,
        ws_endpoint: String,
        controller_address: Address,
    ) -> Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> {
        let config = Config::default();
        let general_chain_identity = GeneralMainChainIdentity::new(
            chain_id.try_into().unwrap(),
            wallet.clone(),
            ws_provider.clone(),
            ws_endpoint,
            controller_address,
            Address::random(),
            Address::random(), 
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );
        Arc::new(RwLock::new(Box::new(general_chain_identity)))
    }

    async fn setup_node_cache(id_address: Address) -> NodeResult<Arc<RwLock<Box<dyn NodeInfoHandler<G2Curve>>>>> {
        let node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryNodeInfoCache::<G2Curve>::new(id_address)),
        ));
        {
            let mut node_cache_write = node_cache.write().await;
            node_cache_write.set_node_rpc_endpoint(HTTP_ENDPOINT.to_string()).await?;
            let private_key = <G2Curve as threshold_bls::group::Curve>::Scalar::rand(&mut thread_rng());
            let mut public_key = <G2Curve as threshold_bls::group::Curve>::Point::one();
            public_key.mul(&private_key);
            node_cache_write.set_dkg_key_pair(private_key, public_key).await?;
        }
        Ok(node_cache)
    }

    async fn setup_group_cache(
        id_address: Address,
        chain_id: usize,
        group_index: usize,
        epoch: usize,
        size: usize,
        threshold: usize,
        member_addresses: Vec<Address>,
        dkg_status: DKGStatus,
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
                assignment_block_height: 100,
                members: member_addresses.clone(),
                coordinator_address: Address::random(),
            };
            group_cache_write.save_task_info(chain_id, dkg_task).await?;
            group_cache_write.update_dkg_status(group_index, epoch, dkg_status).await?;
        }
        Ok(group_cache)
    }

    fn create_schedulers() -> (Arc<RwLock<EventQueue>>, Arc<RwLock<SimpleDynamicTaskScheduler>>) {
        (
            Arc::new(RwLock::new(EventQueue::new())),
            Arc::new(RwLock::new(SimpleDynamicTaskScheduler::new()))
        )
    }

    #[tokio::test]
    async fn test_in_grouping_subscriber_creation() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let controller_address = deploy_controller(ws_provider.clone(), wallet.clone()).await;
        let chain_identity = create_chain_identity(1, wallet, ws_provider, WS_ENDPOINT.to_string(), controller_address).await;
        let id_address = Address::random();
        let node_cache = setup_node_cache(id_address).await.unwrap();
        let group_cache = setup_group_cache(
            id_address, 1, 1, 1, 3, 2, 
            vec![id_address, Address::random(), Address::random()],
            DKGStatus::InPhase,
        ).await.unwrap();
        let (eq, ts) = create_schedulers();
        let subscriber = InGroupingSubscriber::new(chain_identity, node_cache, group_cache, eq, ts, 1000);
        assert_eq!(subscriber.dkg_wait_for_phase_interval_millis, 1000);
    }

    #[tokio::test]
    async fn test_dkg_handler_creation() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let controller_address = deploy_controller(ws_provider.clone(), wallet.clone()).await;
        let coordinator_address = deploy_coordinator(ws_provider.clone(), wallet.clone(), 2).await;
        let chain_identity = create_chain_identity(1, wallet, ws_provider, WS_ENDPOINT.to_string(), controller_address).await;
        let id_address = Address::random();
        let node_cache = setup_node_cache(id_address).await.unwrap();
        let group_cache = setup_group_cache(
            id_address, 1, 1, 1, 3, 2, 
            vec![id_address, Address::random(), Address::random()],
            DKGStatus::InPhase,
        ).await.unwrap();
        let handler = AllInOneDKGHandler::new(|| thread_rng(), chain_identity, node_cache, group_cache.clone(), 1000);
        let task = DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 2,
            members: vec![id_address, Address::random(), Address::random()],
            assignment_block_height: 100,
            coordinator_address,
        };
        assert_eq!(task.group_index, 1);
        assert_eq!(task.epoch, 1);
        assert_eq!(task.size, 3);
        assert_eq!(task.threshold, 2);
        assert_eq!(handler.dkg_wait_for_phase_interval_millis, 1000);
    }

    #[tokio::test]
    async fn test_subscriber_notify() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let controller_address = deploy_controller(ws_provider.clone(), wallet.clone()).await;
        let coordinator_address = deploy_coordinator(ws_provider.clone(), wallet.clone(), 2).await;
        let chain_identity = create_chain_identity(1, wallet, ws_provider, WS_ENDPOINT.to_string(), controller_address).await;
        let id_address = Address::random();
        let node_cache = setup_node_cache(id_address).await.unwrap();
        let group_cache = setup_group_cache(
            id_address, 1, 1, 1, 3, 2, 
            vec![id_address, Address::random(), Address::random()],
            DKGStatus::InPhase,
        ).await.unwrap();
        let (eq, ts) = create_schedulers();
        let subscriber = InGroupingSubscriber::new(chain_identity, node_cache, group_cache, eq, ts, 1000);
        let task = DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 2,
            members: vec![id_address, Address::random(), Address::random()],
            assignment_block_height: 100,
            coordinator_address,
        };
        let run_dkg_event = RunDKG { dkg_task: task };
        let result = subscriber.notify(Topic::RunDKG, &run_dkg_event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscriber_subscribe() {
        let id_address = Address::random();
        let node_cache = setup_node_cache(id_address).await.unwrap();
        let group_cache = setup_group_cache(
            id_address, 1, 1, 1, 3, 2, 
            vec![id_address, Address::random(), Address::random()],
            DKGStatus::InPhase,
        ).await.unwrap();
        let (eq, ts) = create_schedulers();
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let controller_address = Address::random();
        let chain_identity = create_chain_identity(1, wallet, ws_provider, WS_ENDPOINT.to_string(), controller_address).await;
        let subscriber = InGroupingSubscriber::new(chain_identity, node_cache, group_cache, eq.clone(), ts, 1000);
        subscriber.subscribe().await;
    }

    #[tokio::test]
    async fn test_group_cache_operations() {
        let id_address = Address::random();
        let group_cache = setup_group_cache(
            id_address, 1, 2, 2, 5, 3, 
            vec![id_address, Address::random(), Address::random(), Address::random(), Address::random()],
            DKGStatus::InPhase,
        ).await.unwrap();
        {
            let group_cache_read = group_cache.read().await;
            assert_eq!(group_cache_read.get_index().unwrap(), 2);
            assert_eq!(group_cache_read.get_epoch().unwrap(), 2);
            assert_eq!(group_cache_read.get_size().unwrap(), 5);
            assert_eq!(group_cache_read.get_threshold().unwrap(), 3);
            assert_eq!(group_cache_read.get_self_id_address().unwrap(), id_address);
            assert_eq!(group_cache_read.get_dkg_status().unwrap(), DKGStatus::InPhase);
        }
        {
            let mut group_cache_write = group_cache.write().await;
            let result = group_cache_write.update_dkg_status(2, 2, DKGStatus::CommitSuccess).await;
            assert!(result.is_ok());
        }
        {
            let group_cache_read = group_cache.read().await;
            assert_eq!(group_cache_read.get_dkg_status().unwrap(), DKGStatus::CommitSuccess);
        }
    }

    #[tokio::test]
    async fn test_node_cache_operations() {
        let id_address = Address::random();
        let node_cache = setup_node_cache(id_address).await.unwrap();
        {
            let node_cache_read = node_cache.read().await;
            assert_eq!(node_cache_read.get_id_address().unwrap(), id_address);
            assert_eq!(node_cache_read.get_node_rpc_endpoint().unwrap(), HTTP_ENDPOINT);
            assert!(node_cache_read.get_dkg_private_key().is_ok());
            assert!(node_cache_read.get_dkg_public_key().is_ok());
        }
        {
            let mut node_cache_write = node_cache.write().await;
            let new_endpoint = "http://localhost:9545".to_string();
            let result = node_cache_write.set_node_rpc_endpoint(new_endpoint.clone()).await;
            assert!(result.is_ok());
        }
        {
            let node_cache_read = node_cache.read().await;
            assert_eq!(node_cache_read.get_node_rpc_endpoint().unwrap(), "http://localhost:9545");
        }
    }

    #[tokio::test]
    async fn test_dkg_task_with_different_statuses() {
        let id_address = Address::random();
        let statuses = vec![DKGStatus::None, DKGStatus::InPhase, DKGStatus::WaitForPostProcess, DKGStatus::CommitSuccess];
        for (i, status) in statuses.iter().enumerate() {
            let group_cache = setup_group_cache(
                id_address, 1, i + 1, 1, 3, 2, 
                vec![id_address, Address::random(), Address::random()],
                *status,
            ).await.unwrap();
            let group_cache_read = group_cache.read().await;
            assert_eq!(group_cache_read.get_index().unwrap(), i + 1);
            assert_eq!(group_cache_read.get_dkg_status().unwrap(), *status);
        }
    }

    #[tokio::test]
    async fn test_multiple_group_members() {
        let id_address = Address::random();
        let member_addresses = vec![id_address, Address::random(), Address::random(), Address::random(), Address::random()];
        let group_cache = setup_group_cache(
            id_address, 1, 1, 1, 5, 3, 
            member_addresses.clone(),
            DKGStatus::InPhase,
        ).await.unwrap();
        let group_cache_read = group_cache.read().await;
        assert_eq!(group_cache_read.get_size().unwrap(), 5);
        assert_eq!(group_cache_read.get_threshold().unwrap(), 3);
        let members = group_cache_read.get_members().unwrap();
        assert_eq!(members.len(), 5);
        assert!(members.contains_key(&id_address));
    }

    #[tokio::test]
    async fn test_contract_integration() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let controller_address = deploy_controller(ws_provider.clone(), wallet.clone()).await;
        let coordinator_address = deploy_coordinator(ws_provider.clone(), wallet.clone(), 2).await;
        let chain_identity = create_chain_identity(1, wallet, ws_provider, WS_ENDPOINT.to_string(), controller_address).await;
        assert_ne!(controller_address, Address::zero());
        assert_ne!(coordinator_address, Address::zero());
        let chain_identity_read = chain_identity.read().await;
        chain_identity_read.build_controller_client();
        chain_identity_read.build_coordinator_client(coordinator_address);
    }
}