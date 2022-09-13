use super::Subscriber;
use crate::node::{
    contract_client::controller::{ControllerClientBuilder, ControllerTransactions},
    dal::ChainIdentity,
    error::NodeResult,
    event::{dkg_post_process::DKGPostProcess, types::Topic, Event},
    queue::{event_queue::EventQueue, EventSubscriber},
    scheduler::{dynamic::SimpleDynamicTaskScheduler, TaskScheduler},
};
use async_trait::async_trait;
use log::{error, info};
use parking_lot::RwLock;
use std::sync::Arc;

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
    async fn handle(&self, group_index: usize, group_epoch: usize) -> NodeResult<()> {
        let client = self.main_chain_identity.read().build_controller_client();

        client.post_process_dkg(group_index, group_epoch).await?;

        Ok(())
    }
}

impl<I: ChainIdentity + ControllerClientBuilder + Sync + Send + 'static> Subscriber
    for PostGroupingSubscriber<I>
{
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        info!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut DKGPostProcess;

            let DKGPostProcess {
                group_index,
                group_epoch,
            } = *Box::from_raw(struct_ptr);

            let main_chain_identity = self.main_chain_identity.clone();

            self.ts.write().add_task(async move {
                let handler = GeneralDKGPostProcessHandler {
                    main_chain_identity
                };

                if let Err(e) = handler.handle(group_index, group_epoch).await {
                    error!("{:?}", e);
                } else {
                    info!("-------------------------call post process successfully-------------------------");
                }
            });
        }

        Ok(())
    }

    fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write().subscribe(Topic::DKGPostProcess, subscriber);
    }
}
