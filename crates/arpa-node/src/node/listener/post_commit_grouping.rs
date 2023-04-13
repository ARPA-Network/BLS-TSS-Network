use super::Listener;
use crate::node::{
    error::NodeResult,
    event::dkg_success::DKGSuccess,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_contract_client::controller::{ControllerClientBuilder, ControllerViews};
use arpa_node_core::{ChainIdentity, DKGStatus};
use arpa_node_dal::GroupInfoFetcher;
use async_trait::async_trait;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;

pub struct PostCommitGroupingListener<
    G: GroupInfoFetcher<PC>,
    I: ChainIdentity + ControllerClientBuilder<PC>,
    PC: PairingCurve,
> {
    main_chain_identity: Arc<RwLock<I>>,
    group_cache: Arc<RwLock<G>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<G: GroupInfoFetcher<PC>, I: ChainIdentity + ControllerClientBuilder<PC>, PC: PairingCurve>
    PostCommitGroupingListener<G, I, PC>
{
    pub fn new(
        main_chain_identity: Arc<RwLock<I>>,
        group_cache: Arc<RwLock<G>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PostCommitGroupingListener {
            main_chain_identity,
            group_cache,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher<PC> + Sync + Send,
        I: ChainIdentity + ControllerClientBuilder<PC> + Sync + Send,
        PC: PairingCurve + Send + Sync + 'static,
    > EventPublisher<DKGSuccess<PC>> for PostCommitGroupingListener<G, I, PC>
{
    async fn publish(&self, event: DKGSuccess<PC>) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        G: GroupInfoFetcher<PC> + Sync + Send,
        I: ChainIdentity + ControllerClientBuilder<PC> + Sync + Send,
        PC: PairingCurve + Sync + Send + 'static,
    > Listener for PostCommitGroupingListener<G, I, PC>
{
    async fn listen(&self) -> NodeResult<()> {
        let dkg_status = self.group_cache.read().await.get_dkg_status();

        if let Ok(DKGStatus::CommitSuccess) = dkg_status {
            let group_index = self.group_cache.read().await.get_index()?;

            let client = self
                .main_chain_identity
                .read()
                .await
                .build_controller_client();

            if let Ok(group) = client.get_group(group_index).await {
                if group.state {
                    self.publish(DKGSuccess { group }).await;
                }
            }
        }

        Ok(())
    }
}
