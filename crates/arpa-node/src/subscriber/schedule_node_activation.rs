use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::{node_activation::NodeActivation, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
};
use arpa_contract_client::{error::ContractClientError, node_registry::NodeRegistryTransactions};
use arpa_core::log::{build_general_payload, build_transaction_receipt_payload, LogType};
use async_trait::async_trait;
use ethers::types::U256;
use log::{debug, error, info};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct NodeActivationSubscriber<PC: Curve> {
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    eq: Arc<RwLock<EventQueue>>,
    c: PhantomData<PC>,
}

impl<PC: Curve> NodeActivationSubscriber<PC> {
    pub fn new(
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        NodeActivationSubscriber {
            chain_identity,
            eq,
            c: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> Subscriber
    for NodeActivationSubscriber<PC>
{
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
        debug!("{:?}", topic);

        let &NodeActivation {
            chain_id,
            is_eigenlayer,
            node_registry_address,
        } = payload.as_any().downcast_ref::<NodeActivation>().unwrap();

        let node_registry_client = self
            .chain_identity
            .read()
            .await
            .build_node_registry_client(node_registry_address);

        match node_registry_client
            .node_activate_by_native_staking(is_eigenlayer)
            .await
        {
            Ok(receipt) => {
                info!(
                    "{}",
                    build_transaction_receipt_payload(
                        LogType::NodeActivated,
                        "Node activated",
                        chain_id,
                        receipt.transaction_hash,
                        receipt.gas_used.unwrap_or(U256::zero()),
                        receipt.effective_gas_price.unwrap_or(U256::zero()),
                    )
                );
            }
            Err(e) => match e {
                ContractClientError::TransactionFailed(receipt) => {
                    error!(
                        "{}",
                        build_transaction_receipt_payload(
                            LogType::NodeActivationFailed,
                            "Node activate failed",
                            chain_id,
                            receipt.transaction_hash,
                            receipt.gas_used.unwrap_or(U256::zero()),
                            receipt.effective_gas_price.unwrap_or(U256::zero()),
                        )
                    );
                }
                _ => {
                    error!(
                        "{}",
                        build_general_payload(
                            LogType::NodeActivationFailed,
                            &format!("Node activate failed with error: {:?}", e),
                            Some(chain_id)
                        )
                    );
                }
            },
        }

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let subscriber = Box::new(self);

        eq.write()
            .await
            .subscribe(Topic::NodeActivation, subscriber);
    }
}

impl<PC: Curve + std::fmt::Debug + Sync + Send + 'static> DebuggableSubscriber
    for NodeActivationSubscriber<PC>
{
}
