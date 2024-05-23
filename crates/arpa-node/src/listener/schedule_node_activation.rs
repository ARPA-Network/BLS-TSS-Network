use super::Listener;
use crate::{
    context::ChainIdentityHandlerType,
    error::NodeResult,
    event::node_activation::NodeActivation,
    queue::{event_queue::EventQueue, EventPublisher},
};
use arpa_contract_client::{controller::ControllerViews, node_registry::NodeRegistryViews};
use async_trait::async_trait;
use ethers::{providers::Middleware, types::Address};
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use tokio::sync::RwLock;

pub struct NodeActivationListener<PC: Curve> {
    chain_id: usize,
    is_eigenlayer: bool,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    node_registry_address: Option<Address>,
    eq: Arc<RwLock<EventQueue>>,
    pc: PhantomData<PC>,
}

impl<PC: Curve> std::fmt::Display for NodeActivationListener<PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeActivationListener")
    }
}

impl<PC: Curve> NodeActivationListener<PC> {
    pub fn new(
        chain_id: usize,
        is_eigenlayer: bool,
        chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        NodeActivationListener {
            chain_id,
            is_eigenlayer,
            chain_identity,
            node_registry_address: None,
            eq,
            pc: PhantomData,
        }
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> EventPublisher<NodeActivation> for NodeActivationListener<PC> {
    async fn publish(&self, event: NodeActivation) {
        self.eq.read().await.publish(event).await;
    }
}

#[async_trait]
impl<PC: Curve + Sync + Send> Listener for NodeActivationListener<PC> {
    async fn initialize(&mut self) -> NodeResult<()> {
        if self.node_registry_address.is_none() {
            let controller_client = self.chain_identity.read().await.build_controller_client();

            let node_registry_address =
                ControllerViews::<PC>::get_node_registry_address(&controller_client).await?;

            self.node_registry_address = Some(node_registry_address);
        }

        Ok(())
    }

    async fn listen(&self) -> NodeResult<()> {
        let self_address = self.chain_identity.read().await.get_id_address();

        let node_registry_address = self.node_registry_address.unwrap();

        let node_registry_client = self
            .chain_identity
            .read()
            .await
            .build_node_registry_client(node_registry_address);

        let node = node_registry_client.get_node(self_address).await?;

        if node.id_address == self_address && !node.state {
            self.publish(NodeActivation {
                chain_id: self.chain_id,
                is_eigenlayer: self.is_eigenlayer,
                node_registry_address,
            })
            .await;
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

    async fn chain_id(&self) -> usize {
        self.chain_id
    }
}
