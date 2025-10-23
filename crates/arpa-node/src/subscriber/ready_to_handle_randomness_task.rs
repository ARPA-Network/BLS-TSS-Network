use crate::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    committer::{
        client::GeneralCommitterClient, CommitterClient, CommitterClientHandler, CommitterService,
    },
    error::NodeResult,
    event::{ready_to_handle_randomness_task::ReadyToHandleRandomnessTask, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use alloy::primitives::{Address, U256};
use arpa_core::{
    log::{build_task_related_payload, LogType},
    u256_to_vec, BLSTaskType, ComponentTaskType, ExponentialBackoffRetryDescriptor, RandomnessTask,
    SubscriberType, TaskType,
};
use arpa_dal::cache::RandomnessResultCache;
use arpa_dal::{BLSTasksHandler, GroupInfoHandler, SignatureResultCacheHandler};
use async_trait::async_trait;
use log::{debug, error, info};
use serde_json::json;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::{
    group::Curve,
    sig::{SignatureScheme, ThresholdScheme},
};
use tokio::sync::RwLock;

use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};

#[derive(Debug)]
pub struct ReadyToHandleRandomnessTaskSubscriber<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    pub chain_id: u64,
    id_address: Address,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
    randomness_signature_cache:
        Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
    s: PhantomData<S>,
    commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl<PC: Curve, S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>>
    ReadyToHandleRandomnessTaskSubscriber<PC, S>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain_id: u64,
        id_address: Address,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
        randomness_signature_cache: Arc<
            RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
        commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor,
    ) -> Self {
        ReadyToHandleRandomnessTaskSubscriber {
            chain_id,
            id_address,
            group_cache,
            randomness_tasks_cache,
            randomness_signature_cache,
            eq,
            ts,
            c: PhantomData,
            s: PhantomData,
            commit_partial_signature_retry_descriptor,
        }
    }
}

#[async_trait]
pub trait RandomnessHandler {
    async fn handle(self) -> NodeResult<()>;

    async fn send_partial_signature(
        &self,
        task: &RandomnessTask,
        actual_seed: Vec<u8>,
        partial_signature: Vec<u8>,
    ) -> NodeResult<()>;
}

pub struct GeneralRandomnessHandler<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    chain_id: u64,
    id_address: Address,
    tasks: Vec<RandomnessTask>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
    randomness_signature_cache:
        Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
    s: PhantomData<S>,
    commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor,
}

impl<
        PC: Curve + Sync + Send,
        S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar> + Sync + Send,
    > CommitterClientHandler<GeneralCommitterClient, PC> for GeneralRandomnessHandler<PC, S>
{
    async fn get_id_address(&self) -> Address {
        self.id_address
    }

    fn get_group_cache(&self) -> Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>> {
        self.group_cache.clone()
    }

    fn get_commit_partial_signature_retry_descriptor(&self) -> ExponentialBackoffRetryDescriptor {
        self.commit_partial_signature_retry_descriptor
    }
}

