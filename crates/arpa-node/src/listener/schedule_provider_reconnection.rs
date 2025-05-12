use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    scheduler::{fixed::SimpleFixedTaskScheduler, FixedTaskScheduler},
};
use arpa_core::{
    jitter_fluctuate,
    log::{build_general_payload, LogType},
    ComponentTaskType, ListenerDescriptor, ListenerType,
};
use async_trait::async_trait;
use ethers::providers::Middleware;
use log::{debug, error};
use std::{marker::PhantomData, sync::Arc, time::Duration};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ProviderReconnectionListener<PC: Curve> {
    listener_descriptor: ListenerDescriptor,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    f_ts: Arc<RwLock<SimpleFixedTaskScheduler>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> std::fmt::Display for ProviderReconnectionListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProviderReconnectionListener")
    }
}

impl<PC: Curve> ProviderReconnectionListener<PC> {
    pub fn new(
        listener_descriptor: ListenerDescriptor,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        f_ts: Arc<RwLock<SimpleFixedTaskScheduler>>,
    ) -> Self {
        ProviderReconnectionListener {
            listener_descriptor,
            chain_identity,
            f_ts,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for ProviderReconnectionListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        self.chain_identity.write().await.reset_provider().await?;

        debug!(
            "{}",
            build_general_payload(
                LogType::ProviderReconnected,
                "Provider reconnected.",
                Some(self.listener_descriptor.chain_id),
            )
        );

        // try to get read lock, set timeout to prevent deadlock
        let f_ts_lock_result = tokio::time::timeout(Duration::from_secs(5), self.f_ts.read()).await;

        // if cannot get lock within timeout, it may be because the system is shutting down
        let f_ts_guard = match f_ts_lock_result {
            Ok(guard) => guard,
            Err(_) => {
                error!("Timeout while acquiring read lock on fixed task scheduler, possibly during shutdown");
                return Ok(());
            }
        };

        // restart listener tasks
        let tasks_to_restart: Vec<ComponentTaskType> = f_ts_guard
            .get_tasks()
            .into_iter()
            .filter(|task| match task {
                ComponentTaskType::Listener(chain_id, ListenerType::Block)
                | ComponentTaskType::Listener(chain_id, ListenerType::PreGrouping)
                | ComponentTaskType::Listener(chain_id, ListenerType::PostGrouping)
                | ComponentTaskType::Listener(chain_id, ListenerType::PostCommitGrouping)
                | ComponentTaskType::Listener(chain_id, ListenerType::NewRandomnessTask)
                | ComponentTaskType::Listener(
                    chain_id,
                    ListenerType::ReadyToHandleRandomnessTask,
                )
                | ComponentTaskType::Listener(chain_id, ListenerType::ScheduleNodeActivation) => {
                    *chain_id == self.listener_descriptor.chain_id
                }
                _ => false,
            })
            .cloned()
            .collect();

        // release read lock, avoid conflict with subsequent write lock
        drop(f_ts_guard);

        // restart each task separately
        for task in tasks_to_restart {
            debug!("Restarting listener: {}", task);

            // try to get write lock, set timeout
            let write_lock_result =
                tokio::time::timeout(Duration::from_secs(5), self.f_ts.write()).await;

            match write_lock_result {
                Ok(mut write_guard) => {
                    let _ = write_guard.restart_listener(&task);

                    debug!(
                        "{}",
                        build_general_payload(
                            LogType::ListenerRestarted,
                            &format!("Listener {} restarted.", task),
                            Some(self.listener_descriptor.chain_id),
                        )
                    );

                    // immediately release write lock, avoid holding it for a long time
                    drop(write_guard);
                }
                Err(_) => {
                    error!(
                        "Timeout while acquiring write lock to restart listener {}, possibly during shutdown",
                        task
                    );
                }
            }
        }

        Ok(())
    }

    async fn handle_interruption(&self) -> NodeResult<()> {
        self.chain_identity
            .read()
            .await
            .get_provider()
            .get_net_version()
            .await?;

        Ok(())
    }

    fn chain_id(&self) -> usize {
        self.listener_descriptor.chain_id
    }

    fn listener_descriptor(&self) -> ListenerDescriptor {
        self.listener_descriptor
    }

    fn jitter_fn(&self) -> Option<Box<dyn Fn(Duration) -> Duration + Send + Sync>> {
        Some(Box::new(move |duration: Duration| {
            jitter_fluctuate(duration, 0.2)
        }))
    }
}
