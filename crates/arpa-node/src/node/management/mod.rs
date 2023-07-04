use std::collections::HashMap;

use super::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    committer::{client::GeneralCommitterClient, CommitterClient, CommitterService},
    context::{
        chain::{Chain, ChainFetcher, MainChainFetcher},
        types::GeneralContext,
        ContextFetcher,
    },
    error::{NodeError, NodeResult},
    scheduler::FixedTaskScheduler,
};
use anyhow::Result;
use arpa_node_contract_client::{
    adapter::{AdapterClientBuilder, AdapterTransactions},
    controller::{ControllerClientBuilder, ControllerTransactions},
    coordinator::CoordinatorClientBuilder,
    provider::ChainProviderBuilder,
};
use arpa_node_core::{
    BLSTaskType, ChainIdentity, DKGStatus, ExponentialBackoffRetryDescriptor, Group,
    ListenerDescriptor, ListenerType, PartialSignature, RandomnessTask, SchedulerResult, TaskType,
    DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_BASE, DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_FACTOR,
    DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_MAX_ATTEMPTS,
    DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_USE_JITTER, DEFAULT_LISTENER_INTERVAL_MILLIS,
};
use arpa_node_dal::{
    cache::RandomnessResultCache, error::DataAccessResult, BLSTasksFetcher, BLSTasksUpdater,
    ContextInfoUpdater, GroupInfoFetcher, GroupInfoUpdater, NodeInfoFetcher, NodeInfoUpdater,
    SignatureResultCacheFetcher, SignatureResultCacheUpdater,
};
use ethers::types::Address;
use threshold_bls::{group::PairingCurve, poly::Eval, sig::Share};

pub mod server;

pub mod client;

pub struct NodeInfo<PC: PairingCurve> {
    pub id_address: Address,
    pub node_rpc_endpoint: String,
    pub dkg_private_key: PC::Scalar,
    pub dkg_public_key: PC::G2,
}

pub struct GroupInfo<PC: PairingCurve> {
    pub share: Option<Share<PC::Scalar>>,
    pub group: Group<PC>,
    pub dkg_status: DKGStatus,
    pub self_index: usize,
    pub dkg_start_block_height: usize,
}

pub trait ServiceClient<C> {
    async fn prepare_service_client(&self) -> NodeResult<C>;
}

pub trait NodeService {
    async fn node_register(&self) -> NodeResult<()>;

    async fn node_activate(&self) -> NodeResult<()>;

    async fn node_quit(&self) -> NodeResult<()>;

    async fn shutdown_node(&self) -> NodeResult<()>;
}

pub trait ComponentService {
    async fn list_fixed_tasks(&self) -> SchedulerResult<Vec<TaskType>>;

    async fn start_listener(&self, task_type: ListenerType) -> SchedulerResult<()>;

    async fn shutdown_listener(&self, task_type: ListenerType) -> SchedulerResult<()>;
}
pub trait DBService<PC: PairingCurve> {
    async fn get_node_info(&self) -> DataAccessResult<NodeInfo<PC>>;

    async fn get_group_info(&self) -> DataAccessResult<GroupInfo<PC>>;
}

pub trait DKGService {
    async fn post_process_dkg(&self) -> NodeResult<()>;
}

pub trait BLSRandomnessService<PC: PairingCurve> {
    async fn partial_sign(
        &self,
        randomness_task_request_id: Vec<u8>,
        threshold: usize,
        msg: &[u8],
    ) -> Result<Vec<u8>>;

    fn aggregate_partial_sigs(&self, threshold: usize, partial_sigs: &[Vec<u8>])
        -> Result<Vec<u8>>;

    fn verify_sig(&self, public: &PC::G2, msg: &[u8], sig: &[u8]) -> Result<()>;

    fn verify_partial_sigs(
        &self,
        publics: &[PC::G2],
        msg: &[u8],
        partial_sigs: &[&[u8]],
    ) -> Result<()>;

    async fn send_partial_sig(
        &self,
        member_id_address: Address,
        msg: Vec<u8>,
        randomness_task_request_id: Vec<u8>,
        partial: Vec<u8>,
    ) -> Result<()>;

    async fn fulfill_randomness(
        &self,
        group_index: usize,
        randomness_task_request_id: Vec<u8>,
        sig: Vec<u8>,
        partial_sigs: HashMap<Address, Vec<u8>>,
    ) -> Result<()>;
}

