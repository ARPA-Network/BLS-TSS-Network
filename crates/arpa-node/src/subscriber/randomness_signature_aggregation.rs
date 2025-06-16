use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::{ready_to_fulfill_randomness_task::ReadyToFulfillRandomnessTask, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use arpa_contract_client::{
    adapter::{AdapterTransactions, AdapterViews},
    error::ContractClientError,
};
use arpa_core::{
    log::{build_task_related_payload, build_task_related_transaction_receipt_payload, LogType},
    BLSTaskType, ComponentTaskType, PartialSignature, RandomnessTask, SubscriberType, TaskType,
    DEFAULT_MAX_RANDOMNESS_FULFILLMENT_ATTEMPTS,
};
use arpa_dal::{cache::RandomnessResultCache, BLSResultCacheState};
use arpa_dal::{BlockInfoHandler, SignatureResultCacheHandler};
use async_trait::async_trait;
use ethers::types::{Address, U256};
use log::{debug, error, info};
use serde_json::json;
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};
use threshold_bls::{
    group::Curve,
    sig::{SignatureScheme, ThresholdScheme},
};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct RandomnessSignatureAggregationSubscriber<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    randomness_signature_cache:
        Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    c: PhantomData<PC>,
    s: PhantomData<S>,
}

impl<PC: Curve, S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>>
    RandomnessSignatureAggregationSubscriber<PC, S>
{
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        randomness_signature_cache: Arc<
            RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>,
        >,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        RandomnessSignatureAggregationSubscriber {
            chain_id,
            id_address,
            chain_identity,
            block_cache,
            randomness_signature_cache,
            eq,
            ts,
            c: PhantomData,
            s: PhantomData,
        }
    }
}

#[async_trait]
pub trait FulfillRandomnessHandler {
    async fn handle(
        &self,
        group_index: usize,
        randomness_task: RandomnessTask,
        signature: Vec<u8>,
        partial_signatures: BTreeMap<Address, PartialSignature>,
    ) -> NodeResult<()>;
}

pub struct GeneralFulfillRandomnessHandler<PC: Curve> {
    id_address: Address,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    randomness_signature_cache:
        Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
    pc: PhantomData<PC>,
}

