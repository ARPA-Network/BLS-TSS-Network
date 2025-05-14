pub mod block;
pub mod new_randomness_task;
pub mod post_commit_grouping;
pub mod post_grouping;
pub mod pre_grouping;
pub mod randomness_signature_aggregation;
pub mod ready_to_handle_randomness_task;
pub mod schedule_node_activation;
pub mod schedule_provider_reconnection;
use crate::error::NodeResult;
use arpa_core::jitter;
use arpa_core::log::{build_general_payload, LogType};
use arpa_core::ListenerDescriptor;
use async_trait::async_trait;
use log::{debug, error};
use std::fmt::Debug;
use std::fmt::Display;
use std::time::Duration;
use tokio::time::sleep;
use tokio_retry::{strategy::FixedInterval, Retry};

#[async_trait]
pub trait Listener: Debug + Display {
    async fn start(&self) -> NodeResult<()> {
        let interval_millis = self.listener_descriptor().interval_millis;
        let use_jitter = self.listener_descriptor().use_jitter;
        let reset_descriptor = self.listener_descriptor().reset_descriptor;
        let jitter_fn = if let Some(jitter_fn) = self.jitter_fn() {
            jitter_fn
        } else {
            Box::new(jitter)
        };

        let mut next_polling_strategy = FixedInterval::from_millis(interval_millis).map(|e| {
            if use_jitter {
                jitter_fn(e)
            } else {
                e
            }
        });

        loop {
            if let Err(err) = self.listen().await {
                error!(
                    "{}",
                    build_general_payload(
                        LogType::ListenerInterrupted,
                        &format!("{} is interrupted. Retry... Error: {:?}.", self, err),
                        Some(self.chain_id())
                    )
                );

                let reset_strategy = FixedInterval::from_millis(reset_descriptor.interval_millis)
                    .map(|e| {
                        if reset_descriptor.use_jitter {
                            jitter_fn(e)
                        } else {
                            e
                        }
                    })
                    .take(reset_descriptor.max_attempts);

                Retry::spawn(reset_strategy, || async {
                    self.handle_interruption().await
                })
                .await?;
            }
            debug!(
                "{} chain {} is sleeping for {:?}.",
                self,
                self.chain_id(),
                next_polling_strategy.next().unwrap()
            );
            sleep(next_polling_strategy.next().unwrap()).await;
        }
    }

    async fn initialize(&mut self) -> NodeResult<()> {
        Ok(())
    }

    async fn listen(&self) -> NodeResult<()>;

    async fn handle_interruption(&self) -> NodeResult<()> {
        Ok(())
    }

    fn chain_id(&self) -> usize;

    fn listener_descriptor(&self) -> ListenerDescriptor;

    fn jitter_fn(&self) -> Option<Box<dyn Fn(Duration) -> Duration + Send + Sync>> {
        None
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use tokio::time::sleep;
    use log::LevelFilter;

    struct TestLogCollector {
        logs: Arc<Mutex<Vec<String>>>,
    }

    impl log::Log for TestLogCollector {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &log::Record) {
            let logs = self.logs.clone();
            let message = format!("{}", record.args());
            tokio::spawn(async move {
                let mut logs = logs.lock().await;
                logs.push(message);
            });
        }

        fn flush(&self) {}
    }

    #[derive(Debug)]
    struct TestListener {
        listen_counter: Arc<AtomicUsize>,
        handle_interruption_counter: Arc<AtomicUsize>,
        should_fail: Arc<AtomicBool>,
        interruption_should_fail: Arc<AtomicBool>,
        chain_id: usize,
        initialized: Arc<AtomicBool>,
    }

