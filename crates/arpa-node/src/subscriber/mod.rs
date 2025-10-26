pub mod block;
pub mod in_grouping;
pub mod post_grouping;
pub mod post_success_grouping;
pub mod pre_grouping;
pub mod randomness_signature_aggregation;
pub mod ready_to_handle_randomness_task;
pub mod schedule_node_activation;

use crate::{
    error::NodeResult,
    event::{types::Topic, Event},
};
use async_trait::async_trait;

pub trait DebuggableEvent: Event + std::fmt::Debug + Send + Sync {}

pub trait DebuggableSubscriber: Subscriber + std::fmt::Debug + Send + Sync {}

#[async_trait]
pub trait Subscriber {
    async fn notify(&self, topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()>;

    async fn subscribe(self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        error::NodeResult,
        event::{types::Topic, Event},
    };
    use async_trait::async_trait;
    use std::any::Any;

    #[derive(Debug, Clone)]
    struct MockEvent {
        topic: Topic,
        data: String,
    }

    impl MockEvent {
        fn new(topic: Topic, data: String) -> Self {
            Self { topic, data }
        }
    }

    impl Event for MockEvent {
        fn topic(&self) -> Topic {
            self.topic.clone()
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl DebuggableEvent for MockEvent {}

    #[derive(Debug)]
    struct MockSubscriber {
        notify_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        subscribe_called: std::sync::Arc<std::sync::atomic::AtomicBool>,
        last_topic: std::sync::Arc<std::sync::Mutex<Option<Topic>>>,
    }

    impl MockSubscriber {
        fn new() -> Self {
            Self {
                notify_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                subscribe_called: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                last_topic: std::sync::Arc::new(std::sync::Mutex::new(None)),
            }
        }

        fn get_notify_count(&self) -> usize {
            self.notify_count.load(std::sync::atomic::Ordering::SeqCst)
        }

        fn is_subscribe_called(&self) -> bool {
            self.subscribe_called.load(std::sync::atomic::Ordering::SeqCst)
        }

        fn get_last_topic(&self) -> Option<Topic> {
            self.last_topic.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl Subscriber for MockSubscriber {
        async fn notify(&self, topic: Topic, _payload: &dyn DebuggableEvent) -> NodeResult<()> {
            self.notify_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            *self.last_topic.lock().unwrap() = Some(topic);
            Ok(())
        }

        async fn subscribe(self) {
            self.subscribe_called.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    impl DebuggableSubscriber for MockSubscriber {}

    #[derive(Debug)]
    struct ErrorSubscriber {
        should_error: bool,
    }

    impl ErrorSubscriber {
        fn new(should_error: bool) -> Self {
            Self { should_error }
        }
    }

    #[async_trait]
    impl Subscriber for ErrorSubscriber {
        async fn notify(&self, _topic: Topic, _payload: &dyn DebuggableEvent) -> NodeResult<()> {
            if self.should_error {
                Err(crate::error::NodeError::InvalidTaskType.into())
            } else {
                Ok(())
            }
        }

        async fn subscribe(self) {}
    }

    impl DebuggableSubscriber for ErrorSubscriber {}

    #[tokio::test]
    async fn test_subscriber_notify_success() {
        let subscriber = MockSubscriber::new();
        let event = MockEvent::new(Topic::NewBlock(1), "test_data".to_string());
        let result = subscriber.notify(Topic::NewBlock(1), &event).await;
        assert!(result.is_ok());
        assert_eq!(subscriber.get_notify_count(), 1);
        assert_eq!(subscriber.get_last_topic(), Some(Topic::NewBlock(1)));
    }

    #[tokio::test]
    async fn test_subscriber_notify_multiple_times() {
        let subscriber = MockSubscriber::new();
        let event1 = MockEvent::new(Topic::NewBlock(1), "test_data1".to_string());
        let event2 = MockEvent::new(Topic::NewBlock(2), "test_data2".to_string());
        let result1 = subscriber.notify(Topic::NewBlock(1), &event1).await;
        let result2 = subscriber.notify(Topic::NewBlock(2), &event2).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(subscriber.get_notify_count(), 2);
        assert_eq!(subscriber.get_last_topic(), Some(Topic::NewBlock(2)));
    }

    #[tokio::test]
    async fn test_subscriber_notify_error() {
        let subscriber = ErrorSubscriber::new(true);
        let event = MockEvent::new(Topic::NewBlock(1), "test_data".to_string());
        let result = subscriber.notify(Topic::NewBlock(1), &event).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_subscriber_subscribe() {
        let subscriber = MockSubscriber::new();
        assert!(!subscriber.is_subscribe_called());
        subscriber.subscribe().await;
    }

    #[tokio::test]
    async fn test_debuggable_event_trait() {
        let event = MockEvent::new(Topic::NewBlock(1), "test_data".to_string());
        let _: &dyn Event = &event;
        let _: &dyn std::fmt::Debug = &event;
        let _: &dyn DebuggableEvent = &event;
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("MockEvent"));
    }

    #[tokio::test]
    async fn test_debuggable_subscriber_trait() {
        let subscriber = MockSubscriber::new();
        let _: &dyn Subscriber = &subscriber;
        let _: &dyn std::fmt::Debug = &subscriber;
        let _: &dyn DebuggableSubscriber = &subscriber;
        let debug_str = format!("{:?}", subscriber);
        assert!(debug_str.contains("MockSubscriber"));
    }

    #[tokio::test]
    async fn test_event_topic_consistency() {
        let topic = Topic::NewBlock(1);
        let event = MockEvent::new(topic.clone(), "test_data".to_string());
        assert_eq!(event.topic(), topic);
    }

    #[tokio::test]
    async fn test_event_as_any() {
        let event = MockEvent::new(Topic::NewBlock(1), "test_data".to_string());
        let any_ref = event.as_any();
        let downcast_result = any_ref.downcast_ref::<MockEvent>();
        assert!(downcast_result.is_some());
        let downcast_event = downcast_result.unwrap();
        assert_eq!(downcast_event.data, "test_data");
    }

    #[tokio::test]
    async fn test_different_topics() {
        let subscriber = MockSubscriber::new();
        let topics = vec![Topic::NewBlock(1), Topic::NewBlock(2)];
        for (i, topic) in topics.iter().enumerate() {
            let event = MockEvent::new(topic.clone(), format!("data_{}", i));
            let result = subscriber.notify(topic.clone(), &event).await;
            assert!(result.is_ok());
        }
        assert_eq!(subscriber.get_notify_count(), topics.len());
    }

    #[tokio::test]
    async fn test_trait_object_usage() {
        let subscriber: Box<dyn DebuggableSubscriber> = Box::new(MockSubscriber::new());
        let event: Box<dyn DebuggableEvent> = Box::new(MockEvent::new(Topic::NewBlock(1), "boxed_data".to_string()));
        let result = subscriber.notify(Topic::NewBlock(1), event.as_ref()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_sync_requirements() {
        let subscriber = MockSubscriber::new();
        let event = MockEvent::new(Topic::NewBlock(1), "test_data".to_string());
        let handle = tokio::spawn(async move {
            subscriber.notify(Topic::NewBlock(1), &event).await
        });
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_many_notifications() {
        let subscriber = MockSubscriber::new();
        let count = 1000;
        for i in 0..count {
            let event = MockEvent::new(Topic::NewBlock(1), format!("data_{}", i));
            let result = subscriber.notify(Topic::NewBlock(1), &event).await;
            assert!(result.is_ok());
        }
        assert_eq!(subscriber.get_notify_count(), count);
    }

    #[cfg(test)]
    mod integration_tests {
        use super::*;

        #[tokio::test]
        async fn test_multiple_subscribers_with_same_event() {
            let subscriber1 = tests::MockSubscriber::new();
            let subscriber2 = tests::MockSubscriber::new();
            let event = tests::MockEvent::new(Topic::NewBlock(1), "shared_data".to_string());
            let result1 = subscriber1.notify(Topic::NewBlock(1), &event).await;
            let result2 = subscriber2.notify(Topic::NewBlock(1), &event).await;
            assert!(result1.is_ok());
            assert!(result2.is_ok());
            assert_eq!(subscriber1.get_notify_count(), 1);
            assert_eq!(subscriber2.get_notify_count(), 1);
        }

        #[tokio::test]
        async fn test_subscriber_lifecycle() {
            let subscriber = tests::MockSubscriber::new();
            let event = tests::MockEvent::new(Topic::NewBlock(1), "test_data".to_string());
            let notify_result = subscriber.notify(Topic::NewBlock(1), &event).await;
            assert!(notify_result.is_ok());
            subscriber.subscribe().await;
        }
    }
}