#[async_trait]
impl<
        PC: Curve + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Sync
            + Send
            + 'static,
    > RandomnessHandler for GeneralRandomnessHandler<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    async fn handle(self) -> NodeResult<()> {
        for task in self.tasks.iter() {
            let actual_seed = [
                &u256_to_vec(&task.seed)[..],
                &u256_to_vec(&U256::from(task.assignment_block_height))[..],
            ]
            .concat();

            match SimpleBLSCore::<PC, S>::partial_sign(
                self.group_cache.read().await.get_secret_share()?,
                &actual_seed,
            ) {
                Ok(signed_partial_signature) => {
                    info!(
                        "{}",
                        build_task_related_payload(
                            LogType::PartialSignatureFinished,
                            "Partial signature generated.",
                            self.chain_id,
                            &task.request_id,
                            TaskType::BLS(BLSTaskType::Randomness),
                            json!(task),
                            None
                        )
                    );

                    self.send_partial_signature(task, actual_seed, signed_partial_signature)
                        .await?;
                }
                Err(e) => {
                    error!(
                        "{}",
                        build_task_related_payload(
                            LogType::PartialSignatureFailed,
                            &format!("Partial signature generation failed with error: {:?}", e),
                            self.chain_id,
                            &task.request_id,
                            TaskType::BLS(BLSTaskType::Randomness),
                            json!(task),
                            None
                        )
                    );
                }
            }
        }

        Ok(())
    }

    async fn send_partial_signature(
        &self,
        task: &RandomnessTask,
        actual_seed: Vec<u8>,
        partial_signature: Vec<u8>,
    ) -> NodeResult<()> {
        let threshold = self.group_cache.read().await.get_threshold()?;

        let current_group_index = self.group_cache.read().await.get_index()?;

        let current_member_index = self.group_cache.read().await.get_self_index()?;

        if self
            .group_cache
            .read()
            .await
            .is_committer(self.id_address)?
        {
            let contained_res = self
                .randomness_signature_cache
                .read()
                .await
                .contains(&task.request_id)
                .await?;
            if !contained_res {
                let task = self
                    .randomness_tasks_cache
                    .read()
                    .await
                    .get(&task.request_id)
                    .await?;

                self.randomness_signature_cache
                    .write()
                    .await
                    .add(current_group_index, task, actual_seed.to_vec(), threshold)
                    .await?;
            }

            self.randomness_signature_cache
                .write()
                .await
                .add_partial_signature(
                    task.request_id.clone(),
                    self.id_address,
                    current_member_index,
                    partial_signature.clone(),
                )
                .await?;
        }

        let committers = self.prepare_committer_clients().await?;

        for committer in committers.into_iter() {
            let chain_id = self.chain_id;
            let request_id = task.request_id.clone();
            let actual_seed = actual_seed.clone();
            let partial_signature = partial_signature.clone();
            let task_json = json!(task);

            self.ts.write().await.add_task(
                ComponentTaskType::Subscriber(chain_id, SubscriberType::SendingPartialSignature),
                async move {
                    let committer_id = committer.get_committer_id_address();

                    match committer
                        .commit_partial_signature(
                            chain_id,
                            BLSTaskType::Randomness,
                            request_id.clone(),
                            actual_seed,
                            partial_signature,
                        )
                        .await
                    {
                        Ok(true) => {
                            info!(
                                "{}",
                                build_task_related_payload(
                                    LogType::PartialSignatureSent,
                                    "Partial signature sent and accepted.",
                                    chain_id,
                                    &request_id,
                                    TaskType::BLS(BLSTaskType::Randomness),
                                    task_json,
                                    Some(committer_id)
                                )
                            );
                        }
                        Ok(false) => {
                            info!(
                                "{}",
                                build_task_related_payload(
                                    LogType::PartialSignatureSendingRejected,
                                    "Partial signature sent and rejected.",
                                    chain_id,
                                    &request_id,
                                    TaskType::BLS(BLSTaskType::Randomness),
                                    task_json,
                                    Some(committer_id)
                                )
                            );
                        }
                        Err(e) => {
                            error!(
                                "{}",
                                build_task_related_payload(
                                    LogType::PartialSignatureSendingFailed,
                                    &format!(
                                        "Partial signature sending failed with error: {:?}",
                                        e
                                    ),
                                    chain_id,
                                    &request_id,
                                    TaskType::BLS(BLSTaskType::Randomness),
                                    task_json,
                                    Some(committer_id)
                                )
                            );
                        }
                    }
                },
            )?;
        }

        Ok(())
    }
}