    impl Display for TestListener {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestListener-{}", self.chain_id)
        }
    }

    #[async_trait]
    impl Listener for TestListener {
        async fn initialize(&mut self) -> NodeResult<()> {
            if !self.initialized.load(Ordering::SeqCst) {
                self.initialized.store(true, Ordering::SeqCst);
                Ok(())
            } else {
                return Err(anyhow::anyhow!("Already initialized").into());
            }
        }

        async fn listen(&self) -> NodeResult<()> {
            self.listen_counter.fetch_add(1, Ordering::SeqCst);
            
            if self.should_fail.load(Ordering::SeqCst) {
                return Err(anyhow::anyhow!("Simulated listen failure").into());
            }
            
            Ok(())
        }

        async fn handle_interruption(&self) -> NodeResult<()> {
            self.handle_interruption_counter.fetch_add(1, Ordering::SeqCst);
            
            if self.interruption_should_fail.load(Ordering::SeqCst) {
                return Err(anyhow::anyhow!("Simulated interruption handler failure").into());
            }
            
            Ok(())
        }

        fn listener_descriptor(&self) -> arpa_core::ListenerDescriptor {
            arpa_core::ListenerDescriptor {
                chain_id: self.chain_id,
                l_type: arpa_core::ListenerType::Block,
                interval_millis: 150, 
                use_jitter: true,     
                reset_descriptor: arpa_core::FixedIntervalRetryDescriptor {
                    interval_millis: 150, 
                    max_attempts: 3,       
                    use_jitter: true,      
                },
            }
        }
    
        fn chain_id(&self) -> usize {
            self.chain_id
        }
    }

    fn setup_test_listener() -> (TestListener, Arc<AtomicUsize>, Arc<AtomicUsize>, Arc<AtomicBool>, Arc<AtomicBool>) {
        let listen_counter = Arc::new(AtomicUsize::new(0));
        let handle_interruption_counter = Arc::new(AtomicUsize::new(0));
        let should_fail = Arc::new(AtomicBool::new(false));
        let interruption_should_fail = Arc::new(AtomicBool::new(false));
        
        let listener = TestListener {
            listen_counter: listen_counter.clone(),
            handle_interruption_counter: handle_interruption_counter.clone(),
            should_fail: should_fail.clone(),
            interruption_should_fail: interruption_should_fail.clone(),
            chain_id: 1,
            initialized: Arc::new(AtomicBool::new(false)),
        };
        
        (listener, listen_counter, handle_interruption_counter, should_fail, interruption_should_fail)
    }

    #[tokio::test]
    async fn test_normal_operation() {
        let (listener, listen_counter, _, _, _) = setup_test_listener();
        
        let listener_handle = tokio::spawn(async move {
            listener.start().await
        });
        
        sleep(Duration::from_millis(550)).await;
        
        listener_handle.abort();
        let _ = listener_handle.await;
        
        let count = listen_counter.load(Ordering::SeqCst);
        assert!(count >= 4 && count <= 6, "Expected 4-6 calls, got {}", count);
    }

    #[tokio::test]
    async fn test_retry_mechanism() {
        let (listener, _, handle_interruption_counter, should_fail, _) = setup_test_listener();
        
        should_fail.store(true, Ordering::SeqCst);
        
        let listener_handle = tokio::spawn(async move {
            listener.start().await
        });
        
        sleep(Duration::from_millis(400)).await;
        
        listener_handle.abort();
        let _ = listener_handle.await;
        
        let interruption_count = handle_interruption_counter.load(Ordering::SeqCst);
        assert!(interruption_count > 0, "handle_interruption should be called at least once");
    }

    #[tokio::test]
    async fn test_jitter_functionality() {
        let mut intervals_with_jitter = Vec::new();
        let mut strategy = FixedInterval::from_millis(100).map(jitter);
        for _ in 0..10 {
            intervals_with_jitter.push(strategy.next().unwrap());
        }
        
        let mut all_same = true;
        for i in 1..intervals_with_jitter.len() {
            if intervals_with_jitter[i] != intervals_with_jitter[0] {
                all_same = false;
                break;
            }
        }
        assert!(!all_same, "Intervals with jitter should not all be the same");
        
        let mut intervals_without_jitter = Vec::new();
        let mut strategy = FixedInterval::from_millis(100);
        for _ in 0..10 {
            intervals_without_jitter.push(strategy.next().unwrap());
        }
        
        for interval in &intervals_without_jitter {
            assert_eq!(*interval, Duration::from_millis(100));
        }
    }

    #[tokio::test]
    async fn test_initialization() {
        let (mut listener, _, _, _, _) = setup_test_listener();
        
        let result = listener.initialize().await;
        assert!(result.is_ok(), "First initialization should succeed");
        
        let result = listener.initialize().await;
        assert!(result.is_err(), "Second initialization should fail");
    }

    #[tokio::test]
    async fn test_interruption_handling() {
        let (listener, _, _, should_fail, interruption_should_fail) = setup_test_listener();
        
        should_fail.store(true, Ordering::SeqCst);
        interruption_should_fail.store(false, Ordering::SeqCst);
        
        let result = listener.handle_interruption().await;
        assert!(result.is_ok(), "Interruption handling should succeed");
        
        interruption_should_fail.store(true, Ordering::SeqCst);
        
        let result = listener.handle_interruption().await;
        assert!(result.is_err(), "Interruption handling should fail when configured to do so");
    }

    #[tokio::test]
    async fn test_concurrent_listeners() {
        let (listener1, listen_counter1, _, _, _) = setup_test_listener();
        let mut listener2 = TestListener {
            listen_counter: Arc::new(AtomicUsize::new(0)),
            handle_interruption_counter: Arc::new(AtomicUsize::new(0)),
            should_fail: Arc::new(AtomicBool::new(false)),
            interruption_should_fail: Arc::new(AtomicBool::new(false)),
            chain_id: 2,
            initialized: Arc::new(AtomicBool::new(false)),
        };
        let listen_counter2 = listener2.listen_counter.clone();
        
        listener2.initialize().await.unwrap();
        
        let handle1 = tokio::spawn(async move {
            listener1.start().await
        });
        
        let handle2 = tokio::spawn(async move {
            listener2.start().await
        });
        
        sleep(Duration::from_millis(500)).await;
        
        handle1.abort();
        handle2.abort();
        let _ = handle1.await;
        let _ = handle2.await;
        
        assert!(listen_counter1.load(Ordering::SeqCst) > 0);
        assert!(listen_counter2.load(Ordering::SeqCst) > 0);
    }

    #[tokio::test]
    async fn test_logging() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let collector = TestLogCollector { logs: logs.clone() };
        log::set_boxed_logger(Box::new(collector)).unwrap();
        log::set_max_level(LevelFilter::Error);
        
        let (listener, _, _, should_fail, _) = setup_test_listener();
        
        should_fail.store(true, Ordering::SeqCst);
        
        let listener_handle = tokio::spawn(async move {
            listener.start().await
        });
        
        sleep(Duration::from_millis(300)).await;
        
        listener_handle.abort();
        let _ = listener_handle.await;
        
        let logged_messages = logs.lock().await;
        let has_interruption_log = logged_messages.iter().any(|log| 
            log.contains("ListenerInterrupted") && log.contains("TestListener-1")
        );
        
        assert!(has_interruption_log, "Should log an interruption message");
    }

    #[tokio::test]
    async fn test_jitter() {
        let mut s = FixedInterval::from_millis(1000).map(jitter);
        for _ in 0..10 {
            println!("{:?}", s.next().unwrap());
        }
    }
}
