use super::Listener;
use crate::node::{
    error::{NodeError, NodeResult},
    event::new_group_relay_task::NewGroupRelayTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_node_contract_client::controller::{ControllerClientBuilder, ControllerLogs};
use arpa_node_core::{ChainIdentity, GroupRelayTask};
use arpa_node_dal::{BLSTasksFetcher, BLSTasksUpdater};
use async_trait::async_trait;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_retry::{strategy::FixedInterval, RetryIf};

pub struct NewGroupRelayTaskListener<
    I: ChainIdentity + ControllerClientBuilder,
    Q: BLSTasksUpdater<GroupRelayTask> + BLSTasksFetcher<GroupRelayTask>,
> {
    main_chain_identity: Arc<RwLock<I>>,
    group_relay_tasks_cache: Arc<RwLock<Q>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<
        I: ChainIdentity + ControllerClientBuilder,
        Q: BLSTasksUpdater<GroupRelayTask> + BLSTasksFetcher<GroupRelayTask>,
    > NewGroupRelayTaskListener<I, Q>
{
    pub fn new(
        main_chain_identity: Arc<RwLock<I>>,
        group_relay_tasks_cache: Arc<RwLock<Q>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        NewGroupRelayTaskListener {
            main_chain_identity,
            group_relay_tasks_cache,
            eq,
        }
    }
}

#[async_trait]
impl<
        I: ChainIdentity + ControllerClientBuilder + Sync + Send,
        Q: BLSTasksUpdater<GroupRelayTask> + BLSTasksFetcher<GroupRelayTask> + Sync + Send,
    > EventPublisher<NewGroupRelayTask> for NewGroupRelayTaskListener<I, Q>
{
    async fn publish(&self, event: NewGroupRelayTask) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<
        I: ChainIdentity + ControllerClientBuilder + Sync + Send,
        Q: BLSTasksUpdater<GroupRelayTask> + BLSTasksFetcher<GroupRelayTask> + Sync + Send + 'static,
    > Listener for NewGroupRelayTaskListener<I, Q>
{
    async fn start(mut self) -> NodeResult<()> {
        let client = self
            .main_chain_identity
            .read()
            .await
            .build_controller_client();

        let retry_strategy = FixedInterval::from_millis(2000);

        if let Err(err) = RetryIf::spawn(
            retry_strategy.clone(),
            || async {
                let group_relay_tasks_cache = self.group_relay_tasks_cache.clone();
                let eq = self.eq.clone();

                client
                    .subscribe_group_relay_task(move |group_relay_task| {
                        let group_relay_tasks_cache = group_relay_tasks_cache.clone();
                        let eq = eq.clone();

                        async move {
                            let contained_res = group_relay_tasks_cache
                                .read()
                                .await
                                .contains(group_relay_task.controller_global_epoch)
                                .await;
                            if let Ok(false) = contained_res {
                                info!("received new group relay task. {:?}", group_relay_task);

                                group_relay_tasks_cache
                                    .write()
                                    .await
                                    .add(group_relay_task.clone())
                                    .await
                                    .map_err(anyhow::Error::from)?;

                                eq.read()
                                    .await
                                    .publish(NewGroupRelayTask::new(group_relay_task))
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