#[async_trait]
impl<PC: Curve> FulfillRandomnessHandler for GeneralFulfillRandomnessHandler<PC> {
    async fn handle(
        &self,
        group_index: usize,
        randomness_task: RandomnessTask,
        signature: Vec<u8>,
        partial_signatures: BTreeMap<Address, PartialSignature>,
    ) -> NodeResult<()> {
        let client = self
            .chain_identity
            .read()
            .await
            .build_adapter_client(self.id_address);

        let chain_id = self.chain_identity.read().await.get_chain_id();

        let randomness_task_request_id = randomness_task.request_id.clone();

        let randomness_task_json = json!(randomness_task);

        if client.is_task_pending(&randomness_task_request_id).await? {
            if self.block_cache.read().await.get_block_height()
                - randomness_task.assignment_block_height
                > 86400 / self.block_cache.read().await.get_block_time()
            {
                self.randomness_signature_cache
                    .write()
                    .await
                    .update_commit_result(&randomness_task_request_id, BLSResultCacheState::Expired)
                    .await?;

                info!("mark randomness task as expired. task request id: {}, assignment_block_height:{:?}",
                    format!("0x{}", hex::encode(randomness_task_request_id)), randomness_task.assignment_block_height);

                return Ok(());
            }

            let wei_per_gas = self
                .chain_identity
                .read()
                .await
                .get_current_gas_price()
                .await?;

            if wei_per_gas > randomness_task.callback_max_gas_price {
                self.randomness_signature_cache
                    .write()
                    .await
                    .update_commit_result(
                        &randomness_task_request_id,
                        BLSResultCacheState::NotCommitted,
                    )
                    .await?;

                self.randomness_signature_cache
                    .write()
                    .await
                    .incr_committed_times(&randomness_task_request_id)
                    .await?;

                info!("cancel fulfilling randomness as gas price is too high! task request id: {}, current_gas_price:{:?}, max_gas_price: {:?}",
                format!("0x{}", hex::encode(&randomness_task_request_id)), wei_per_gas, randomness_task.callback_max_gas_price);

                return Ok(());
            }

            match client
                .fulfill_randomness(
                    group_index,
                    randomness_task,
                    signature.clone(),
                    partial_signatures,
                )
                .await
            {
                Ok(receipt) => {
                    self.randomness_signature_cache
                        .write()
                        .await
                        .update_commit_result(
                            &randomness_task_request_id,
                            BLSResultCacheState::Committed,
                        )
                        .await?;

                    info!(
                        "{}",
                        build_task_related_transaction_receipt_payload(
                            LogType::FulfillmentFinished,
                            "Randomness fulfilled successfully.",
                            chain_id,
                            &randomness_task_request_id,
                            TaskType::BLS(BLSTaskType::Randomness),
                            randomness_task_json,
                            receipt.transaction_hash,
                            receipt.gas_used.unwrap_or(U256::zero()),
                            receipt.effective_gas_price.unwrap_or(U256::zero()),
                        )
                    );
                }
                Err(e) => {
                    self.randomness_signature_cache
                        .write()
                        .await
                        .update_commit_result(
                            &randomness_task_request_id,
                            BLSResultCacheState::NotCommitted,
                        )
                        .await?;

                    match e {
                        ContractClientError::TransactionFailed(receipt) => {
                            error!(
                                "{}",
                                build_task_related_transaction_receipt_payload(
                                    LogType::FulfillmentFailed,
                                    "Randomness fulfillment reverted.",
                                    chain_id,
                                    &randomness_task_request_id,
                                    TaskType::BLS(BLSTaskType::Randomness),
                                    randomness_task_json,
                                    receipt.transaction_hash,
                                    receipt.gas_used.unwrap_or(U256::zero()),
                                    receipt.effective_gas_price.unwrap_or(U256::zero()),
                                )
                            );
                        }
                        _ => {
                            error!(
                                "{}",
                                build_task_related_payload(
                                    LogType::FulfillmentFailed,
                                    &format!("Randomness fulfillment failed with error: {:?}", e),
                                    chain_id,
                                    &randomness_task_request_id,
                                    TaskType::BLS(BLSTaskType::Randomness),
                                    randomness_task_json,
                                    None,
                                )
                            );
                        }
                    }
                }
            }

            self.randomness_signature_cache
                .write()
                .await
                .incr_committed_times(&randomness_task_request_id)
                .await?;
        } else {
            self.randomness_signature_cache
                .write()
                .await
                .update_commit_result(
                    &randomness_task_request_id,
                    BLSResultCacheState::CommittedByOthers,
                )
                .await?;
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
    > Subscriber for RandomnessSignatureAggregationSubscriber<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let ReadyToFulfillRandomnessTask {
            tasks: ready_signature_caches,
            ..
        } = payload
            .as_any()
            .downcast_ref::<ReadyToFulfillRandomnessTask>()
            .unwrap();

        for ready_signature_cache in ready_signature_caches {
            let RandomnessResultCache {
                group_index,
                randomness_task,
                message: _,
                threshold,
                partial_signatures,
                committed_times,
            } = ready_signature_cache.clone();

            if committed_times >= DEFAULT_MAX_RANDOMNESS_FULFILLMENT_ATTEMPTS {
                self.randomness_signature_cache
                    .write()
                    .await
                    .update_commit_result(&randomness_task.request_id, BLSResultCacheState::FAULTY)
                    .await?;

                error!("mark randomness task as faulty for too many failed fulfillment attempts. task request id: {}",
                format!("0x{}", hex::encode(&randomness_task.request_id)));

                continue;
            }

            let partials = partial_signatures
                .values()
                .map(|partial| partial.signed_partial_signature.clone())
                .collect::<Vec<Vec<u8>>>();

            match SimpleBLSCore::<PC, S>::aggregate(threshold, &partials) {
                Ok(signature) => {
                    info!(
                        "{}",
                        build_task_related_payload(
                            LogType::AggregatedSignatureFinished,
                            "Randomness signature aggregated successfully.",
                            self.chain_id,
                            &randomness_task.request_id,
                            TaskType::BLS(BLSTaskType::Randomness),
                            json!(randomness_task),
                            None
                        )
                    );

                    let id_address = self.id_address;

                    let block_cache = self.block_cache.clone();

                    let chain_identity = self.chain_identity.clone();

                    let randomness_signature_cache = self.randomness_signature_cache.clone();

                    self.ts.write().await.add_task(
                        ComponentTaskType::Subscriber(
                            self.chain_identity.read().await.get_chain_id(),
                            SubscriberType::RandomnessSignatureAggregation,
                        ),
                        async move {
                            let handler = GeneralFulfillRandomnessHandler {
                                id_address,
                                chain_identity,
                                block_cache,
                                randomness_signature_cache,
                                pc: PhantomData,
                            };

                            if let Err(e) = handler
                                .handle(
                                    group_index,
                                    randomness_task,
                                    signature.clone(),
                                    partial_signatures,
                                )
                                .await
                            {
                                error!("{:?}", e);
                            }
                        },
                    )?;
                }
                Err(e) => {
                    error!(
                        "{}",
                        build_task_related_payload(
                            LogType::AggregatedSignatureFailed,
                            &format!(
                                "Randomness signature aggregation failed with error: {:?}",
                                e
                            ),
                            self.chain_id,
                            &randomness_task.request_id,
                            TaskType::BLS(BLSTaskType::Randomness),
                            json!(randomness_task),
                            None
                        )
                    );
                }
            }
        }

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let chain_id = self.chain_id;

        let subscriber = Box::new(self);

        eq.write()
            .await
            .subscribe(Topic::ReadyToFulfillRandomnessTask(chain_id), subscriber);
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
    > DebuggableSubscriber for RandomnessSignatureAggregationSubscriber<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
}

#[cfg(feature = "unittest")]
mod tests {
    use super::*;
    use crate::{
        event::{ready_to_fulfill_randomness_task::ReadyToFulfillRandomnessTask, types::Topic},
        queue::event_queue::EventQueue,
        scheduler::dynamic::SimpleDynamicTaskScheduler,
        test_contracts::mockadapter::{deploy_mock_adapter, get_mock_adapter_at},
    };
    use arpa_core::{Config, GeneralMainChainIdentity, RandomnessTask, PartialSignature};
    use arpa_dal::{
        cache::{InMemoryBlockInfoCache, InMemorySignatureResultCache}, 
        BlockInfoHandler, BlockInfoUpdater, SignatureResultCacheHandler
    };
    use ethers::prelude::*;
    use std::{collections::BTreeMap, sync::Arc};
    use threshold_bls::{poly::Eval, schemes::bn254::{G2Curve, G2Scheme}};
    use tokio::sync::RwLock;

    const TEST_CHAIN_ID: u64 = 1;
    const TEST_BLOCK_HEIGHT: usize = 1000;
    const TEST_BLOCK_TIME: usize = 12;
    const TEST_SUBSCRIPTION_ID: u64 = 1;
    const TEST_GROUP_INDEX: u32 = 1;
    const TEST_SEED: usize = 12345;
    const TEST_REQUEST_CONFIRMATIONS: u16 = 6;
    const TEST_CALLBACK_GAS_LIMIT: u32 = 100000;
    const TEST_CALLBACK_MAX_GAS_PRICE: usize = 20000000000;
    const TEST_ASSIGNMENT_BLOCK_HEIGHT: usize = 100;
    const TEST_THRESHOLD: usize = 2;
    const HIGH_BLOCK_HEIGHT: usize = 100000;
    const LOW_GAS_PRICE: usize = 1;
    const SIGNATURE_SIZE: usize = 32;
    const RANDOMNESS_SIZE: usize = 96;
    const NUM_SIGNERS: usize = 3;
    const WS_ENDPOINT: &str = "ws://localhost:8545";

    async fn setup_anvil() -> (ethers::utils::AnvilInstance, Arc<Provider<Ws>>, LocalWallet) {
        let anvil = ethers::utils::Anvil::new().spawn();
        let ws_provider = Provider::<Ws>::connect(anvil.ws_endpoint())
            .await.expect("Failed to connect to anvil");
        let ws_provider = Arc::new(ws_provider);
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let wallet = wallet.with_chain_id(anvil.chain_id());
        (anvil, ws_provider, wallet)
    }

    async fn deploy_adapter(ws_provider: Arc<Provider<Ws>>, wallet: LocalWallet) -> Address {
        let client = SignerMiddleware::new((*ws_provider).clone(), wallet.clone());
        let client = Arc::new(client);
        deploy_mock_adapter(client.clone()).await.unwrap()
    }

    async fn create_chain_identity(
        chain_id: u64,
        wallet: LocalWallet,
        ws_provider: Arc<Provider<Ws>>,
        ws_endpoint: String,
        adapter_address: Address,
    ) -> Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> {
        let config = Config::default();
        let general_chain_identity = GeneralMainChainIdentity::new(
            chain_id.try_into().unwrap(),
            wallet.clone(),
            ws_provider.clone(),
            ws_endpoint,
            Address::random(),
            Address::random(), 
            adapter_address,
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );
        Arc::new(RwLock::new(Box::new(general_chain_identity)))
    }

    fn create_block_cache(
        chain_id: usize,
        block_height: usize,
        block_time: usize,
    ) -> Arc<RwLock<Box<dyn BlockInfoHandler>>> {
        let mut cache = InMemoryBlockInfoCache::new(chain_id, block_time);
        cache.set_block_height(block_height);
        Arc::new(RwLock::new(Box::new(cache)))
    }

    fn create_signature_cache() -> Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>> {
        let cache = InMemorySignatureResultCache::new();
        Arc::new(RwLock::new(Box::new(cache)))
    }

    fn create_schedulers() -> (Arc<RwLock<EventQueue>>, Arc<RwLock<SimpleDynamicTaskScheduler>>) {
        (
            Arc::new(RwLock::new(EventQueue::new())),
            Arc::new(RwLock::new(SimpleDynamicTaskScheduler::new()))
        )
    }

    fn create_test_randomness_task(request_id: Vec<u8>) -> RandomnessTask {
        let mut full_request_id = vec![0u8; 32];
        let copy_len = std::cmp::min(request_id.len(), 32);
        full_request_id[..copy_len].copy_from_slice(&request_id[..copy_len]);
        
        RandomnessTask {
            request_id: full_request_id, 
            subscription_id: TEST_SUBSCRIPTION_ID,
            group_index: TEST_GROUP_INDEX,
            request_type: arpa_core::RandomnessRequestType::Randomness,
            params: vec![],
            requester: Address::random(),
            seed: U256::from(TEST_SEED),
            request_confirmations: TEST_REQUEST_CONFIRMATIONS,
            callback_gas_limit: TEST_CALLBACK_GAS_LIMIT,
            callback_max_gas_price: U256::from(TEST_CALLBACK_MAX_GAS_PRICE), 
            assignment_block_height: TEST_ASSIGNMENT_BLOCK_HEIGHT,
        }
    }

    fn create_test_partial_signatures() -> BTreeMap<Address, PartialSignature> {
        let mut partial_signatures = BTreeMap::new();
        
        for i in 0..NUM_SIGNERS {
            let addr = Address::random();
            let eval = Eval {
                value: vec![i as u8; SIGNATURE_SIZE], 
                index: i as u32,
            };
            let serialized = bincode::serialize(&eval).unwrap();
            partial_signatures.insert(addr, PartialSignature {
                index: i,
                signed_partial_signature: serialized, 
            });
        }
        partial_signatures
    }

    async fn setup_signature_cache_with_task(
        cache: Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
        randomness_task: RandomnessTask,
        partial_signatures: BTreeMap<Address, PartialSignature>,
        committed_times: usize,
    ) {
        cache.write().await.add(
            randomness_task.group_index as usize,
            randomness_task.clone(),
            vec![1, 2, 3],
            TEST_THRESHOLD,
        ).await.unwrap();

        for (address, partial_sig) in partial_signatures {
            cache.write().await.add_partial_signature(
                randomness_task.request_id.clone(),
                address,
                partial_sig.index,
                partial_sig.signed_partial_signature,
            ).await.unwrap();
        }
        
        for _ in 0..committed_times {
            cache.write().await.incr_committed_times(&randomness_task.request_id).await.unwrap();
        }
    }

    async fn create_signature_cache_with_task(
        randomness_task: RandomnessTask,
        partial_signatures: BTreeMap<Address, PartialSignature>,
        committed_times: usize,
    ) -> Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>> {
        let cache = create_signature_cache();
        setup_signature_cache_with_task(cache.clone(), randomness_task, partial_signatures, committed_times).await;
        cache
    }

    #[tokio::test]
    async fn test_subscriber_creation() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let adapter_address = deploy_adapter(ws_provider.clone(), wallet.clone()).await;
        
        let chain_identity = create_chain_identity(
            TEST_CHAIN_ID, wallet, ws_provider, WS_ENDPOINT.to_string(), adapter_address,
        ).await;

        let block_cache = create_block_cache(TEST_CHAIN_ID as usize, TEST_BLOCK_HEIGHT, TEST_BLOCK_TIME);
        let signature_cache = create_signature_cache();
        let (eq, ts) = create_schedulers();
        let id_address = Address::random();

        let subscriber = RandomnessSignatureAggregationSubscriber::<G2Curve, G2Scheme>::new(
            TEST_CHAIN_ID.try_into().unwrap(), id_address, chain_identity, block_cache, signature_cache, eq, ts,
        );

        assert_eq!(subscriber.chain_id, TEST_CHAIN_ID  as usize);
        assert_eq!(subscriber.id_address, id_address);
    }

    #[tokio::test]
    async fn test_fulfill_randomness_handler_creation() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let adapter_address = deploy_adapter(ws_provider.clone(), wallet.clone()).await;
        
        let chain_identity = create_chain_identity(
            TEST_CHAIN_ID, wallet, ws_provider, WS_ENDPOINT.to_string(), adapter_address,
        ).await;

        let block_cache = create_block_cache(TEST_CHAIN_ID as usize, TEST_BLOCK_HEIGHT, TEST_BLOCK_TIME);
        let signature_cache = create_signature_cache();
        let id_address = Address::random();

        let handler = GeneralFulfillRandomnessHandler {
            id_address,
            chain_identity,
            block_cache,
            randomness_signature_cache: signature_cache,
            pc: PhantomData,
        };

        assert_eq!(handler.id_address, id_address);
    }

