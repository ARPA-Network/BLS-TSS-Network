use arpa_node_contract_client::{
    adapter::AdapterClientBuilder,
    controller::{ControllerClientBuilder, ControllerTransactions},
    coordinator::CoordinatorClientBuilder,
    provider::ChainProviderBuilder,
};
use arpa_node_core::{ChainIdentity, RandomnessTask};
use arpa_node_dal::{
    BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, MdcContextUpdater,
    NodeInfoFetcher, NodeInfoUpdater,
};

use super::{
    context::{
        chain::{ChainFetcher, MainChainFetcher},
        types::GeneralContext,
        ContextFetcher,
    },
    error::NodeResult,
};

pub mod server;

pub trait NodeService {
    async fn node_register(&self) -> NodeResult<()>;

    async fn node_activate(&self) -> NodeResult<()>;

    async fn node_quit(&self) -> NodeResult<()>;

    async fn shutdown_node(&self) -> NodeResult<()>;
}

impl<
        N: NodeInfoFetcher
            + NodeInfoUpdater
            + MdcContextUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher
            + GroupInfoUpdater
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
            + AdapterClientBuilder
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
    > NodeService for GeneralContext<N, G, T, I>
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
