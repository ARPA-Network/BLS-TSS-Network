use super::Listener;
use crate::node::{
    contract_client::controller::{ControllerClientBuilder, ControllerLogs},
    dal::{types::GroupRelayTask, ChainIdentity},
    dal::{BLSTasksFetcher, BLSTasksUpdater},
    error::{NodeError, NodeResult},
    event::new_group_relay_task::NewGroupRelayTask,
    queue::{event_queue::EventQueue, EventPublisher},
};
use async_trait::async_trait;
use log::{error, info};
use parking_lot::RwLock;
use std::sync::Arc;
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

impl<
        I: ChainIdentity + ControllerClientBuilder,
        Q: BLSTasksUpdater<GroupRelayTask> + BLSTasksFetcher<GroupRelayTask>,
    > EventPublisher<NewGroupRelayTask> for NewGroupRelayTaskListener<I, Q>
{
    fn publish(&self, event: NewGroupRelayTask) {
        self.eq.read().publish(event);
    }
}

#[async_trait]
impl<
        I: ChainIdentity + ControllerClientBuilder + Sync + Send,
        Q: BLSTasksUpdater<GroupRelayTask> + BLSTasksFetcher<GroupRelayTask> + Sync + Send + 'static,
    > Listener for NewGroupRelayTaskListener<I, Q>
{
    async fn start(mut self) -> NodeResult<()> {
        let client = self.main_chain_identity.read().build_controller_client();

        let retry_strategy = FixedInterval::from_millis(2000);

        if let Err(err) = RetryIf::spawn(
            retry_strategy.clone(),
            || async {
                let group_relay_tasks_cache = self.group_relay_tasks_cache.clone();
                let eq = self.eq.clone();

                client
                    .subscribe_group_relay_task(Box::new(move |group_relay_task| {
                        if let Ok(false) = group_relay_tasks_cache
                            .read()
                            .contains(group_relay_task.controller_global_epoch)
                        {
                            info!("received new group relay task. {:?}", group_relay_task);

                            group_relay_tasks_cache
                                .write()
                                .add(group_relay_task.clone())?;

                            eq.read().publish(NewGroupRelayTask::new(group_relay_task));
                        }
                        Ok(())
                    }))
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
