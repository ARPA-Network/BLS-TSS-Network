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

        let f_ts_lock_result = tokio::time::timeout(Duration::from_secs(5), self.f_ts.read()).await;

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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{context::ChainIdentityHandlerType, scheduler::TaskScheduler};
    use arpa_core::{
        Config, FixedIntervalRetryDescriptor, GeneralMainChainIdentity, ListenerType, SubscriberType,
    };
    use ethers::{
        providers::{Provider, Ws},
        signers::{LocalWallet, Signer},
        types::Address,
        utils::{Anvil, AnvilInstance},
    };
    use std::{sync::Arc, time::Duration};
    use threshold_bls::schemes::bn254::G2Curve;
    use tokio::sync::RwLock;

    #[derive(Debug)]
    struct MockListener {
        descriptor: ListenerDescriptor,
        message: String,
    }

    impl std::fmt::Display for MockListener {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "MockListener({})", self.message)
        }
    }

    #[async_trait]
    impl Listener for MockListener {
        async fn listen(&self) -> NodeResult<()> {
            println!("{} would run here", self.message);
            Ok(())
        }

        async fn handle_interruption(&self) -> NodeResult<()> {
            println!("{} handle_interruption", self.message);
            Ok(())
        }

        fn chain_id(&self) -> usize {
            self.descriptor.chain_id
        }

        fn listener_descriptor(&self) -> ListenerDescriptor {
            self.descriptor.clone()
        }
    }

    fn create_mock_listener(chain_id: usize, l_type: ListenerType, message: &str) -> MockListener {
        MockListener {
            descriptor: ListenerDescriptor {
                chain_id,
                l_type,
                interval_millis: 1000,
                use_jitter: true,
                reset_descriptor: FixedIntervalRetryDescriptor {
                    interval_millis: 5000,
                    max_attempts: 3,
                    use_jitter: true,
                },
            },
            message: message.to_string(),
        }
    }

    async fn setup_test_environment() -> NodeResult<(
        AnvilInstance,
        usize,
        Arc<RwLock<ChainIdentityHandlerType<G2Curve>>>,
        Arc<RwLock<SimpleFixedTaskScheduler>>,
    )> {
        println!("Starting test setup");
        
        let anvil = Anvil::new().spawn();
        println!("Anvil instance started at {}", anvil.endpoint());
        
        let ws_provider = Arc::new(Provider::<Ws>::connect(anvil.ws_endpoint()).await?);
        println!("Connected to Anvil WebSocket at {}", anvil.ws_endpoint());
        
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let wallet_with_chain_id = wallet.clone().with_chain_id(anvil.chain_id());
        let chain_id = anvil.chain_id() as usize;
        println!("Using wallet address: {}, Chain ID: {}", wallet.clone().address(), chain_id);
        
        let adapter_address = Address::random();
        let controller_address = Address::random();
        let config = Config::default();
        
        let chain_identity = GeneralMainChainIdentity::new(
            chain_id,
            wallet_with_chain_id.clone(),
            ws_provider.clone(),
            anvil.ws_endpoint(),
            controller_address,
            adapter_address,
            Address::random(),
            config.get_time_limits().contract_transaction_retry_descriptor,
            config.get_time_limits().contract_view_retry_descriptor,
            None,
        );
        
        let chain_identity_arc: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> = 
            Arc::new(RwLock::new(Box::new(chain_identity) as ChainIdentityHandlerType<G2Curve>));
        
        println!("Chain identity created");
        
        let fixed_task_scheduler = SimpleFixedTaskScheduler::new();
        let f_ts = Arc::new(RwLock::new(fixed_task_scheduler));
        println!("Fixed task scheduler created");

        Ok((anvil, chain_id, chain_identity_arc, f_ts))
    }

    async fn add_test_tasks(
        f_ts: &Arc<RwLock<SimpleFixedTaskScheduler>>,
        chain_id: usize,
    ) -> NodeResult<()> {
        let mut scheduler = f_ts.write().await;
        
        scheduler.add_listener_task(
            create_mock_listener(chain_id, ListenerType::Block, "Block listener")
        )?;
        
        scheduler.add_listener_task(
            create_mock_listener(chain_id, ListenerType::NewRandomnessTask, "NewRandomnessTask listener")
        )?;
        
        scheduler.add_listener_task(
            create_mock_listener(chain_id + 1, ListenerType::Block, "Other chain Block listener")
        )?;
        
        scheduler.add_task(
            ComponentTaskType::Subscriber(chain_id, SubscriberType::Block),
            async {
                println!("Subscriber task would run here");
            },
        )?;

        Ok(())
    }

    fn assert_listener_descriptor_equals(actual: &ListenerDescriptor, expected: &ListenerDescriptor) {
        assert_eq!(actual.chain_id, expected.chain_id);
        assert_eq!(actual.l_type, expected.l_type);
        assert_eq!(actual.interval_millis, expected.interval_millis);
        assert_eq!(actual.use_jitter, expected.use_jitter);
    }

    #[tokio::test]
    async fn test_provider_reconnection_listener() -> NodeResult<()> {
        let (_anvil, chain_id, chain_identity_arc, f_ts) = setup_test_environment().await?;
        
        add_test_tasks(&f_ts, chain_id).await?;
        println!("Test tasks added to scheduler");
        
        let listener_descriptor = ListenerDescriptor {
            chain_id,
            l_type: ListenerType::ScheduleProviderReconnection,
            interval_millis: 5000,
            use_jitter: true,
            reset_descriptor: FixedIntervalRetryDescriptor {
                interval_millis: 5000,
                max_attempts: 3,
                use_jitter: true,
            },
        };
        
        let mut listener = ProviderReconnectionListener::<G2Curve>::new(
            listener_descriptor.clone(),
            chain_identity_arc.clone(),
            f_ts.clone(),
        );
        
        println!("ProviderReconnectionListener created");
        
        listener.initialize().await?;
        println!("Listener initialized");
        
        let tasks_before: Vec<ComponentTaskType> = {
            let scheduler = f_ts.read().await;
            scheduler.get_tasks().into_iter().cloned().collect()
        };
        
        println!("Tasks before listen: {:?}", tasks_before);
        
        {
            println!("Simulating provider disconnection by resetting provider");
            let mut chain_identity = chain_identity_arc.write().await;
            let _ = chain_identity.reset_provider().await;
        }
        
        println!("Executing listener.listen()");
        let listen_result = listener.listen().await;
        assert!(listen_result.is_ok(), "listener.listen() failed: {:?}", listen_result);
        
        {
            let scheduler = f_ts.read().await;
            let tasks_after = scheduler.get_tasks();
            println!("Tasks after listen: {:?}", tasks_after);
            
            for task in &tasks_before {
                if let ComponentTaskType::Listener(task_chain_id, _) = task {
                    if *task_chain_id == chain_id {
                        assert!(
                            scheduler.has_task(&task),
                            "Task {:?} should still exist after reconnection",
                            task
                        );
                    }
                }
            }
        }
        
        println!("Testing handle_interruption");
        let interruption_result = listener.handle_interruption().await;
        assert!(
            interruption_result.is_ok(),
            "handle_interruption failed: {:?}",
            interruption_result
        );
        
        assert_eq!(listener.chain_id(), chain_id);
        let returned_descriptor = listener.listener_descriptor();
        assert_listener_descriptor_equals(&returned_descriptor, &listener_descriptor);
        
        if let Some(jitter_fn) = listener.jitter_fn() {
            let original_duration = Duration::from_millis(1000);
            let jittered_duration = jitter_fn(original_duration);
            assert!(
                jittered_duration >= Duration::from_millis(800) && jittered_duration <= Duration::from_millis(1200),
                "Jittered duration should be within 20% of original"
            );
        } else {
            panic!("jitter_fn should return Some");
        }
        
        let display_string = format!("{}", listener);
        assert_eq!(display_string, "ProviderReconnectionListener");
        
        println!("Test completed successfully");
        Ok(())
    }
    
    impl SimpleFixedTaskScheduler {
        pub fn has_task(&self, task: &ComponentTaskType) -> bool {
            self.get_tasks().contains(&task)
        }
        
        pub fn get_task_state(&self, _task: &ComponentTaskType) -> &'static str {
            "unknown"
        }
    }
}