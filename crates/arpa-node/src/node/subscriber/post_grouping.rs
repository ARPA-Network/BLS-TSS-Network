use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::node::{
    error::NodeResult,
    event::{dkg_post_process::DKGPostProcess, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, SubscriberType, TaskScheduler, TaskType},
};
use arpa_node_contract_client::controller::{ControllerClientBuilder, ControllerTransactions};
use arpa_node_core::ChainIdentity;
use arpa_node_log::*;
use async_trait::async_trait;
use log::{debug, error, info};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PostGroupingSubscriber<I: ChainIdentity + ControllerClientBuilder> {
    main_chain_identity: Arc<RwLock<I>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl<I: ChainIdentity + ControllerClientBuilder> PostGroupingSubscriber<I> {
    pub fn new(
        main_chain_identity: Arc<RwLock<I>>,
        eq: Arc<RwLock<EventQueue>>,
        ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    ) -> Self {
        PostGroupingSubscriber {
            main_chain_identity,
            eq,
            ts,
        }
    }
}

#[async_trait]
pub trait DKGPostProcessHandler {
    async fn handle(&self, group_index: usize, group_epoch: usize) -> NodeResult<()>;
}

pub struct GeneralDKGPostProcessHandler<I: ChainIdentity + ControllerClientBuilder> {
    main_chain_identity: Arc<RwLock<I>>,
}

#[async_trait]
impl<I: ChainIdentity + ControllerClientBuilder + Sync + Send> DKGPostProcessHandler
    for GeneralDKGPostProcessHandler<I>
{
    #[log_function]
    async fn handle(&self, group_index: usize, group_epoch: usize) -> NodeResult<()> {
        let client = self
            .main_chain_identity
            .read()
            .await
            .build_controller_client();

        client.post_process_dkg(group_index, group_epoch).await?;

        Ok(())
    }
}

#[async_trait]
impl<I: ChainIdentity + ControllerClientBuilder + std::fmt::Debug + Sync + Send + 'static>
    Subscriber for PostGroupingSubscriber<I>
{
    #[log_function]
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let &DKGPostProcess {
            group_index,
            group_epoch,
        } = payload.as_any().downcast_ref::<DKGPostProcess>().unwrap();

        let main_chain_identity = self.main_chain_identity.clone();

        self.ts.write().await.add_task(TaskType::Subscriber(SubscriberType::PostGrouping),async move {
                let handler = GeneralDKGPostProcessHandler {
                    main_chain_identity
                };

                if let Err(e) = handler.handle(group_index, group_epoch).await {
                    error!("{:?}", e);
                } else {
                    info!("-------------------------call post process successfully-------------------------");
                }
            })?;

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write()
            .await
            .subscribe(Topic::DKGPostProcess, subscriber);
    }
}

impl<I: ChainIdentity + ControllerClientBuilder + std::fmt::Debug + Sync + Send + 'static>
    DebuggableSubscriber for PostGroupingSubscriber<I>
{
}