#[async_trait]
impl<
        PC: Curve + std::fmt::Debug + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Sync
            + Send
            + 'static,
    > Subscriber for ReadyToHandleRandomnessTaskSubscriber<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    async fn notify(&self, topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()> {
        debug!("{:?}", topic);

        let ReadyToHandleRandomnessTask { tasks, .. } = payload
            .as_any()
            .downcast_ref::<ReadyToHandleRandomnessTask>()
            .unwrap()
            .clone();

        let chain_id = self.chain_id;

        let id_address = self.id_address;

        let group_cache_for_handler = self.group_cache.clone();

        let randomness_tasks_cache_for_handler = self.randomness_tasks_cache.clone();

        let randomness_signature_cache_for_handler = self.randomness_signature_cache.clone();

        let task_scheduler_for_handler = self.ts.clone();

        let commit_partial_signature_retry_descriptor =
            self.commit_partial_signature_retry_descriptor;

        self.ts.write().await.add_task(
            ComponentTaskType::Subscriber(chain_id, SubscriberType::ReadyToHandleRandomnessTask),
            async move {
                let handler = GeneralRandomnessHandler {
                    chain_id,
                    id_address,
                    tasks,
                    group_cache: group_cache_for_handler,
                    randomness_tasks_cache: randomness_tasks_cache_for_handler,
                    randomness_signature_cache: randomness_signature_cache_for_handler,
                    ts: task_scheduler_for_handler,
                    c: PhantomData::<PC>,
                    s: PhantomData::<S>,
                    commit_partial_signature_retry_descriptor,
                };

                if let Err(e) = handler.handle().await {
                    error!("{:?}", e);
                }
            },
        )?;

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let chain_id = self.chain_id;

        let subscriber = Box::new(self);

        eq.write()
            .await
            .subscribe(Topic::ReadyToHandleRandomnessTask(chain_id), subscriber);
    }
}

