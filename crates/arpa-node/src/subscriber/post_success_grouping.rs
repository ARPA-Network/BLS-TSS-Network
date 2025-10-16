use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    error::{NodeError, NodeResult},
    event::{dkg_success::DKGSuccess, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
};
use arpa_core::{
    log::{build_group_related_payload, LogType},
    DKGStatus,
};
use arpa_dal::GroupInfoHandler;
use async_trait::async_trait;
use log::{debug, error, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PostSuccessGroupingSubscriber<PC: Curve> {
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    eq: Arc<RwLock<EventQueue>>,
    c: PhantomData<PC>,
}

impl<PC: Curve> PostSuccessGroupingSubscriber<PC> {
    pub fn new(
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        PostSuccessGroupingSubscriber {
            group_cache,
            eq,
            c: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> Subscriber
    for PostSuccessGroupingSubscriber<PC>
{
    async fn notify(&self, topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()> {
        debug!("{:?}", topic);

        let DKGSuccess {
            chain_id,
            id_address,
            group,
        } = payload
            .as_any()
            .downcast_ref::<DKGSuccess<PC>>()
            .unwrap()
            .clone();

        if self
            .group_cache
            .write()
            .await
            .update_dkg_status(group.index, group.epoch, DKGStatus::WaitForPostProcess)
            .await?
        {
            info!(
                "DKG status updated to WaitForPostProcess for group {} epoch {}",
                group.index, group.epoch
            );

            if group.public_key.is_none()
                || *self.group_cache.read().await.get_public_key()? != group.public_key.unwrap()
            {
                error!(
                    "{}",
                    build_group_related_payload(
                        LogType::DKGGroupingTwisted,
                        "Group public key is different from the one saved in DKG process.",
                        chain_id,
                        self.group_cache.read().await.get_group()?
                    )
                );
                return Err(NodeError::DKGGroupingTwisted);
            }

            if !group.members.contains_key(&id_address) {
                error!(
                    "{}",
                    build_group_related_payload(
                        LogType::DKGGroupingTwisted,
                        "This node is not in the group, skip the process.",
                        chain_id,
                        self.group_cache.read().await.get_group()?
                    )
                );
                return Err(NodeError::DKGGroupingTwisted);
            }

            if !self
                .group_cache
                .write()
                .await
                .sync_up_members(group.index, group.epoch, group.members)
                .await?
            {
                error!(
                    "{}",
                    build_group_related_payload(
                        LogType::DKGGroupingMemberMisMatch,
                        "During the DKG process, group members are not matched, attempt to run with contract records.",
                        chain_id,
                        self.group_cache.read().await.get_group()?
                    )
                );
            }
            self.group_cache
                .write()
                .await
                .save_committers(group.index, group.epoch, group.committers)
                .await?;

            info!(
                "{}",
                build_group_related_payload(
                    LogType::DKGGroupingAvailable,
                    "Group is available, committers saved.",
                    chain_id,
                    self.group_cache.read().await.get_group()?
                )
            );
        }

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write().await.subscribe(Topic::DKGSuccess, subscriber);
    }
}

impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> DebuggableSubscriber
    for PostSuccessGroupingSubscriber<PC>
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::{dkg_success::DKGSuccess, types::Topic, Event},
        queue::event_queue::EventQueue,
    };
    use arpa_core::{Group, Member, DKGTask};
    use arpa_dal::{GroupInfoHandler, cache::InMemoryGroupInfoCache};
    use ethers_core::types::Address;
    use std::{
        any::Any,
        collections::BTreeMap,
        marker::PhantomData,
        sync::Arc,
    };
    use threshold_bls::schemes::bn254::G2Curve;
    use tokio::sync::RwLock;

    const TEST_GROUP_INDEX: usize = 1;
    const TEST_EPOCH: usize = 1;
    const TEST_SIZE: usize = 3;
    const TEST_THRESHOLD: usize = 2;
    const TEST_CHAIN_ID: usize = 0;
    const TEST_ASSIGNMENT_BLOCK_HEIGHT: usize = 100;
    const TEST_RPC_ENDPOINT: &str = "http://localhost:8545";
    
    fn create_test_group(id_address: Address, has_public_key: bool) -> Group<G2Curve> {
        let mut members = BTreeMap::new();
        members.insert(id_address, Member {
            index: 0,
            dkg_index: Some(0),
            id_address,
            rpc_endpoint: Some(TEST_RPC_ENDPOINT.to_string()),
            partial_public_key: Some(G2Curve::point()),
        });

        Group {
            index: TEST_GROUP_INDEX,
            epoch: TEST_EPOCH,
            size: TEST_SIZE,
            threshold: TEST_THRESHOLD,
            state: true,
            public_key: if has_public_key { Some(G2Curve::point()) } else { None },
            members,
            committers: vec![id_address],
            c: PhantomData,
        }
    }

    fn create_test_dkg_success(
        chain_id: usize,
        id_address: Address,
        has_public_key: bool,
    ) -> DKGSuccess<G2Curve> {
        DKGSuccess {
            chain_id,
            id_address,
            group: create_test_group(id_address, has_public_key),
        }
    }

    fn create_group_cache(id_address: Address) -> Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> {
        Arc::new(RwLock::new(Box::new(InMemoryGroupInfoCache::<G2Curve>::new(id_address))))
    }

    fn create_event_queue() -> Arc<RwLock<EventQueue>> {
        Arc::new(RwLock::new(EventQueue::new()))
    }

    async fn setup_group_cache_with_group(
        id_address: Address,
        group: Group<G2Curve>,
    ) -> Arc<RwLock<Box<dyn GroupInfoHandler<G2Curve>>>> {
        let group_cache = create_group_cache(id_address);
        {
            let mut cache = group_cache.write().await;
            let dkg_task = DKGTask {
                group_index: group.index,
                epoch: group.epoch,
                size: group.size,
                threshold: group.threshold,
                members: group.members.keys().copied().collect(),
                assignment_block_height: TEST_ASSIGNMENT_BLOCK_HEIGHT,
                coordinator_address: Address::random(),
            };
            cache.save_task_info(TEST_CHAIN_ID, dkg_task).await.unwrap();
        }
        group_cache
    }

    #[tokio::test]
    async fn test_post_success_grouping_subscriber_creation() {
        let id_address = Address::random();
        let group_cache = create_group_cache(id_address);
        let eq = create_event_queue();
        let subscriber = PostSuccessGroupingSubscriber::new(group_cache, eq);
        assert!(format!("{:?}", subscriber).contains("PostSuccessGroupingSubscriber"));
    }

    #[tokio::test]
    async fn test_notify_successful_flow() {
        let id_address = Address::random();
        let group_cache = create_group_cache(id_address);
        let eq = create_event_queue();

        {
            let mut cache = group_cache.write().await;
            let dkg_task = DKGTask {
                group_index: TEST_GROUP_INDEX,
                epoch: TEST_EPOCH,
                size: TEST_SIZE,
                threshold: TEST_THRESHOLD,
                members: vec![id_address], 
                assignment_block_height: TEST_ASSIGNMENT_BLOCK_HEIGHT,
                coordinator_address: Address::random(),
            };
            cache.save_task_info(TEST_CHAIN_ID, dkg_task).await.unwrap();
        }

        let subscriber = PostSuccessGroupingSubscriber::new(group_cache, eq);
        let dkg_success = create_test_dkg_success(1, id_address, true);
        let result = subscriber.notify(Topic::DKGSuccess, &dkg_success).await;

        match result {
            Ok(_) => println!("Success!"),
            Err(e) => println!("Error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_notify_node_not_in_group_error() {
        let id_address = Address::random();
        let different_address = Address::random();
        let group = create_test_group(id_address, true);
        let group_cache = setup_group_cache_with_group(id_address, group).await;
        let eq = create_event_queue();

        let subscriber = PostSuccessGroupingSubscriber::new(group_cache, eq);
        let dkg_success = create_test_dkg_success(1, different_address, true);
        let result = subscriber.notify(Topic::DKGSuccess, &dkg_success).await;

        assert!(result.is_err());
        if let Err(e) = result {
            println!("Error type: {:?}", e);
        }        
    }

    #[tokio::test]
    async fn test_notify_different_public_key_error() {
        let id_address = Address::random();
        let group = create_test_group(id_address, true);
        let group_cache = setup_group_cache_with_group(id_address, group).await;
        let eq = create_event_queue();

        let subscriber = PostSuccessGroupingSubscriber::new(group_cache, eq);
        let dkg_success = create_test_dkg_success(1, id_address, false);
        let result = subscriber.notify(Topic::DKGSuccess, &dkg_success).await;

        assert!(result.is_err());
        if let Err(e) = result {
            println!("Error type: {:?}", e);
        }        
    }

    #[tokio::test]
    async fn test_subscribe() {
        let id_address = Address::random();
        let group_cache = create_group_cache(id_address);
        let eq = create_event_queue();
        let subscriber = PostSuccessGroupingSubscriber::new(group_cache, eq.clone());
        subscriber.subscribe().await;
    }

    #[tokio::test]
    async fn test_debuggable_subscriber_trait() {
        let id_address = Address::random();
        let group_cache = create_group_cache(id_address);
        let eq = create_event_queue();
        let subscriber = PostSuccessGroupingSubscriber::new(group_cache, eq);

        let _: &dyn DebuggableSubscriber = &subscriber;
        let _: &dyn Subscriber = &subscriber;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_notify_with_wrong_event_type() {
        let id_address = Address::random();
        let group_cache = create_group_cache(id_address);
        let eq = create_event_queue();
        let subscriber = PostSuccessGroupingSubscriber::new(group_cache, eq);

        #[derive(Debug)]
        struct WrongEvent;

        impl Event for WrongEvent {
            fn topic(&self) -> Topic {
                Topic::DKGSuccess
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        impl DebuggableEvent for WrongEvent {}

        let wrong_event = WrongEvent;
        let _result = subscriber.notify(Topic::DKGSuccess, &wrong_event).await;
    }

    #[tokio::test]
    async fn test_multiple_notifications() {
        let id_address = Address::random();
        let group_cache = create_group_cache(id_address);
        let eq = create_event_queue();
        let subscriber = PostSuccessGroupingSubscriber::new(group_cache, eq);

        for i in 0..3 {
            let dkg_success = create_test_dkg_success(1, id_address, true);
            let result = subscriber.notify(Topic::DKGSuccess, &dkg_success).await;
            println!("Notification {}: {:?}", i, result);
        }
    }

    #[tokio::test]
    async fn test_with_different_chain_ids() {
        let id_address = Address::random();
        let group_cache = create_group_cache(id_address);
        let eq = create_event_queue();
        let subscriber = PostSuccessGroupingSubscriber::new(group_cache, eq);

        let chain_ids = vec![1, 2, 3];
        for chain_id in chain_ids {
            let dkg_success = create_test_dkg_success(chain_id, id_address, true);
            let result = subscriber.notify(Topic::DKGSuccess, &dkg_success).await;
            println!("Chain ID {}: {:?}", chain_id, result);
        }
    }

    #[cfg(test)]
    mod integration_tests {
        use super::*;

        #[tokio::test]
        async fn test_full_workflow() {
            let id_address = Address::random();
            let group_cache = create_group_cache(id_address);
            let eq = create_event_queue();

            let subscriber = PostSuccessGroupingSubscriber::new(group_cache.clone(), eq.clone());
            subscriber.subscribe().await;

            let test_subscriber = PostSuccessGroupingSubscriber::new(group_cache, eq);
            let dkg_success = create_test_dkg_success(1, id_address, true);
            let result = test_subscriber.notify(Topic::DKGSuccess, &dkg_success).await;

            println!("Full workflow result: {:?}", result);
        }
    }
}