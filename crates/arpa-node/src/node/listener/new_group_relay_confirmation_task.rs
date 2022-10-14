use super::Listener;
use crate::node::{
    error::{NodeError, NodeResult},
    event::new_group_relay_confirmation_task::NewGroupRelayConfirmationTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_contract_client::adapter::{AdapterClientBuilder, AdapterLogs};
use arpa_node_core::{ChainIdentity, GroupRelayConfirmationTask};
use arpa_node_dal::{BLSTasksFetcher, BLSTasksUpdater};
use async_trait::async_trait;
use ethers::types::Address;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct NewGroupRelayConfirmationTaskListener<
    I: ChainIdentity + AdapterClientBuilder,
    Q: BLSTasksUpdater<GroupRelayConfirmationTask> + BLSTasksFetcher<GroupRelayConfirmationTask>,
> {
    chain_id: usize,
    id_address: Address,
    chain_identity: Arc<RwLock<I>>,
    group_relay_confirmation_tasks_cache: Arc<RwLock<Q>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<
        I: ChainIdentity + AdapterClientBuilder,
        Q: BLSTasksUpdater<GroupRelayConfirmationTask> + BLSTasksFetcher<GroupRelayConfirmationTask>,
    > NewGroupRelayConfirmationTaskListener<I, Q>
{
    pub fn new(
        chain_id: usize,
        id_address: Address,
        chain_identity: Arc<RwLock<I>>,
        group_relay_confirmation_tasks_cache: Arc<RwLock<Q>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        NewGroupRelayConfirmationTaskListener {
            chain_id,
            id_address,
            chain_identity,
            group_relay_confirmation_tasks_cache,
            eq,
        }
    }
}

#[async_trait]
impl<
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
        Q: BLSTasksUpdater<GroupRelayConfirmationTask>
            + BLSTasksFetcher<GroupRelayConfirmationTask>
            + Sync
            + Send,
    > EventPublisher<NewGroupRelayConfirmationTask>
    for NewGroupRelayConfirmationTaskListener<I, Q>
{
    async fn publish(&self, event: NewGroupRelayConfirmationTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        I: ChainIdentity + AdapterClientBuilder + Sync + Send,
        Q: BLSTasksUpdater<GroupRelayConfirmationTask>
            + BLSTasksFetcher<GroupRelayConfirmationTask>
            + Sync
            + Send
            + 'static,
    > Listener for NewGroupRelayConfirmationTaskListener<I, Q>
{
    async fn start(mut self) -> NodeResult<()> {
        let client = self
            .chain_identity
            .read()
            .await
            .build_adapter_client(self.id_address);

        let retry_strategy = FixedInterval::from_millis(2000);

        if let Err(err) = RetryIf::spawn(
            retry_strategy.clone(),
            || async {
                let chain_id = self.chain_id;
                let group_relay_confirmation_tasks_cache =
                    self.group_relay_confirmation_tasks_cache.clone();
                let eq = self.eq.clone();

                client
                    .subscribe_group_relay_confirmation_task(move |group_relay_confirmation_task| {
                        let group_relay_confirmation_tasks_cache =
                            group_relay_confirmation_tasks_cache.clone();
                        let eq = eq.clone();

                        async move {
                            if let Ok(false) = group_relay_confirmation_tasks_cache
                                .read()
                                .await
                                .contains(group_relay_confirmation_task.index)
                                .await
                            {
                                info!(
                                    "received new group_relay_confirmation task. {:?}",
                                    group_relay_confirmation_task
                                );

                                group_relay_confirmation_tasks_cache
                                    .write()
                                    .await
                                    .add(group_relay_confirmation_task.clone())
                                    .await
                                    .map_err(anyhow::Error::from)?;

                                eq.read()
                                    .await
                                    .publish(NewGroupRelayConfirmationTask::new(
                                        chain_id,
                                        group_relay_confirmation_task,
                                    ))
                                    .await;
                            }
                            Ok(())
                        }
                    })
                    .await?;

                Ok(())
            },
            |e: &NodeError| {
                error!("listener is interrupted. Retry... Error: {:?}, ", e);
                true
            },
        )
        .await
        {
            error!("{:?}", err);
        }

        Ok(())
    }
}