impl<
        PC: Curve + std::fmt::Debug + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Sync
            + Send
            + 'static,
    > DebuggableSubscriber for ReadyToHandleRandomnessTaskSubscriber<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        algorithm::bls::SimpleBLSCore, committer::CommitterClient, event::{ready_to_handle_randomness_task::ReadyToHandleRandomnessTask, types::Topic}, queue::event_queue::EventQueue, scheduler::dynamic::SimpleDynamicTaskScheduler
    };
    use arpa_core::{ExponentialBackoffRetryDescriptor, RandomnessTask};
    use arpa_dal::{
        cache::{InMemoryGroupInfoCache, InMemoryBLSTasksQueue, InMemorySignatureResultCache, RandomnessResultCache},
        BLSTasksHandler, GroupInfoHandler, SignatureResultCacheHandler,
    };
    use std::{sync::Arc, time::Duration};
    use threshold_bls::schemes::bn254::{G2Curve, G2Scheme};
    use tokio::sync::RwLock;
    use tonic::{transport::Server, Request, Response, Status};
    use crate::rpc_stub::committer::{
        committer_service_server::{CommitterService, CommitterServiceServer},
        CommitPartialSignatureRequest, CommitPartialSignatureReply,
        committer_service_client::CommitterServiceClient,
        commit_partial_signature_request::BlsTaskType,
    };

    const DEFAULT_BASE_PORT: u16 = 50051;
    const DEFAULT_RETRY_BASE: u64 = 1000;
    const DEFAULT_RETRY_FACTOR: u64 = 2;
    const DEFAULT_RETRY_ATTEMPTS: usize = 3;
    const DEFAULT_GROUP_SIZE: usize = 3;
    const DEFAULT_THRESHOLD: usize = 2;
    const DEFAULT_GROUP_INDEX: u32 = 1;
    const DEFAULT_EPOCH: usize = 1;
    const DEFAULT_CHAIN_ID: usize = 1;
    const DEFAULT_BLOCK_HEIGHT: usize = 100;
    const DEFAULT_SUBSCRIPTION_ID: u64 = 1;
    const DEFAULT_CONFIRMATIONS: u16 = 6;
    const DEFAULT_GAS_LIMIT: u32 = 200000;
    const DEFAULT_GAS_PRICE: u64 = 1000000000;
    const DEFAULT_SEED: u64 = 12345;

    #[derive(Debug, Clone)]
    pub struct MockCommitterService {
        pub should_accept: bool,
        pub response_delay: Option<Duration>,
        pub call_count: Arc<std::sync::Mutex<usize>>,
        pub received_requests: Arc<std::sync::Mutex<Vec<CommitPartialSignatureRequest>>>,
    }

    impl MockCommitterService {
        pub fn new(should_accept: bool) -> Self {
            Self {
                should_accept,
                response_delay: None,
                call_count: Arc::new(std::sync::Mutex::new(0)),
                received_requests: Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }

        pub fn with_delay(mut self, delay: Duration) -> Self {
            self.response_delay = Some(delay);
            self
        }

        pub fn get_call_count(&self) -> usize {
            *self.call_count.lock().unwrap()
        }

        pub fn get_received_requests(&self) -> Vec<CommitPartialSignatureRequest> {
            self.received_requests.lock().unwrap().clone()
        }
    }

    #[tonic::async_trait]
    impl CommitterService for MockCommitterService {
        async fn commit_partial_signature(
            &self,
            request: Request<CommitPartialSignatureRequest>,
        ) -> Result<Response<CommitPartialSignatureReply>, Status> {
            *self.call_count.lock().unwrap() += 1;
            self.received_requests.lock().unwrap().push(request.get_ref().clone());

            if let Some(delay) = self.response_delay {
                tokio::time::sleep(delay).await;
            }

            Ok(Response::new(CommitPartialSignatureReply {
                result: self.should_accept,
            }))
        }
    }

    #[derive(Debug, Clone)]
    pub struct MockCommitterClient {
        pub id_address: Address,
        pub committer_id_address: Address,
        pub committer_endpoint: String,
        pub grpc_client: Arc<tokio::sync::Mutex<CommitterServiceClient<tonic::transport::Channel>>>,
    }

    impl MockCommitterClient {
        pub async fn new(
            id_address: Address,
            committer_id_address: Address,
            server_address: String,
        ) -> Result<Self, Box<dyn std::error::Error>> {
            let channel = tonic::transport::Channel::from_shared(server_address.clone())?
                .connect()
                .await?;
            
            Ok(Self {
                id_address,
                committer_id_address,
                committer_endpoint: server_address,
                grpc_client: Arc::new(tokio::sync::Mutex::new(CommitterServiceClient::new(channel))),
            })
        }

        pub async fn commit_partial_signature(
            &self,
            chain_id: usize,
            task_type: arpa_core::BLSTaskType,
            request_id: Vec<u8>,
            message: Vec<u8>,
            partial_signature: Vec<u8>,
        ) -> crate::error::NodeResult<bool> {
            let grpc_task_type = match task_type {
                arpa_core::BLSTaskType::Randomness => BlsTaskType::Randomness,
                arpa_core::BLSTaskType::GroupRelay => BlsTaskType::GroupRelay,
                arpa_core::BLSTaskType::GroupRelayConfirmation => BlsTaskType::GroupRelayConfirmation,
            };

            let request = CommitPartialSignatureRequest {
                id_address: format!("{:?}", self.id_address),
                chain_id: chain_id as u32,
                task_type: grpc_task_type as i32,
                request_id,
                message,
                partial_signature,
            };

            let response = self.grpc_client.lock().await
                .commit_partial_signature(request)
                .await
                .map_err(|e| crate::error::NodeError::RpcResponseError(e))?;
            Ok(response.into_inner().result)
        }
    }

    impl CommitterClient for MockCommitterClient {
        fn get_id_address(&self) -> Address {
            self.id_address
        }

        fn get_committer_id_address(&self) -> Address {
            self.committer_id_address
        }

        fn get_committer_endpoint(&self) -> &str {
            &self.committer_endpoint
        }

        fn build(
            _id_address: Address,
            _committer_id_address: Address,
            _committer_endpoint: String,
            _commit_partial_signature_retry_descriptor: ExponentialBackoffRetryDescriptor,
        ) -> Self {
            panic!("Use MockCommitterClient::new() instead for async construction")
        }
    }

    fn default_retry_descriptor() -> ExponentialBackoffRetryDescriptor {
        ExponentialBackoffRetryDescriptor {
            base: DEFAULT_RETRY_BASE,
            factor: DEFAULT_RETRY_FACTOR,
            max_attempts: DEFAULT_RETRY_ATTEMPTS,
            use_jitter: false,
        }
    }

    async fn start_mock_grpc_server(
        service: MockCommitterService,
        port: u16,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let addr = format!("127.0.0.1:{}", port).parse().unwrap();
            if let Err(e) = Server::builder()
                .add_service(CommitterServiceServer::new(service))
                .serve(addr)
                .await
            {
                eprintln!("Mock gRPC server error: {}", e);
            }
        })
    }

    async fn setup_group_cache_with_secret(
        id_address: Address,
        chain_id: usize,
        group_index: usize,
        epoch: usize,
        size: usize,
        threshold: usize,
        member_addresses: Vec<Address>,
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

            group_cache_write.save_task_info(chain_id, dkg_task).await?;
            group_cache_write.update_dkg_status(group_index, epoch, arpa_core::DKGStatus::InPhase).await?;

            use rand::thread_rng;
            use threshold_bls::{group::Element, sig::Share};
            use dkg_core::primitives::{DKGOutput, Group as DKGGroup, Node};
            use threshold_bls::poly::{PublicPoly, Idx};
            
            let secret_scalar = <G2Curve as threshold_bls::group::Curve>::Scalar::rand(&mut thread_rng());
            
            let mut dkg_nodes = Vec::new();
            for (i, &_addr) in member_addresses.iter().enumerate() {
                let mut node_public_key = <G2Curve as threshold_bls::group::Curve>::Point::one();
                let node_secret = <G2Curve as threshold_bls::group::Curve>::Scalar::rand(&mut thread_rng());
                node_public_key.mul(&node_secret);
                dkg_nodes.push(Node::new(i as Idx, node_public_key));
            }

            let qual = DKGGroup {
                nodes: dkg_nodes,
                threshold: threshold,
            };

            let public = PublicPoly::<G2Curve>::new(threshold - 1);
            let self_index = member_addresses
                .iter()
                .position(|&addr| addr == id_address)
                .unwrap_or(0) as Idx;
                
            let share = Share {
                index: self_index,
                private: secret_scalar,
            };

            let dkg_output = DKGOutput {
                qual,
                public,
                share,
                disqualified_node_indices: vec![],
            };
        
            group_cache_write.save_successful_output(group_index, epoch, dkg_output).await?;
            
            let committers = member_addresses.iter().take(2).cloned().collect::<Vec<_>>();
            group_cache_write.save_committers(group_index, epoch, committers).await?;
        }
        Ok(group_cache)
    }

    async fn setup_group_cache_simple(
        id_address: Address,
        chain_id: usize,
        group_index: usize,
        epoch: usize,
        size: usize,
        threshold: usize,
        member_addresses: Vec<Address>,
    ) -> NodeResult<Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>>> {
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address)),
        ));

        let dkg_task = arpa_core::DKGTask {
            group_index,
            epoch,
            size,
            threshold,
            assignment_block_height: DEFAULT_BLOCK_HEIGHT,
            members: member_addresses.clone(),
            coordinator_address: Address::random(),
        };

        {
            let mut group_cache_write = group_cache.write().await;
            group_cache_write.save_task_info(chain_id, dkg_task).await?;
            
            let committers = member_addresses.iter().take(2).cloned().collect::<Vec<_>>();
            group_cache_write.save_committers(group_index, epoch, committers).await?;
            group_cache_write.update_dkg_status(group_index, epoch, arpa_core::DKGStatus::InPhase).await?;
        }
        Ok(group_cache)
    }

    async fn setup_randomness_tasks_cache(
        tasks: Vec<RandomnessTask>,
    ) -> Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>> {
        let cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryBLSTasksQueue::<RandomnessTask>::new()), 
        ));

        {
            let mut cache_write = cache.write().await;
            for task in tasks {
                cache_write.add(task).await.unwrap();
            }
        }
        cache
    }

    async fn setup_randomness_signature_cache() -> Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>> {
        Arc::new(RwLock::new(
            Box::new(InMemorySignatureResultCache::<RandomnessResultCache>::new()), 
        ))
    }

    fn create_test_randomness_task() -> RandomnessTask {
        RandomnessTask {
            request_id: vec![1, 2, 3, 4],
            subscription_id: DEFAULT_SUBSCRIPTION_ID,
            group_index: DEFAULT_GROUP_INDEX,
            request_type: arpa_core::RandomnessRequestType::Randomness,
            params: vec![],
            requester: Address::random(),
            seed: ethers::types::U256::from(DEFAULT_SEED),
            request_confirmations: DEFAULT_CONFIRMATIONS,
            callback_gas_limit: DEFAULT_GAS_LIMIT,
            callback_max_gas_price: ethers::types::U256::from(DEFAULT_GAS_PRICE),
            assignment_block_height: DEFAULT_BLOCK_HEIGHT,
        }
    }

    fn create_schedulers() -> (Arc<RwLock<EventQueue>>, Arc<RwLock<SimpleDynamicTaskScheduler>>) {
        (
            Arc::new(RwLock::new(EventQueue::new())),
            Arc::new(RwLock::new(SimpleDynamicTaskScheduler::new())),
        )
    }

    #[tokio::test]
    async fn test_randomness_task_subscriber_creation() {
        let id_address = Address::random();

        let group_cache = setup_group_cache_simple( 
            id_address,
            DEFAULT_CHAIN_ID,
            DEFAULT_GROUP_INDEX as usize,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::random(), Address::random()],
        ).await.unwrap();

        let randomness_tasks_cache = setup_randomness_tasks_cache(vec![]).await;
        let randomness_signature_cache = setup_randomness_signature_cache().await;
        let (eq, ts) = create_schedulers();

        let subscriber = ReadyToHandleRandomnessTaskSubscriber::<G2Curve, G2Scheme>::new(
            DEFAULT_CHAIN_ID,
            id_address,
            group_cache,
            randomness_tasks_cache,
            randomness_signature_cache,
            eq,
            ts,
            default_retry_descriptor(),
        );

        assert_eq!(subscriber.chain_id, DEFAULT_CHAIN_ID);
        assert_eq!(subscriber.id_address, id_address);
    }

    #[tokio::test]
    async fn test_mock_grpc_committer_client_success() {
        let service = MockCommitterService::new(true);
        let port = DEFAULT_BASE_PORT;
        
        let _server_handle = start_mock_grpc_server(service.clone(), port).await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let id_address = Address::random();
        let committer_id_address = Address::random();
        let server_address = format!("http://127.0.0.1:{}", port);
        
        let mock_client = MockCommitterClient::new(
            id_address, 
            committer_id_address, 
            server_address,
        ).await.expect("Failed to create mock client");

        let result = mock_client
            .commit_partial_signature(
                DEFAULT_CHAIN_ID,
                arpa_core::BLSTaskType::Randomness,
                vec![1, 2, 3],
                vec![4, 5, 6],
                vec![7, 8, 9],
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert_eq!(mock_client.get_committer_id_address(), committer_id_address);
        
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(service.get_call_count(), 1);
        
        let received_requests = service.get_received_requests();
        assert_eq!(received_requests.len(), 1);
        assert_eq!(received_requests[0].chain_id, DEFAULT_CHAIN_ID as u32);
        assert_eq!(received_requests[0].task_type, BlsTaskType::Randomness as i32);
        assert_eq!(received_requests[0].request_id, vec![1, 2, 3]);
        assert_eq!(received_requests[0].message, vec![4, 5, 6]);
        assert_eq!(received_requests[0].partial_signature, vec![7, 8, 9]);
    }

    #[tokio::test]
    async fn test_mock_grpc_committer_client_rejection() {
        let service = MockCommitterService::new(false);
        let port = DEFAULT_BASE_PORT + 1;
        
        let _server_handle = start_mock_grpc_server(service.clone(), port).await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let id_address = Address::random();
        let committer_id_address = Address::random();
        let server_address = format!("http://127.0.0.1:{}", port);
        
        let mock_client = MockCommitterClient::new(
            id_address, 
            committer_id_address,  
            server_address,
        ).await.expect("Failed to create mock client");

        let result = mock_client
            .commit_partial_signature(
                DEFAULT_CHAIN_ID,
                arpa_core::BLSTaskType::Randomness,
                vec![1, 2, 3],
                vec![4, 5, 6],
                vec![7, 8, 9],
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(service.get_call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_grpc_committer_with_delay() {
        let service = MockCommitterService::new(true)
            .with_delay(Duration::from_millis(100));
        let port = DEFAULT_BASE_PORT + 2;
        
        let _server_handle = start_mock_grpc_server(service.clone(), port).await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let id_address = Address::random();
        let committer_id_address = Address::random();
        let server_address = format!("http://127.0.0.1:{}", port);
        
        let mock_client = MockCommitterClient::new(
            id_address, 
            committer_id_address,  
            server_address,
        ).await.expect("Failed to create mock client");

        let start_time = std::time::Instant::now();
        
        let result = mock_client
            .commit_partial_signature(
                DEFAULT_CHAIN_ID,
                arpa_core::BLSTaskType::Randomness,
                vec![1, 2, 3],
                vec![4, 5, 6],
                vec![7, 8, 9],
            )
            .await;

        let elapsed = start_time.elapsed();
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert!(elapsed >= Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_multiple_task_types() {
        let service = MockCommitterService::new(true);
        let port = DEFAULT_BASE_PORT + 3;
        
        let _server_handle = start_mock_grpc_server(service.clone(), port).await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let id_address = Address::random();
        let committer_id_address = Address::random();
        let server_address = format!("http://127.0.0.1:{}", port);
        
        let mock_client = MockCommitterClient::new(
            id_address, 
            committer_id_address,  
            server_address,
        ).await.expect("Failed to create mock client");

        let task_types = vec![
            arpa_core::BLSTaskType::Randomness,
            arpa_core::BLSTaskType::GroupRelay,
            arpa_core::BLSTaskType::GroupRelayConfirmation,
        ];

        for (i, task_type) in task_types.iter().enumerate() {
            let result = mock_client
                .commit_partial_signature(
                    DEFAULT_CHAIN_ID,
                    task_type.clone(),
                    vec![i as u8],
                    vec![i as u8 + 10],
                    vec![i as u8 + 20],
                )
                .await;

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), true);
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(service.get_call_count(), 3);

        let received_requests = service.get_received_requests();
        assert_eq!(received_requests.len(), 3);
        
        assert_eq!(received_requests[0].task_type, BlsTaskType::Randomness as i32);
        assert_eq!(received_requests[1].task_type, BlsTaskType::GroupRelay as i32);
        assert_eq!(received_requests[2].task_type, BlsTaskType::GroupRelayConfirmation as i32);
    }

    #[tokio::test]
    async fn test_subscriber_notify_with_mock_grpc() {
        let id_address = Address::random();
        let tasks = vec![create_test_randomness_task()];

        let group_cache = setup_group_cache_with_secret(
            id_address,
            DEFAULT_CHAIN_ID,
            DEFAULT_GROUP_INDEX as usize,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::random(), Address::random()],
        ).await.unwrap();

        let randomness_tasks_cache = setup_randomness_tasks_cache(tasks.clone()).await;
        let randomness_signature_cache = setup_randomness_signature_cache().await;
        let (eq, ts) = create_schedulers();

        let subscriber = ReadyToHandleRandomnessTaskSubscriber::<G2Curve, G2Scheme>::new(
            DEFAULT_CHAIN_ID,
            id_address,
            group_cache,
            randomness_tasks_cache,
            randomness_signature_cache,
            eq,
            ts,
            default_retry_descriptor(),
        );

        let event = ReadyToHandleRandomnessTask { chain_id: DEFAULT_CHAIN_ID, tasks };

        let result = subscriber
            .notify(Topic::ReadyToHandleRandomnessTask(DEFAULT_CHAIN_ID), &event)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_partial_signature_generation() {
        let id_address = Address::random();
        let task = create_test_randomness_task();

        let group_cache = setup_group_cache_with_secret(
            id_address,
            DEFAULT_CHAIN_ID,
            DEFAULT_GROUP_INDEX as usize,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, Address::random(), Address::random()],
        ).await.unwrap();

        let actual_seed = [
            &arpa_core::u256_to_vec(&task.seed)[..],
            &arpa_core::u256_to_vec(&ethers::types::U256::from(task.assignment_block_height))[..],
        ]
        .concat();

        let group_cache_guard = group_cache.read().await;
        let secret_share = group_cache_guard.get_secret_share().unwrap();
        let partial_signature_result = SimpleBLSCore::<G2Curve, G2Scheme>::partial_sign(
            secret_share,
            &actual_seed,
        );

        assert!(partial_signature_result.is_ok());
    }

    #[tokio::test]
    async fn test_signature_cache_operations() {
        let randomness_signature_cache = setup_randomness_signature_cache().await;
        let task = create_test_randomness_task();
        let message = vec![1, 2, 3, 4];

        let result = randomness_signature_cache
            .write()
            .await
            .add(DEFAULT_GROUP_INDEX as usize, task.clone(), message, DEFAULT_THRESHOLD)
            .await;

        assert!(result.is_ok());

        let contains_result = randomness_signature_cache
            .read()
            .await
            .contains(&task.request_id)
            .await;

        assert!(contains_result.is_ok());
        assert!(contains_result.unwrap());
    }

    #[tokio::test]
    async fn test_group_cache_committer_operations() {
        let id_address = Address::random();
        let committer_address = Address::random();
        let non_committer_address = Address::random();

        let group_cache = setup_group_cache_simple(
            id_address,
            DEFAULT_CHAIN_ID,
            DEFAULT_GROUP_INDEX as usize,
            DEFAULT_EPOCH,
            DEFAULT_GROUP_SIZE,
            DEFAULT_THRESHOLD,
            vec![id_address, committer_address, non_committer_address],
        ).await.unwrap();

        let result_id = group_cache.read().await.is_committer(id_address);
        let result_committer = group_cache.read().await.is_committer(committer_address);
        let result_non_committer = group_cache.read().await.is_committer(non_committer_address);

        assert!(result_id.is_ok());
        assert!(result_committer.is_ok());
        assert!(result_non_committer.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_grpc_clients() {
        let service1 = MockCommitterService::new(true);
        let service2 = MockCommitterService::new(false);
        let port1 = DEFAULT_BASE_PORT + 4;
        let port2 = DEFAULT_BASE_PORT + 5;
        
        let _server_handle1 = start_mock_grpc_server(service1.clone(), port1).await;
        let _server_handle2 = start_mock_grpc_server(service2.clone(), port2).await;
        
        tokio::time::sleep(Duration::from_millis(100)).await;

        let client1 = MockCommitterClient::new(
            Address::random(),           
            Address::random(),           
            format!("http://127.0.0.1:{}", port1),
        ).await.unwrap();

        let client2 = MockCommitterClient::new(
            Address::random(),           
            Address::random(),           
            format!("http://127.0.0.1:{}", port2),
        ).await.unwrap();

        let result1 = client1
            .commit_partial_signature(
                DEFAULT_CHAIN_ID,
                arpa_core::BLSTaskType::Randomness,
                vec![1],
                vec![2],
                vec![3],
            )
            .await;

        let result2 = client2
            .commit_partial_signature(
                DEFAULT_CHAIN_ID,
                arpa_core::BLSTaskType::Randomness,
                vec![4],
                vec![5],
                vec![6],
            )
            .await;

        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), true);
        
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), false);

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(service1.get_call_count(), 1);
        assert_eq!(service2.get_call_count(), 1);
    }
}