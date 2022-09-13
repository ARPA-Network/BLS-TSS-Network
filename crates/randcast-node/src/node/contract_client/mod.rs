pub mod ethers;
pub mod rpc_mock;
pub mod types;

pub mod controller {
    use crate::node::{
        dal::types::{DKGTask, GroupRelayTask},
        error::NodeResult,
    };
    use async_trait::async_trait;
    use ethers::types::Address;

    use super::types::Node;

    #[async_trait]
    pub trait ControllerTransactions {
        async fn node_register(&self, id_public_key: Vec<u8>) -> NodeResult<()>;

        async fn commit_dkg(
            &self,
            group_index: usize,
            group_epoch: usize,
            public_key: Vec<u8>,
            partial_public_key: Vec<u8>,
            disqualified_nodes: Vec<Address>,
        ) -> NodeResult<()>;

        async fn post_process_dkg(&self, group_index: usize, group_epoch: usize) -> NodeResult<()>;
    }

    #[async_trait]
    pub trait ControllerViews {
        async fn get_node(&self, id_address: Address) -> NodeResult<Node>;
    }

    #[async_trait]
    pub trait ControllerLogs {
        async fn subscribe_dkg_task(
            &self,
            cb: Box<dyn Fn(DKGTask) -> NodeResult<()> + Sync + Send>,
        ) -> NodeResult<()>;

        async fn subscribe_group_relay_task(
            &self,
            cb: Box<dyn Fn(GroupRelayTask) -> NodeResult<()> + Sync + Send>,
        ) -> NodeResult<()>;
    }

    pub trait ControllerClientBuilder {
        type Service: ControllerTransactions + ControllerViews + ControllerLogs + Sync + Send;

        fn build_controller_client(&self) -> Self::Service;
    }
}

pub mod coordinator {
    use crate::node::error::{NodeError, NodeResult};
    use async_trait::async_trait;
    use dkg_core::BoardPublisher;
    use ethers::types::Address;
    use thiserror::Error;
    use threshold_bls::curve::bls12381::Curve;

    #[derive(Debug, Error)]
    pub enum DKGContractError {
        #[error(transparent)]
        SerializationError(#[from] bincode::Error),
        #[error(transparent)]
        PublishingError(#[from] NodeError),
    }

    #[async_trait]
    pub trait CoordinatorTransactions {
        /// Participant publishes their data and depending on the phase the data gets inserted
        /// in the shares, responses or justifications mapping. Reverts if the participant
        /// has already published their data for a phase or if the DKG has ended.
        async fn publish(&self, value: Vec<u8>) -> NodeResult<()>;
    }

    #[async_trait]
    pub trait CoordinatorViews {
        // Helpers to fetch data in the mappings. If a participant has registered but not
        // published their data for a phase, the array element at their index is expected to be 0

        /// Gets the participants' shares
        async fn get_shares(&self) -> NodeResult<Vec<Vec<u8>>>;

        /// Gets the participants' responses
        async fn get_responses(&self) -> NodeResult<Vec<Vec<u8>>>;

        /// Gets the participants' justifications
        async fn get_justifications(&self) -> NodeResult<Vec<Vec<u8>>>;

        /// Gets the participants' ethereum addresses
        async fn get_participants(&self) -> NodeResult<Vec<Address>>;

        /// Gets the participants' BLS keys along with the thershold of the DKG
        async fn get_bls_keys(&self) -> NodeResult<(usize, Vec<Vec<u8>>)>;

        /// Returns the current phase of the DKG.
        async fn in_phase(&self) -> NodeResult<usize>;
    }

    pub trait CoordinatorClientBuilder {
        type Service: CoordinatorTransactions
            + CoordinatorViews
            + BoardPublisher<Curve>
            + Sync
            + Send;

        fn build_coordinator_client(&self, contract_address: Address) -> Self::Service;
    }
}

pub mod adapter {
    use crate::node::{
        dal::types::{
            Group, GroupRelayConfirmationTask, GroupRelayConfirmationTaskState, RandomnessTask,
        },
        error::NodeResult,
    };
    use async_trait::async_trait;
    use ethers::types::Address;
    use std::collections::HashMap;

    #[async_trait]
    pub trait AdapterTransactions {
        async fn request_randomness(&self, message: &str) -> NodeResult<()>;

        async fn fulfill_randomness(
            &self,
            group_index: usize,
            signature_index: usize,
            signature: Vec<u8>,
            partial_signatures: HashMap<Address, Vec<u8>>,
        ) -> NodeResult<()>;

        async fn fulfill_relay(
            &self,
            relayer_group_index: usize,
            task_index: usize,
            signature: Vec<u8>,
            group_as_bytes: Vec<u8>,
        ) -> NodeResult<()>;

        async fn cancel_invalid_relay_confirmation_task(&self, task_index: usize)
            -> NodeResult<()>;

        async fn confirm_relay(
            &self,
            task_index: usize,
            group_relay_confirmation_as_bytes: Vec<u8>,
            signature: Vec<u8>,
        ) -> NodeResult<()>;

        async fn set_initial_group(&self, group: Vec<u8>) -> NodeResult<()>;
    }

    #[async_trait]
    pub trait AdapterViews {
        async fn get_group(&self, group_index: usize) -> NodeResult<Group>;

        async fn get_last_output(&self) -> NodeResult<u64>;

        async fn get_signature_task_completion_state(&self, index: usize) -> NodeResult<bool>;

        async fn get_group_relay_cache(&self, group_index: usize) -> NodeResult<Group>;

        async fn get_group_relay_confirmation_task_state(
            &self,
            task_index: usize,
        ) -> NodeResult<GroupRelayConfirmationTaskState>;
    }

    #[async_trait]
    pub trait AdapterLogs {
        async fn subscribe_randomness_task(
            &self,
            cb: Box<dyn Fn(RandomnessTask) -> NodeResult<()> + Sync + Send>,
        ) -> NodeResult<()>;

        async fn subscribe_group_relay_confirmation_task(
            &self,
            cb: Box<dyn Fn(GroupRelayConfirmationTask) -> NodeResult<()> + Sync + Send>,
        ) -> NodeResult<()>;
    }

    pub trait AdapterClientBuilder {
        type Service: AdapterTransactions + AdapterViews + AdapterLogs + Sync + Send;

        fn build_adapter_client(&self, main_id_address: Address) -> Self::Service;
    }
}

pub mod provider {

    use async_trait::async_trait;

    use crate::node::error::NodeResult;

    #[async_trait]
    pub trait BlockFetcher {
        async fn subscribe_new_block_height(
            &self,
            cb: Box<dyn Fn(usize) -> NodeResult<()> + Sync + Send>,
        ) -> NodeResult<()>;
    }

    pub trait ChainProviderBuilder {
        type Service: BlockFetcher + Sync + Send;

        fn build_chain_provider(&self) -> Self::Service;
    }
}