impl<
        N: NodeInfoFetcher<PC>
            + NodeInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<PC>
            + GroupInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: SignatureResultCacheFetcher<RandomnessResultCache>
            + SignatureResultCacheUpdater<RandomnessResultCache>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder<PC>
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > NodeService for GeneralContext<N, G, T, C, I, PC>
{
    async fn node_register(&self) -> NodeResult<()> {
        let client = self
            .get_main_chain()
            .get_chain_identity()
            .read()
            .await
            .build_controller_client();

        let dkg_public_key = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_dkg_public_key()?
            .to_owned();

        client
            .node_register(bincode::serialize(&dkg_public_key).unwrap())
            .await?;

        Ok(())
    }

    async fn node_activate(&self) -> NodeResult<()> {
        todo!()
    }

    async fn node_quit(&self) -> NodeResult<()> {
        todo!()
    }

    async fn shutdown_node(&self) -> NodeResult<()> {
        // TODO shutdown gracefully
        std::process::exit(1);
    }
}

impl<
        N: NodeInfoFetcher<PC>
            + NodeInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<PC>
            + GroupInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: SignatureResultCacheFetcher<RandomnessResultCache>
            + SignatureResultCacheUpdater<RandomnessResultCache>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder<PC>
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > ComponentService for GeneralContext<N, G, T, C, I, PC>
{
    async fn list_fixed_tasks(&self) -> SchedulerResult<Vec<TaskType>> {
        Ok(self
            .get_fixed_task_handler()
            .read()
            .await
            .get_tasks()
            .into_iter()
            .cloned()
            .collect())
    }

    async fn start_listener(&self, task_type: ListenerType) -> SchedulerResult<()> {
        self.get_main_chain()
            .init_listener(
                self.get_event_queue(),
                self.get_fixed_task_handler(),
                ListenerDescriptor::build(task_type, DEFAULT_LISTENER_INTERVAL_MILLIS),
            )
            .await
    }

    async fn shutdown_listener(&self, task_type: ListenerType) -> SchedulerResult<()> {
        self.get_fixed_task_handler()
            .write()
            .await
            .abort(&TaskType::Listener(task_type))
            .await
    }
}

impl<
        N: NodeInfoFetcher<PC>
            + NodeInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<PC>
            + GroupInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: SignatureResultCacheFetcher<RandomnessResultCache>
            + SignatureResultCacheUpdater<RandomnessResultCache>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder<PC>
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > DBService<PC> for GeneralContext<N, G, T, C, I, PC>
{
    async fn get_node_info(&self) -> DataAccessResult<NodeInfo<PC>> {
        let id_address = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_id_address()?;
        let node_rpc_endpoint = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_node_rpc_endpoint()?
            .to_owned();
        let dkg_private_key = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_dkg_private_key()?
            .to_owned();
        let dkg_public_key = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_dkg_public_key()?
            .to_owned();

        Ok(NodeInfo {
            id_address,
            node_rpc_endpoint,
            dkg_private_key,
            dkg_public_key,
        })
    }

    async fn get_group_info(&self) -> DataAccessResult<GroupInfo<PC>> {
        let share = self
            .get_main_chain()
            .get_group_cache()
            .read()
            .await
            .get_secret_share()
            .map(|s| s.to_owned())
            .ok();
        let group = self
            .get_main_chain()
            .get_group_cache()
            .read()
            .await
            .get_group()?
            .to_owned();
        let dkg_status = self
            .get_main_chain()
            .get_group_cache()
            .read()
            .await
            .get_dkg_status()?;

        let self_index = self
            .get_main_chain()
            .get_group_cache()
            .read()
            .await
            .get_self_index()?;
        let dkg_start_block_height = self
            .get_main_chain()
            .get_group_cache()
            .read()
            .await
            .get_dkg_start_block_height()?;

        Ok(GroupInfo {
            share,
            group,
            dkg_status,
            self_index,
            dkg_start_block_height,
        })
    }
}

impl<
        N: NodeInfoFetcher<PC>
            + NodeInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<PC>
            + GroupInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: SignatureResultCacheFetcher<RandomnessResultCache>
            + SignatureResultCacheUpdater<RandomnessResultCache>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder<PC>
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > DKGService for GeneralContext<N, G, T, C, I, PC>
{
    async fn post_process_dkg(&self) -> NodeResult<()> {
        let client = self
            .get_main_chain()
            .get_chain_identity()
            .read()
            .await
            .build_controller_client();

        let group_index = self
            .get_main_chain()
            .get_group_cache()
            .read()
            .await
            .get_index()?;

        let group_epoch = self
            .get_main_chain()
            .get_group_cache()
            .read()
            .await
            .get_epoch()?;

        client.post_process_dkg(group_index, group_epoch).await?;

        Ok(())
    }
}

