use super::types::Subscriber;
use crate::node::{
    contract_client::controller_client::{ControllerTransactions, MockControllerClient},
    dal::types::ChainIdentity,
    error::errors::NodeResult,
    event::{
        dkg_post_process::DKGPostProcess,
        types::{Event, Topic},
    },
    queue::event_queue::{EventQueue, EventSubscriber},
    scheduler::dynamic::{DynamicTaskScheduler, SimpleDynamicTaskScheduler},
};
use async_trait::async_trait;
use log::{error, info};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct PostGroupingSubscriber {
    main_chain_identity: Arc<RwLock<ChainIdentity>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

impl PostGroupingSubscriber {
    pub fn new(
        main_chain_identity: Arc<RwLock<ChainIdentity>>,
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

pub struct MockDKGPostProcessHandler {
    controller_address: String,
    id_address: String,
}

#[async_trait]
impl DKGPostProcessHandler for MockDKGPostProcessHandler {
    async fn handle(&self, group_index: usize, group_epoch: usize) -> NodeResult<()> {
        let client =
            MockControllerClient::new(self.controller_address.clone(), self.id_address.clone());

        client.post_process_dkg(group_index, group_epoch).await?;

        Ok(())
    }
}

impl Subscriber for PostGroupingSubscriber {
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        info!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut DKGPostProcess;

            let DKGPostProcess {
                group_index,
                group_epoch,
            } = *Box::from_raw(struct_ptr);

            let controller_address = self
                .main_chain_identity
                .read()
                .get_provider_rpc_endpoint()
                .to_string();

            let id_address = self.main_chain_identity.read().get_id_address().to_string();

            self.ts.write().add_task(async move {
                let handler = MockDKGPostProcessHandler {
                    controller_address,
                    id_address,
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