    #[tokio::test]
    async fn test_fulfill_randomness_handler_task_not_pending() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let adapter_address = deploy_adapter(ws_provider.clone(), wallet.clone()).await;
        
        let client = SignerMiddleware::new((*ws_provider).clone(), wallet.clone());
        let client = Arc::new(client);
        let adapter_contract = get_mock_adapter_at(adapter_address, client.clone());

        let chain_identity = create_chain_identity(
            TEST_CHAIN_ID, wallet, ws_provider, WS_ENDPOINT.to_string(), adapter_address,
        ).await;

        let block_cache = create_block_cache(TEST_CHAIN_ID as usize, TEST_BLOCK_HEIGHT, TEST_BLOCK_TIME);
        let randomness_task = create_test_randomness_task(vec![1, 2, 3, 4]);
        let partial_signatures = create_test_partial_signatures();
        let signature_cache = create_signature_cache_with_task(
            randomness_task.clone(), partial_signatures.clone(), 0,
        ).await;
        let id_address = Address::random();
        
        let request_id = H256::from_slice(&randomness_task.request_id);
        adapter_contract.set_request_commitment(request_id.into(), H256::zero().into()).send().await.unwrap().await.unwrap();

        let handler = GeneralFulfillRandomnessHandler {
            id_address,
            chain_identity,
            block_cache,
            randomness_signature_cache: signature_cache.clone(),
            pc: PhantomData,
        };

