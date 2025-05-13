use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::dkg_post_process::DKGPostProcess,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::controller::ControllerViews;
use arpa_core::{DKGStatus, ListenerDescriptor};
use arpa_dal::{BlockInfoHandler, GroupInfoHandler};
use async_trait::async_trait;
use log::info;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PostGroupingListener<PC: Curve> {
    listener_descriptor: ListenerDescriptor,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
    dkg_timeout_duration: usize,
}

impl<PC: Curve> std::fmt::Display for PostGroupingListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PostGroupingListener")
    }
}

impl<PC: Curve> PostGroupingListener<PC> {
    pub fn new(
        listener_descriptor: ListenerDescriptor,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
        dkg_timeout_duration: usize,
    ) -> Self {
        PostGroupingListener {
            listener_descriptor,
            chain_identity,
            block_cache,
            group_cache,
            eq,
            pc: PhantomData,
            dkg_timeout_duration,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send + 'static> EventPublisher<DKGPostProcess<PC>>
    for PostGroupingListener<PC>
{
    async fn publish(&self, event: DKGPostProcess<PC>) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send + 'static> Listener for PostGroupingListener<PC> {
    async fn listen(&self) -> NodeResult<()> {
        let dkg_status = self.group_cache.read().await.get_dkg_status();

        if let Ok(dkg_status) = dkg_status {
            match dkg_status {
                DKGStatus::None => {}
                DKGStatus::InPhase | DKGStatus::CommitSuccess | DKGStatus::WaitForPostProcess => {
                    let dkg_start_block_height =
                        self.group_cache.read().await.get_dkg_start_block_height()?;

                    let block_height = self.block_cache.read().await.get_block_height();

                    let dkg_timeout_block_height =
                        dkg_start_block_height + self.dkg_timeout_duration;

                    info!("checking post process... dkg_start_block_height: {}, current_block_height: {}, timeuout_dkg_block_height: {}",
                    dkg_start_block_height,block_height,dkg_timeout_block_height);

                    if block_height > dkg_timeout_block_height {
                        let group_index = self.group_cache.read().await.get_index().unwrap_or(0);

                        let group_epoch = self.group_cache.read().await.get_epoch().unwrap_or(0);

                        let client = self.chain_identity.read().await.build_controller_client();

                        if let Ok(group) = client.get_group(group_index).await {
                            self.publish(DKGPostProcess {
                                group_index,
                                group_epoch,
                                group,
                            })
                            .await;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_interruption(&self) -> NodeResult<()> {
        Ok(())
    }

    fn chain_id(&self) -> usize {
        self.listener_descriptor.chain_id
    }

    fn listener_descriptor(&self) -> ListenerDescriptor {
        self.listener_descriptor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use crate::event::types::Topic;
    use crate::queue::EventSubscriber;
    use crate::subscriber::{DebuggableEvent, DebuggableSubscriber, Subscriber};
    use arpa_core::DKGStatus;
    use arpa_dal::cache::{InMemoryBlockInfoCache, InMemoryGroupInfoCache};
    use ethers::types::Address;
    use std::time::Duration;
    use tokio::time::timeout;
    use anyhow::anyhow;

    async fn mock_subscribe_to_events(
        eq: &mut EventQueue,
        subscriber_name: &str,
    ) -> tokio::sync::mpsc::Receiver<Box<dyn std::any::Any + Send>> {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        
        struct TestSubscriber {
            name: String,
            sender: tokio::sync::mpsc::Sender<Box<dyn std::any::Any + Send>>,
        }
        
        impl std::fmt::Debug for TestSubscriber {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "TestSubscriber({})", self.name)
            }
        }
        
        #[async_trait]
        impl Subscriber for TestSubscriber {
            async fn notify(&self, _topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()> {
                if let Some(post_process_event) = payload.as_any().downcast_ref::<DKGPostProcess>() {
                    let cloned_event = DKGPostProcess {
                        group_index: post_process_event.group_index,
                        group_epoch: post_process_event.group_epoch,
                    };
                    let boxed = Box::new(cloned_event) as Box<dyn std::any::Any + Send>;
                    self.sender.send(boxed).await.map_err(|e| {
                        let err: crate::error::NodeError = anyhow!("Failed to send event: {}", e).into();
                        err
                    })?;
                }
                Ok(())
            }
            
            async fn subscribe(self) {
            }
        }
        
        impl DebuggableSubscriber for TestSubscriber {}
        
        let subscriber = TestSubscriber {
            name: subscriber_name.to_string(),
            sender,
        };

        let dummy_event = DKGPostProcess { 
            group_index: 1, 
            group_epoch: 1,
        };
        
        let topic = dummy_event.topic();
        
        eq.subscribe(topic, Box::new(subscriber));
        
        receiver
    }
    
    #[tokio::test]
    async fn test_post_grouping_listener_timeout() -> NodeResult<()> {
        let chain_id = 1;
        let id_address = Address::random();
        let dkg_timeout_duration = 100;
        
        let block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>> = Arc::new(RwLock::new(
            Box::new(InMemoryBlockInfoCache::new(chain_id, 15)),
        ));
        
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<threshold_bls::schemes::bn254::G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<threshold_bls::schemes::bn254::G2Curve>::new(id_address)),
        ));
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        
        {
            let mut group_cache_write = group_cache.write().await;
            let dkg_task = arpa_core::DKGTask {
                group_index: 1,
                epoch: 1,
                size: 3,
                threshold: 2,
                assignment_block_height: 50,
                members: vec![id_address, Address::random(), Address::random()],
                coordinator_address: Address::random()
            };
            
            group_cache_write.save_task_info(0, dkg_task).await?;
            group_cache_write.update_dkg_status(1, 1, DKGStatus::InPhase).await?;
            assert_eq!(group_cache_write.get_index().unwrap(), 1);
            assert_eq!(group_cache_write.get_epoch().unwrap(), 1);
            assert_eq!(group_cache_write.get_dkg_start_block_height().unwrap(), 50);
        }
        
        {
            let mut block_cache_write = block_cache.write().await;
            block_cache_write.set_block_height(200);
            assert_eq!(block_cache_write.get_block_height(), 200);
        }
        
        let listener = PostGroupingListener::<threshold_bls::schemes::bn254::G2Curve>::new(
            block_cache.clone(),
            group_cache.clone(),
            event_queue.clone(),
            dkg_timeout_duration,
        );
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };
        
        listener.listen().await?;
        
        let received_event = timeout(Duration::from_secs(1), event_receiver.recv()).await
            .map_err(|_| anyhow!("Timeout: No event received"))?
            .ok_or_else(|| anyhow!("Error: Event channel closed"))?;
        
        if let Some(post_process_event) = received_event.downcast_ref::<DKGPostProcess>() {
            assert_eq!(post_process_event.group_index, 1);
            assert_eq!(post_process_event.group_epoch, 1);
        } else {
            return Err(anyhow!("Received unexpected event type").into());
        }
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_post_grouping_listener_no_timeout() -> NodeResult<()> {
        let chain_id = 1;
        let id_address = Address::random();
        let dkg_timeout_duration = 100;
        
        let block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>> = Arc::new(RwLock::new(
            Box::new(InMemoryBlockInfoCache::new(chain_id,15)),
        ));
        
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<threshold_bls::schemes::bn254::G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<threshold_bls::schemes::bn254::G2Curve>::new(id_address)),
        ));
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        
        {
            let mut group_cache_write = group_cache.write().await;
            let dkg_task = arpa_core::DKGTask {
                group_index: 1,
                epoch: 1,
                size: 3,
                threshold: 2,
                assignment_block_height: 50,
                members: vec![id_address, Address::random(), Address::random()],
                coordinator_address: Address::random()
            };
            
            group_cache_write.save_task_info(0, dkg_task).await?;
            group_cache_write.update_dkg_status(1, 1, DKGStatus::InPhase).await?;
            assert_eq!(group_cache_write.get_index().unwrap(), 1);
            assert_eq!(group_cache_write.get_epoch().unwrap(), 1);
            assert_eq!(group_cache_write.get_dkg_start_block_height().unwrap(), 50);
        }
        
        {
            let mut block_cache_write = block_cache.write().await;
            block_cache_write.set_block_height(100);
            assert_eq!(block_cache_write.get_block_height(), 100);
        }
        
        let listener = PostGroupingListener::<threshold_bls::schemes::bn254::G2Curve>::new(
            block_cache.clone(),
            group_cache.clone(),
            event_queue.clone(),
            dkg_timeout_duration,
        );
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };
        
        listener.listen().await?;
        
        let timeout_result = timeout(Duration::from_millis(100), event_receiver.recv()).await;
        assert!(timeout_result.is_err(), "Unexpectedly received an event when DKG is not timed out");
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_post_grouping_listener_dkg_none() -> NodeResult<()> {
        let chain_id = 1;
        let id_address = Address::random();
        let dkg_timeout_duration = 100;
        
        let block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>> = Arc::new(RwLock::new(
            Box::new(InMemoryBlockInfoCache::new(chain_id, 15)),
        ));
        
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<threshold_bls::schemes::bn254::G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<threshold_bls::schemes::bn254::G2Curve>::new(id_address)),
        ));
        
        let event_queue = Arc::new(RwLock::new(EventQueue::new()));
        
        {
            let mut group_cache_write = group_cache.write().await;
            let dkg_task = arpa_core::DKGTask {
                group_index: 1,
                epoch: 1,
                size: 3,
                threshold: 2,
                assignment_block_height: 50,
                members: vec![id_address, Address::random(), Address::random()],
                coordinator_address: Address::random()
            };
            
            group_cache_write.save_task_info(0, dkg_task).await?;
            group_cache_write.update_dkg_status(1, 1, DKGStatus::None).await?;
            
            assert_eq!(group_cache_write.get_index().unwrap(), 1);
            assert_eq!(group_cache_write.get_epoch().unwrap(), 1);
            assert_eq!(group_cache_write.get_dkg_status().unwrap(), DKGStatus::None);
        }
        
        {
            let mut block_cache_write = block_cache.write().await;
            block_cache_write.set_block_height(200);
        }
        
        let listener = PostGroupingListener::<threshold_bls::schemes::bn254::G2Curve>::new(
            block_cache.clone(),
            group_cache.clone(),
            event_queue.clone(),
            dkg_timeout_duration,
        );
        
        let mut event_receiver = {
            let mut eq_write = event_queue.write().await;
            mock_subscribe_to_events(&mut *eq_write, "test_subscriber").await
        };
        
        listener.listen().await?;
        
        let timeout_result = timeout(Duration::from_millis(100), event_receiver.recv()).await;
        assert!(timeout_result.is_err(), "Unexpectedly received an event when DKG status is None");
        
        Ok(())
    }
}