impl<
        N: NodeInfoFetcher<PC>
            + NodeInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<PC>
            + GroupInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: SignatureResultCacheFetcher<RandomnessResultCache>
            + SignatureResultCacheUpdater<RandomnessResultCache>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder<PC>
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > BLSRandomnessService<PC> for GeneralContext<N, G, T, C, I, PC>
{
    async fn partial_sign(
        &self,
        randomness_task_request_id: Vec<u8>,
        threshold: usize,
        msg: &[u8],
    ) -> Result<Vec<u8>> {
        let id_address = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_id_address()?;

        let partial_signature = SimpleBLSCore::<PC>::partial_sign(
            self.get_main_chain()
                .get_group_cache()
                .read()
                .await
                .get_secret_share()?,
            msg,
        )?;

        if self
            .get_main_chain()
            .get_group_cache()
            .read()
            .await
            .is_committer(id_address)?
        {
            let contained_res = self
                .get_main_chain()
                .get_randomness_result_cache()
                .read()
                .await
                .contains(&randomness_task_request_id)
                .await?;

            if !contained_res {
                let group_index = self
                    .get_main_chain()
                    .get_group_cache()
                    .read()
                    .await
                    .get_index()?;

                let task = self
                    .get_main_chain()
                    .get_randomness_tasks_cache()
                    .read()
                    .await
                    .get(&randomness_task_request_id)
                    .await?;

                self.get_main_chain()
                    .get_randomness_result_cache()
                    .write()
                    .await
                    .add(group_index, task, msg.to_vec(), threshold)
                    .await?;
            }

            self.get_main_chain()
                .get_randomness_result_cache()
                .write()
                .await
                .add_partial_signature(
                    randomness_task_request_id.clone(),
                    id_address,
                    partial_signature.clone(),
                )
                .await?;
        }

        Ok(partial_signature)
    }

    fn aggregate_partial_sigs(
        &self,
        threshold: usize,
        partial_sigs: &[Vec<u8>],
    ) -> Result<Vec<u8>> {
        SimpleBLSCore::<PC>::aggregate(threshold, partial_sigs)
    }

    fn verify_sig(&self, public: &PC::G2, msg: &[u8], sig: &[u8]) -> Result<()> {
        SimpleBLSCore::<PC>::verify(public, msg, sig)
    }

    fn verify_partial_sigs(
        &self,
        publics: &[PC::G2],
        msg: &[u8],
        partial_sigs: &[&[u8]],
    ) -> Result<()> {
        SimpleBLSCore::<PC>::verify_partial_sigs(publics, msg, partial_sigs)
    }

    async fn send_partial_sig(
        &self,
        member_id_address: Address,
        msg: Vec<u8>,
        randomness_task_request_id: Vec<u8>,
        partial: Vec<u8>,
    ) -> Result<()> {
        let id_address = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_id_address()?;

        let committer_id_address = self
            .get_main_chain()
            .get_group_cache()
            .read()
            .await
            .get_member(member_id_address)?
            .id_address;

        let endpoint = self
            .get_main_chain()
            .get_group_cache()
            .read()
            .await
            .get_member(member_id_address)?
            .rpc_endpoint
            .as_ref()
            .unwrap()
            .to_string();

        let commit_partial_signature_retry_descriptor = ExponentialBackoffRetryDescriptor {
            base: DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_BASE,
            factor: DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_FACTOR,
            max_attempts: DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_MAX_ATTEMPTS,
            use_jitter: DEFAULT_COMMIT_PARTIAL_SIGNATURE_RETRY_USE_JITTER,
        };

        let committer_client = GeneralCommitterClient::build(
            id_address,
            committer_id_address,
            endpoint,
            commit_partial_signature_retry_descriptor,
        );

        let chain_id = self
            .get_main_chain()
            .get_chain_identity()
            .read()
            .await
            .get_chain_id();

        committer_client
            .commit_partial_signature(
                chain_id,
                BLSTaskType::Randomness,
                randomness_task_request_id,
                msg,
                partial,
            )
            .await?;

        Ok(())
    }

    async fn fulfill_randomness(
        &self,
        group_index: usize,
        randomness_task_request_id: Vec<u8>,
        sig: Vec<u8>,
        partial_sigs: HashMap<Address, Vec<u8>>,
    ) -> Result<()> {
        let id_address = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_id_address()?;

        let client = self
            .get_main_chain()
            .get_chain_identity()
            .read()
            .await
            .build_adapter_client(id_address);

        let partial_signatures = partial_sigs
            .iter()
            .map(|(addr, partial)| {
                let eval: Eval<Vec<u8>> = bincode::deserialize(partial)?;
                let partial = PartialSignature {
                    index: eval.index as usize,
                    signature: eval.value,
                };
                Ok((*addr, partial))
            })
            .collect::<Result<_, NodeError>>()?;

        let randomness_task = self
            .get_main_chain()
            .get_randomness_tasks_cache()
            .read()
            .await
            .get(&randomness_task_request_id)
            .await?;

        client
            .fulfill_randomness(group_index, randomness_task, sig, partial_signatures)
            .await?;

        Ok(())
    }
}
