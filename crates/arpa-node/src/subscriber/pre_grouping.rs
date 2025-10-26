use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    error::NodeResult,
    event::{new_dkg_task::NewDKGTask, run_dkg::RunDKG, types::Topic},
    queue::{event_queue::EventQueue, EventPublisher, EventSubscriber},
};
use arpa_core::{
    log::{build_group_related_payload, LogType},
    DKGStatus,
};
use arpa_dal::GroupInfoHandler;
use async_trait::async_trait;
use log::{debug, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PreGroupingSubscriber<PC: Curve> {
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    c: PhantomData<PC>,
}

impl<PC: Curve> PreGroupingSubscriber<PC> {
    pub fn new(
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PreGroupingSubscriber {
            group_cache,
            eq,
            c: PhantomData,
        }
    }
}

#[async_trait]
impl<C: Curve + std::fmt::Debug + Sync + Send> EventPublisher<RunDKG> for PreGroupingSubscriber<C> {
    async fn publish(&self, event: RunDKG) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<C: Curve + std::fmt::Debug + Sync + Send + 'static> Subscriber for PreGroupingSubscriber<C> {
    async fn notify(&self, topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()> {
        debug!("{:?}", topic);

        let NewDKGTask {
            chain_id,
            dkg_task,
            self_index,
        } = payload
            .as_any()
            .downcast_ref::<NewDKGTask>()
            .unwrap()
            .clone();

        let cache_index = self.group_cache.read().await.get_index().unwrap_or(0);

        let cache_epoch = self.group_cache.read().await.get_epoch().unwrap_or(0);

        let task_group_index = dkg_task.group_index;

        let task_epoch = dkg_task.epoch;

        if cache_index != task_group_index || cache_epoch != task_epoch {
            self.group_cache
                .write()
                .await
                .save_task_info(self_index, dkg_task.clone())
                .await?;

            let res = self
                .group_cache
                .write()
                .await
                .update_dkg_status(task_group_index, task_epoch, DKGStatus::InPhase)
                .await?;

            if res {
                self.publish(RunDKG { dkg_task }).await;

                info!(
                    "{}",
                    build_group_related_payload(
                        LogType::DKGGroupingStarted,
                        "start handling new DKG task.",
                        chain_id,
                        self.group_cache.read().await.get_group()?
                    )
                );
            }
        }

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write().await.subscribe(Topic::NewDKGTask, subscriber);
    }
}

impl<C: Curve + std::fmt::Debug + Sync + Send + 'static> DebuggableSubscriber
    for PreGroupingSubscriber<C>
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::{new_dkg_task::NewDKGTask, run_dkg::RunDKG, types::Topic, Event},
        queue::{event_queue::EventQueue, EventPublisher},
    };
    use arpa_core::DKGTask;
    use arpa_dal::{GroupInfoHandler, cache::InMemoryGroupInfoCache};
    use alloy::primitives::{Address};
    use std::{any::Any, sync::Arc};
    use threshold_bls::schemes::bn254::G2Curve;
    use tokio::sync::RwLock;

    const CHAIN_ID: u64 = 1;
    const GROUP_SIZE: usize = 3;
    const THRESHOLD: usize = 2;
    const ASSIGNMENT_BLOCK_HEIGHT: usize = 100;
    const SELF_INDEX: usize = 0;

    fn create_test_dkg_task(group_index: usize, epoch: usize) -> DKGTask {
        DKGTask {
            group_index,
            epoch,
            size: GROUP_SIZE,
            threshold: THRESHOLD,
            members: vec![Address::ZERO, Address::ZERO, Address::ZERO],
            assignment_block_height: ASSIGNMENT_BLOCK_HEIGHT,
            coordinator_address: Address::ZERO,
        }
    }

    fn create_test_new_dkg_task(group_index: usize, epoch: usize) -> NewDKGTask {
        NewDKGTask {
            chain_id: CHAIN_ID,
            dkg_task: create_test_dkg_task(group_index, epoch),
            self_index: SELF_INDEX,
        }
    }

    fn create_subscriber() -> (PreGroupingSubscriber<G2Curve>, Arc<RwLock<EventQueue>>) {
        let id_address = Address::ZERO;
        let group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> = Arc::new(RwLock::new(
            Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address)),
        ));
        let eq = Arc::new(RwLock::new(EventQueue::new()));
        let subscriber = PreGroupingSubscriber::new(group_cache, eq.clone());
        (subscriber, eq)
    }

    #[tokio::test]
    async fn test_pre_grouping_subscriber_creation() {
        let (subscriber, _) = create_subscriber();
        assert!(format!("{:?}", subscriber).contains("PreGroupingSubscriber"));
    }

    #[tokio::test]
    async fn test_notify_with_different_group_index() {
        let (subscriber, _) = create_subscriber();
        let new_dkg_task = create_test_new_dkg_task(1, 1);
        let result = subscriber.notify(Topic::NewDKGTask, &new_dkg_task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_with_different_epoch() {
        let (subscriber, _) = create_subscriber();
        let new_dkg_task = create_test_new_dkg_task(0, 1);
        let result = subscriber.notify(Topic::NewDKGTask, &new_dkg_task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_with_same_group_index_and_epoch() {
        let (subscriber, _) = create_subscriber();
        let new_dkg_task = create_test_new_dkg_task(0, 0);
        let result = subscriber.notify(Topic::NewDKGTask, &new_dkg_task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_publish_functionality() {
        let (subscriber, _) = create_subscriber();
        let run_dkg_event = RunDKG {
            dkg_task: create_test_dkg_task(1, 1),
        };
        subscriber.publish(run_dkg_event).await;
    }

    #[tokio::test]
    async fn test_subscribe() {
        let (subscriber, _) = create_subscriber();
        subscriber.subscribe().await;
    }

    #[tokio::test]
    async fn test_event_publisher_trait() {
        let (subscriber, _) = create_subscriber();
        let _: &dyn EventPublisher<RunDKG> = &subscriber;
    }

    #[tokio::test]
    async fn test_debuggable_subscriber_trait() {
        let (subscriber, _) = create_subscriber();
        let _: &dyn DebuggableSubscriber = &subscriber;
        let _: &dyn Subscriber = &subscriber;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_notify_with_wrong_event_type() {
        let (subscriber, _) = create_subscriber();

        #[derive(Debug)]
        struct WrongEvent;

        impl Event for WrongEvent {
            fn topic(&self) -> Topic {
                Topic::NewDKGTask
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        impl DebuggableEvent for WrongEvent {}

        let wrong_event = WrongEvent;
        let _result = subscriber.notify(Topic::NewDKGTask, &wrong_event).await;
    }

    #[tokio::test]
    async fn test_multiple_notifications() {
        let (subscriber, _) = create_subscriber();
        for i in 0..3 {
            let new_dkg_task = create_test_new_dkg_task(i + 1, i + 1);
            let result = subscriber.notify(Topic::NewDKGTask, &new_dkg_task).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_error_handling_in_notify() {
        let (subscriber, _) = create_subscriber();
        let new_dkg_task = create_test_new_dkg_task(1, 1);
        let result = subscriber.notify(Topic::NewDKGTask, &new_dkg_task).await;
        match result {
            Ok(_) => println!("Notification succeeded"),
            Err(e) => println!("Notification failed with error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_concurrent_notifications() {
        let (subscriber, _) = create_subscriber();
        let subscriber = Arc::new(subscriber);
        let mut handles = vec![];
        
        for i in 0..5 {
            let subscriber_clone = subscriber.clone();
            let handle = tokio::spawn(async move {
                let new_dkg_task = create_test_new_dkg_task(i + 1, i + 1);
                subscriber_clone.notify(Topic::NewDKGTask, &new_dkg_task).await
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await.unwrap();
            println!("Concurrent notification result: {:?}", result);
        }
    }
    
    #[cfg(test)]
    mod integration_tests {
        use super::*;

        #[tokio::test]
        async fn test_full_workflow() {
            let (subscriber, _) = create_subscriber();
            subscriber.subscribe().await;
            let (test_subscriber, _) = create_subscriber();
            let new_dkg_task = create_test_new_dkg_task(1, 1);
            let result = test_subscriber.notify(Topic::NewDKGTask, &new_dkg_task).await;
            println!("Full workflow result: {:?}", result);
        }

        #[tokio::test]
        async fn test_publish_and_notify_integration() {
            let (subscriber, _) = create_subscriber();
            let new_dkg_task = create_test_new_dkg_task(1, 1);
            let result = subscriber.notify(Topic::NewDKGTask, &new_dkg_task).await;
            assert!(result.is_ok());
        }
    }
}