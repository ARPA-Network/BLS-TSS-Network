use std::collections::HashMap;

use super::{
    algorithm::bls::{BLSCore, SimpleBLSCore},
    committer::{client::GeneralCommitterClient, CommitterClient, CommitterService},
    context::{
        chain::{Chain, ChainFetcher, MainChainFetcher},
        types::GeneralContext,
        ContextFetcher,
    },
    error::NodeResult,
    scheduler::{FixedTaskScheduler, ListenerType, TaskType},
};
use anyhow::Result;
use arpa_node_contract_client::{
    adapter::{AdapterClientBuilder, AdapterTransactions},
    controller::{ControllerClientBuilder, ControllerTransactions},
    coordinator::CoordinatorClientBuilder,
    provider::ChainProviderBuilder,
};
use arpa_node_core::{
    BLSTaskError, ChainIdentity, DKGStatus, Group, RandomnessTask, SchedulerResult,
    TaskType as BLSTaskType,
};
use arpa_node_dal::{
    error::{DataAccessError, DataAccessResult},
    BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, MdcContextUpdater,
    NodeInfoFetcher, NodeInfoUpdater, SignatureResultCacheFetcher, SignatureResultCacheUpdater,
};
use ethers::types::Address;
use threshold_bls::{group::PairingCurve, sig::Share};

pub mod server;

pub struct NodeInfo<C: PairingCurve> {
    pub id_address: Address,
    pub node_rpc_endpoint: String,
    pub dkg_private_key: C::Scalar,
    pub dkg_public_key: C::G2,
}

pub struct GroupInfo<C: PairingCurve> {
    pub share: Option<Share<C::Scalar>>,
    pub group: Group<C>,
    pub dkg_status: DKGStatus,
    pub self_index: usize,
    pub dkg_start_block_height: usize,
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
pub trait DBService<C: PairingCurve> {
    async fn get_node_info(&self) -> DataAccessResult<NodeInfo<C>>;

    async fn get_group_info(&self) -> DataAccessResult<GroupInfo<C>>;
}

pub trait DKGService {
    async fn post_process_dkg(&self) -> NodeResult<()>;
}

pub trait BLSRandomnessService<C: PairingCurve> {
    async fn partial_sign(&self, sig_index: usize, threshold: usize, msg: &[u8])
        -> Result<Vec<u8>>;

    fn aggregate_partial_sigs(&self, threshold: usize, partial_sigs: &[Vec<u8>])
        -> Result<Vec<u8>>;

    fn verify_sig(&self, public: &C::G2, msg: &[u8], sig: &[u8]) -> Result<()>;

    fn verify_partial_sigs(
        &self,
        publics: &[C::G2],
        msg: &[u8],
        partial_sigs: &[&[u8]],
    ) -> Result<()>;

    async fn send_partial_sig(
        &self,
        member_id_address: Address,
        msg: Vec<u8>,
        sig_index: usize,
        partial: Vec<u8>,
    ) -> Result<()>;

    async fn fulfill_randomness(
        &self,
        group_index: usize,
        sig_index: usize,
        sig: Vec<u8>,
        partial_sigs: HashMap<Address, Vec<u8>>,
    ) -> Result<()>;
}

impl<
        N: NodeInfoFetcher<C>
            + NodeInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<C>
            + GroupInfoUpdater<C>
            + MdcContextUpdater
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
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<C>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > NodeService for GeneralContext<N, G, T, I, C>
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
        N: NodeInfoFetcher<C>
            + NodeInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<C>
            + GroupInfoUpdater<C>
            + MdcContextUpdater
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
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<C>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > ComponentService for GeneralContext<N, G, T, I, C>
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
                TaskType::Listener(task_type),
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
        N: NodeInfoFetcher<C>
            + NodeInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<C>
            + GroupInfoUpdater<C>
            + MdcContextUpdater
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
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<C>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > DBService<C> for GeneralContext<N, G, T, I, C>
{
    async fn get_node_info(&self) -> DataAccessResult<NodeInfo<C>> {
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

    async fn get_group_info(&self) -> DataAccessResult<GroupInfo<C>> {
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
        N: NodeInfoFetcher<C>
            + NodeInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<C>
            + GroupInfoUpdater<C>
            + MdcContextUpdater
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
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<C>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > DKGService for GeneralContext<N, G, T, I, C>
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
        N: NodeInfoFetcher<C>
            + NodeInfoUpdater<C>
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<C>
            + GroupInfoUpdater<C>
            + MdcContextUpdater
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
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<C>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > BLSRandomnessService<C> for GeneralContext<N, G, T, I, C>
{
    async fn partial_sign(
        &self,
        sig_index: usize,
        threshold: usize,
        msg: &[u8],
    ) -> Result<Vec<u8>> {
        let id_address = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_id_address()?;

        let partial_signature = SimpleBLSCore::<C>::partial_sign(
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
                .contains(sig_index);

            if !contained_res {
                let group_index = self
                    .get_main_chain()
                    .get_group_cache()
                    .read()
                    .await
                    .get_index()?;

                self.get_main_chain()
                    .get_randomness_result_cache()
                    .write()
                    .await
                    .add(
                        group_index,
                        sig_index,
                        String::from_utf8(msg.to_vec()).map_err(|e| {
                            let e: BLSTaskError = e.into();
                            let e: DataAccessError = e.into();
                            e
                        })?,
                        threshold,
                    )?;
            }

            self.get_main_chain()
                .get_randomness_result_cache()
                .write()
                .await
                .add_partial_signature(sig_index, id_address, partial_signature.clone())?;
        }

        Ok(partial_signature)
    }

    fn aggregate_partial_sigs(
        &self,
        threshold: usize,
        partial_sigs: &[Vec<u8>],
    ) -> Result<Vec<u8>> {
        SimpleBLSCore::<C>::aggregate(threshold, partial_sigs)
    }

    fn verify_sig(&self, public: &C::G2, msg: &[u8], sig: &[u8]) -> Result<()> {
        SimpleBLSCore::<C>::verify(public, msg, sig)
    }

    fn verify_partial_sigs(
        &self,
        publics: &[C::G2],
        msg: &[u8],
        partial_sigs: &[&[u8]],
    ) -> Result<()> {
        SimpleBLSCore::<C>::verify_partial_sigs(publics, msg, partial_sigs)
    }

    async fn send_partial_sig(
        &self,
        member_id_address: Address,
        msg: Vec<u8>,
        sig_index: usize,
        partial: Vec<u8>,
    ) -> Result<()> {
        let id_address = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_id_address()?;

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

        let committer_client = GeneralCommitterClient::build(id_address, endpoint);

        let chain_id = self
            .get_main_chain()
            .get_chain_identity()
            .read()
            .await
            .get_chain_id();

        committer_client
            .commit_partial_signature(chain_id, BLSTaskType::Randomness, msg, sig_index, partial)
            .await?;

        Ok(())
    }

    async fn fulfill_randomness(
        &self,
        group_index: usize,
        sig_index: usize,
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

        client
            .fulfill_randomness(group_index, sig_index, sig, partial_sigs)
            .await?;

        Ok(())
    }
}
