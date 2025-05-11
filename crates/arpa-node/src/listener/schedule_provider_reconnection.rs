use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    scheduler::{fixed::SimpleFixedTaskScheduler, FixedTaskScheduler},
};
use arpa_core::{
    log::{build_general_payload, LogType},
    ComponentTaskType, ListenerType,
};
use async_trait::async_trait;
use ethers::providers::Middleware;
use log::info;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ProviderReconnectionListener<PC: Curve> {
    chain_id: usize,
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
        chain_id: usize,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        f_ts: Arc<RwLock<SimpleFixedTaskScheduler>>,
    ) -> Self {
        ProviderReconnectionListener {
            chain_id,
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

        info!(
            "{}",
            build_general_payload(
                LogType::ProviderReconnected,
                "Provider reconnected.",
                Some(self.chain_id),
            )
        );

        let f_ts_guard = self.f_ts.read().await;

        // Restart the listener task again
        // for task in f_ts_guard.get_tasks() {
        //     match task {
        //         ComponentTaskType::Listener(chain_id, ListenerType::Block)
        //         | ComponentTaskType::Listener(chain_id, ListenerType::PreGrouping)
        //         | ComponentTaskType::Listener(chain_id, ListenerType::PostGrouping)
        //         | ComponentTaskType::Listener(chain_id, ListenerType::PostCommitGrouping)
        //         | ComponentTaskType::Listener(chain_id, ListenerType::NewRandomnessTask)
        //         | ComponentTaskType::Listener(
        //             chain_id,
        //             ListenerType::ReadyToHandleRandomnessTask,
        //         )
        //         | ComponentTaskType::Listener(
        //             chain_id,
        //             ListenerType::RandomnessSignatureAggregation,
        //         )
        //         | ComponentTaskType::Listener(chain_id, ListenerType::ScheduleNodeActivation) => {
        //             if *chain_id == self.chain_id {
        //                 let _ = self.f_ts.write().await.restart_listener(task);

        //                 info!(
        //                     "{}",
        //                     build_general_payload(
        //                         LogType::ListenerRestarted,
        //                         &format!("Listener {} restarted.", task),
        //                         Some(self.chain_id),
        //                     )
        //                 );
        //             }
        //         }
        //         _ => {}
        //     }
        // }

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

    async fn chain_id(&self) -> usize {
        self.chain_id
    }
}