        let result = handler.handle(
            randomness_task.group_index as usize,
            randomness_task.clone(),
            vec![1; RANDOMNESS_SIZE],
            partial_signatures,
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fulfill_randomness_handler_task_expired() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let adapter_address = deploy_adapter(ws_provider.clone(), wallet.clone()).await;
        
        let client = SignerMiddleware::new((*ws_provider).clone(), wallet.clone());
        let client = Arc::new(client);
        let adapter_contract = get_mock_adapter_at(adapter_address, client.clone());

        let chain_identity = create_chain_identity(
            TEST_CHAIN_ID, wallet, ws_provider, WS_ENDPOINT.to_string(), adapter_address,
        ).await;

        let block_cache = create_block_cache(TEST_CHAIN_ID as usize, HIGH_BLOCK_HEIGHT, TEST_BLOCK_TIME);
        let randomness_task = create_test_randomness_task(vec![1, 2, 3, 4]);
        let partial_signatures = create_test_partial_signatures();
        let signature_cache = create_signature_cache_with_task(
            randomness_task.clone(), partial_signatures.clone(), 0,
        ).await;
        let id_address = Address::random();

        let request_id = H256::from_slice(&randomness_task.request_id);
        adapter_contract.set_request_commitment(request_id.into(), H256::from_low_u64_be(1).into()).send().await.unwrap().await.unwrap();

        let handler = GeneralFulfillRandomnessHandler {
            id_address,
            chain_identity,
            block_cache,
            randomness_signature_cache: signature_cache.clone(),
            pc: PhantomData,
        };

        let result = handler.handle(
            randomness_task.group_index as usize,
            randomness_task.clone(),
            vec![1; RANDOMNESS_SIZE],
            partial_signatures,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fulfill_randomness_handler_gas_price_too_high() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let adapter_address = deploy_adapter(ws_provider.clone(), wallet.clone()).await;
        
        let client = SignerMiddleware::new((*ws_provider).clone(), wallet.clone());
        let client = Arc::new(client);
        let adapter_contract = get_mock_adapter_at(adapter_address, client.clone());

        let chain_identity = create_chain_identity(
            TEST_CHAIN_ID, wallet, ws_provider, WS_ENDPOINT.to_string(), adapter_address,
        ).await;

        let block_cache = create_block_cache(TEST_CHAIN_ID as usize, TEST_BLOCK_HEIGHT, TEST_BLOCK_TIME);
        let mut randomness_task = create_test_randomness_task(vec![1, 2, 3, 4]);
        let partial_signatures = create_test_partial_signatures();
        let signature_cache = create_signature_cache_with_task(
            randomness_task.clone(), partial_signatures.clone(), 0,
        ).await;
        let id_address = Address::random();
        randomness_task.callback_max_gas_price = U256::from(LOW_GAS_PRICE);
        
        let request_id = H256::from_slice(&randomness_task.request_id);
        adapter_contract.set_request_commitment(request_id.into(), H256::from_low_u64_be(1).into()).send().await.unwrap().await.unwrap();

        let handler = GeneralFulfillRandomnessHandler {
            id_address,
            chain_identity,
            block_cache,
            randomness_signature_cache: signature_cache.clone(),
            pc: PhantomData,
        };

        let result = handler.handle(
            randomness_task.group_index as usize,
            randomness_task.clone(),
            vec![1; RANDOMNESS_SIZE],
            partial_signatures,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fulfill_randomness_handler_success() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let adapter_address = deploy_adapter(ws_provider.clone(), wallet.clone()).await;
        
        let client = SignerMiddleware::new((*ws_provider).clone(), wallet.clone());
        let client = Arc::new(client);
        let adapter_contract = get_mock_adapter_at(adapter_address, client.clone());

        let chain_identity = create_chain_identity(
            TEST_CHAIN_ID, wallet, ws_provider, WS_ENDPOINT.to_string(), adapter_address,
        ).await;

        let block_cache = create_block_cache(TEST_CHAIN_ID as usize, TEST_BLOCK_HEIGHT, TEST_BLOCK_TIME);
        let randomness_task = create_test_randomness_task(vec![1, 2, 3, 4]);
        let partial_signatures = create_test_partial_signatures();
        let signature_cache = create_signature_cache_with_task(
            randomness_task.clone(), partial_signatures.clone(), 0,
        ).await;
        let id_address = Address::random();
        
        let request_id = H256::from_slice(&randomness_task.request_id);
        adapter_contract.set_request_commitment(request_id.into(), H256::from_low_u64_be(1).into()).send().await.unwrap().await.unwrap();
        adapter_contract.set_should_revert(request_id.into(), false).send().await.unwrap().await.unwrap();
        adapter_contract.set_should_revert_with_custom_error(request_id.into(), false).send().await.unwrap().await.unwrap();

        let handler = GeneralFulfillRandomnessHandler {
            id_address,
            chain_identity,
            block_cache,
            randomness_signature_cache: signature_cache.clone(),
            pc: PhantomData,
        };

        let result = handler.handle(
            randomness_task.group_index as usize,
            randomness_task.clone(),
            vec![1; SIGNATURE_SIZE],
            partial_signatures,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fulfill_randomness_handler_transaction_failed() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let adapter_address = deploy_adapter(ws_provider.clone(), wallet.clone()).await;
        
        let client = SignerMiddleware::new((*ws_provider).clone(), wallet.clone());
        let client = Arc::new(client);
        let adapter_contract = get_mock_adapter_at(adapter_address, client.clone());

        let chain_identity = create_chain_identity(
            TEST_CHAIN_ID, wallet, ws_provider, WS_ENDPOINT.to_string(), adapter_address,
        ).await;

        let block_cache = create_block_cache(TEST_CHAIN_ID as usize, TEST_BLOCK_HEIGHT, TEST_BLOCK_TIME);
        let randomness_task = create_test_randomness_task(vec![1, 2, 3, 4]);
        let partial_signatures = create_test_partial_signatures();
        let signature_cache = create_signature_cache_with_task(
            randomness_task.clone(), partial_signatures.clone(), 0,
        ).await;
        let id_address = Address::random();
        let request_id = H256::from_slice(&randomness_task.request_id);
        adapter_contract.set_request_commitment(request_id.into(), H256::from_low_u64_be(1).into()).send().await.unwrap().await.unwrap();
        adapter_contract.set_should_revert(request_id.into(), true).send().await.unwrap().await.unwrap();

        let handler = GeneralFulfillRandomnessHandler {
            id_address,
            chain_identity,
            block_cache,
            randomness_signature_cache: signature_cache.clone(),
            pc: PhantomData,
        };

        let result = handler.handle(
            randomness_task.group_index as usize,
            randomness_task.clone(),
            vec![1; SIGNATURE_SIZE],
            partial_signatures,
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fulfill_randomness_handler_custom_error() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let adapter_address = deploy_adapter(ws_provider.clone(), wallet.clone()).await;
        
        let client = SignerMiddleware::new((*ws_provider).clone(), wallet.clone());
        let client = Arc::new(client);
        let adapter_contract = get_mock_adapter_at(adapter_address, client.clone());

        let chain_identity = create_chain_identity(
            TEST_CHAIN_ID, wallet, ws_provider, WS_ENDPOINT.to_string(), adapter_address,
        ).await;

        let block_cache = create_block_cache(TEST_CHAIN_ID as usize, TEST_BLOCK_HEIGHT, TEST_BLOCK_TIME);
        let randomness_task = create_test_randomness_task(vec![1, 2, 3, 4]);
        let partial_signatures = create_test_partial_signatures();
        let signature_cache = create_signature_cache_with_task(
            randomness_task.clone(), partial_signatures.clone(), 0,
        ).await;
        let id_address = Address::random();

        let request_id = H256::from_slice(&randomness_task.request_id);
        adapter_contract.set_request_commitment(request_id.into(), H256::from_low_u64_be(1).into()).send().await.unwrap().await.unwrap();
        adapter_contract.set_should_revert_with_custom_error(request_id.into(), true).send().await.unwrap().await.unwrap();

        let handler = GeneralFulfillRandomnessHandler {
            id_address,
            chain_identity,
            block_cache,
            randomness_signature_cache: signature_cache.clone(),
            pc: PhantomData,
        };

        let result = handler.handle(
            randomness_task.group_index as usize,
            randomness_task.clone(),
            vec![1; SIGNATURE_SIZE],
            partial_signatures,
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscriber_notify_with_faulty_task() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let adapter_address = deploy_adapter(ws_provider.clone(), wallet.clone()).await;
        
        let chain_identity = create_chain_identity(
            TEST_CHAIN_ID, wallet, ws_provider, WS_ENDPOINT.to_string(), adapter_address,
        ).await;

        let block_cache = create_block_cache(TEST_CHAIN_ID as usize, TEST_BLOCK_HEIGHT, TEST_BLOCK_TIME);
        let (eq, ts) = create_schedulers();
        let id_address = Address::random();

        let randomness_task = create_test_randomness_task(vec![1, 2, 3, 4]);
        let partial_signatures = create_test_partial_signatures();
        
        let signature_cache = create_signature_cache_with_task(
            randomness_task.clone(),
            partial_signatures.clone(),
            DEFAULT_MAX_RANDOMNESS_FULFILLMENT_ATTEMPTS,
        ).await;

        let subscriber = RandomnessSignatureAggregationSubscriber::<G2Curve, G2Scheme>::new(
            TEST_CHAIN_ID as usize, id_address, chain_identity, block_cache, signature_cache.clone(), eq, ts,
        );

        let result_cache = RandomnessResultCache {
            group_index: randomness_task.group_index as usize,
            randomness_task: randomness_task.clone(),
            message: vec![1, 2, 3],
            threshold: TEST_THRESHOLD,
            partial_signatures,
            committed_times: DEFAULT_MAX_RANDOMNESS_FULFILLMENT_ATTEMPTS, 
        };

        let event = ReadyToFulfillRandomnessTask {
            chain_id: TEST_CHAIN_ID.try_into().unwrap(),
            tasks: vec![result_cache],
        };

        let result = subscriber.notify(Topic::ReadyToFulfillRandomnessTask(TEST_CHAIN_ID as usize), &event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscriber_notify_with_valid_task() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let adapter_address = deploy_adapter(ws_provider.clone(), wallet.clone()).await;
        
        let chain_identity = create_chain_identity(
            TEST_CHAIN_ID, wallet, ws_provider, WS_ENDPOINT.to_string(), adapter_address,
        ).await;

        let block_cache = create_block_cache(TEST_CHAIN_ID as usize, TEST_BLOCK_HEIGHT, TEST_BLOCK_TIME);
        let signature_cache = create_signature_cache();
        let (eq, ts) = create_schedulers();
        let id_address = Address::random();

        let subscriber = RandomnessSignatureAggregationSubscriber::<G2Curve, G2Scheme>::new(
            TEST_CHAIN_ID as usize, id_address, chain_identity, block_cache, signature_cache.clone(), eq, ts,
        );

        let randomness_task = create_test_randomness_task(vec![1, 2, 3, 4]);
        let partial_signatures = create_test_partial_signatures();

        let result_cache = RandomnessResultCache {
            group_index: randomness_task.group_index as usize,
            randomness_task: randomness_task.clone(),
            message: vec![1, 2, 3],
            threshold: TEST_THRESHOLD,
            partial_signatures,
            committed_times: 0,
        };

        let event = ReadyToFulfillRandomnessTask {
            chain_id: TEST_CHAIN_ID as usize,
            tasks: vec![result_cache],
        };

        let result = subscriber.notify(Topic::ReadyToFulfillRandomnessTask(TEST_CHAIN_ID as usize), &event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscriber_subscribe() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let adapter_address = deploy_adapter(ws_provider.clone(), wallet.clone()).await;
        
        let chain_identity = create_chain_identity(
            TEST_CHAIN_ID, wallet, ws_provider, WS_ENDPOINT.to_string(), adapter_address,
        ).await;

        let block_cache = create_block_cache(TEST_CHAIN_ID as usize, TEST_BLOCK_HEIGHT, TEST_BLOCK_TIME);
        let signature_cache = create_signature_cache();
        let (eq, ts) = create_schedulers();
        let id_address = Address::random();

        let subscriber = RandomnessSignatureAggregationSubscriber::<G2Curve, G2Scheme>::new(
            TEST_CHAIN_ID as usize, id_address, chain_identity, block_cache, signature_cache, eq.clone(), ts,
        );

        subscriber.subscribe().await;
    }

    #[tokio::test]
    async fn test_subscriber_notify_with_multiple_tasks() {
        let (_anvil, ws_provider, wallet) = setup_anvil().await;
        let adapter_address = deploy_adapter(ws_provider.clone(), wallet.clone()).await;
        
        let chain_identity = create_chain_identity(
            TEST_CHAIN_ID, wallet, ws_provider, WS_ENDPOINT.to_string(), adapter_address,
        ).await;

        let block_cache = create_block_cache(TEST_CHAIN_ID as usize, TEST_BLOCK_HEIGHT, TEST_BLOCK_TIME);
        let signature_cache = create_signature_cache();
        let (eq, ts) = create_schedulers();
        let id_address = Address::random();

        let subscriber = RandomnessSignatureAggregationSubscriber::<G2Curve, G2Scheme>::new(
            TEST_CHAIN_ID as usize, id_address, chain_identity, block_cache, signature_cache.clone(), eq, ts,
        );

        let mut tasks = Vec::new();
        for i in 0..NUM_SIGNERS {
            let randomness_task = create_test_randomness_task(vec![i as u8, i as u8+1, i as u8+2, i as u8+3]);
            let partial_signatures = create_test_partial_signatures();

            let result_cache = RandomnessResultCache {
                group_index: randomness_task.group_index as usize,
                randomness_task: randomness_task.clone(),
                message: vec![i as u8, i as u8+1, i as u8+2],
                threshold: TEST_THRESHOLD,
                partial_signatures,
                committed_times: 0,
            };

            tasks.push(result_cache);
        }

        let event = ReadyToFulfillRandomnessTask { chain_id: TEST_CHAIN_ID as usize, tasks };

        let result = subscriber.notify(Topic::ReadyToFulfillRandomnessTask(TEST_CHAIN_ID as usize), &event).await;
        assert!(result.is_ok());
    }
}