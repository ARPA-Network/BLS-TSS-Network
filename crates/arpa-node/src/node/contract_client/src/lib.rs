use async_trait::async_trait;
use error::ContractClientResult;

pub mod error;
pub mod ethers;
pub mod rpc_mock;

#[async_trait]
pub trait ServiceClient<C> {
    async fn prepare_service_client(&self) -> ContractClientResult<C>;
}

pub mod controller {
    use std::future::Future;

    use arpa_node_core::{DKGTask, GroupRelayTask, Node};

    use async_trait::async_trait;
    use ethers_core::types::Address;

    use crate::error::ContractClientResult;

    #[async_trait]
    pub trait ControllerTransactions {
        async fn node_register(&self, id_public_key: Vec<u8>) -> ContractClientResult<()>;

        async fn commit_dkg(
            &self,
            group_index: usize,
            group_epoch: usize,
            public_key: Vec<u8>,
            partial_public_key: Vec<u8>,
            disqualified_nodes: Vec<Address>,
        ) -> ContractClientResult<()>;

        async fn post_process_dkg(
            &self,
            group_index: usize,
            group_epoch: usize,
        ) -> ContractClientResult<()>;
    }

    #[async_trait]
    pub trait ControllerViews {
        async fn get_node(&self, id_address: Address) -> ContractClientResult<Node>;
    }

    #[async_trait]
    pub trait ControllerLogs {
        async fn subscribe_dkg_task<
            C: FnMut(DKGTask) -> F + Send,
            F: Future<Output = ContractClientResult<()>> + Send,
        >(
            &self,
            cb: C,
        ) -> ContractClientResult<()>;

        async fn subscribe_group_relay_task<
            C: FnMut(GroupRelayTask) -> F + Send,
            F: Future<Output = ContractClientResult<()>> + Send,
        >(
            &self,
            cb: C,
        ) -> ContractClientResult<()>;
    }

    pub trait ControllerClientBuilder {
        type Service: ControllerTransactions + ControllerViews + ControllerLogs + Send + Sync;

        fn build_controller_client(&self) -> Self::Service;
    }
}

pub mod coordinator {
    use async_trait::async_trait;
    use dkg_core::BoardPublisher;
    use ethers_core::types::Address;
    use thiserror::Error;
    use threshold_bls::{group::Curve, schemes::bn254::G2Curve};

    use crate::error::{ContractClientError, ContractClientResult};

    #[derive(Debug, Error)]
    pub enum DKGContractError {
        #[error(transparent)]
        SerializationError(#[from] bincode::Error),
        #[error(transparent)]
        PublishingError(#[from] ContractClientError),
    }

    #[async_trait]
    pub trait CoordinatorTransactions {
        /// Participant publishes their data and depending on the phase the data gets inserted
        /// in the shares, responses or justifications mapping. Reverts if the participant
        /// has already published their data for a phase or if the DKG has ended.
        async fn publish(&self, value: Vec<u8>) -> ContractClientResult<()>;
    }

    #[async_trait]
    pub trait CoordinatorViews {
        // Helpers to fetch data in the mappings. If a participant has registered but not
        // published their data for a phase, the array element at their index is expected to be 0

        /// Gets the participants' shares
        async fn get_shares(&self) -> ContractClientResult<Vec<Vec<u8>>>;

        /// Gets the participants' responses
        async fn get_responses(&self) -> ContractClientResult<Vec<Vec<u8>>>;

        /// Gets the participants' justifications
        async fn get_justifications(&self) -> ContractClientResult<Vec<Vec<u8>>>;

        /// Gets the participants' ethereum addresses
        async fn get_participants(&self) -> ContractClientResult<Vec<Address>>;

        /// Gets the participants' BLS keys along with the thershold of the DKG
        async fn get_bls_keys(&self) -> ContractClientResult<(usize, Vec<Vec<u8>>)>;

        /// Returns the current phase of the DKG.
        async fn in_phase(&self) -> ContractClientResult<usize>;
    }

    pub trait CoordinatorClientBuilder<C: Curve = G2Curve> {
        type Service: CoordinatorTransactions + CoordinatorViews + BoardPublisher<C> + Sync + Send;

        fn build_coordinator_client(&self, contract_address: Address) -> Self::Service;
    }
}

pub mod adapter {
    use arpa_node_core::{
        Group, GroupRelayConfirmationTask, GroupRelayConfirmationTaskState, RandomnessTask,
    };
    use async_trait::async_trait;
    use ethers_core::types::Address;
    use std::{collections::HashMap, future::Future};
    use threshold_bls::group::PairingCurve;

    use crate::error::ContractClientResult;

    #[async_trait]
    pub trait AdapterTransactions {
        async fn request_randomness(&self, message: &str) -> ContractClientResult<()>;

        async fn fulfill_randomness(
            &self,
            group_index: usize,
            signature_index: usize,
            signature: Vec<u8>,
            partial_signatures: HashMap<Address, Vec<u8>>,
        ) -> ContractClientResult<()>;

        async fn fulfill_relay(
            &self,
            relayer_group_index: usize,
            task_index: usize,
            signature: Vec<u8>,
            group_as_bytes: Vec<u8>,
        ) -> ContractClientResult<()>;

        async fn cancel_invalid_relay_confirmation_task(
            &self,
            task_index: usize,
        ) -> ContractClientResult<()>;

        async fn confirm_relay(
            &self,
            task_index: usize,
            group_relay_confirmation_as_bytes: Vec<u8>,
            signature: Vec<u8>,
        ) -> ContractClientResult<()>;

        async fn set_initial_group(&self, group: Vec<u8>) -> ContractClientResult<()>;
    }

    #[async_trait]
    pub trait AdapterViews<C: PairingCurve> {
        async fn get_group(&self, group_index: usize) -> ContractClientResult<Group<C>>;

        async fn get_last_output(&self) -> ContractClientResult<u64>;

        async fn get_signature_task_completion_state(
            &self,
            index: usize,
        ) -> ContractClientResult<bool>;

        async fn get_group_relay_cache(&self, group_index: usize)
            -> ContractClientResult<Group<C>>;

        async fn get_group_relay_confirmation_task_state(
            &self,
            task_index: usize,
        ) -> ContractClientResult<GroupRelayConfirmationTaskState>;
    }

    #[async_trait]
    pub trait AdapterLogs {
        async fn subscribe_randomness_task<
            C: FnMut(RandomnessTask) -> F + Send,
            F: Future<Output = ContractClientResult<()>> + Send,
        >(
            &self,
            cb: C,
        ) -> ContractClientResult<()>;

        async fn subscribe_group_relay_confirmation_task<
            C: FnMut(GroupRelayConfirmationTask) -> F + Send,
            F: Future<Output = ContractClientResult<()>> + Send,
        >(
            &self,
            cb: C,
        ) -> ContractClientResult<()>;
    }

    pub trait AdapterClientBuilder<C: PairingCurve> {
        type Service: AdapterTransactions + AdapterViews<C> + AdapterLogs + Send + Sync;

        fn build_adapter_client(&self, main_id_address: Address) -> Self::Service;
    }
}

pub mod provider {

    use std::future::Future;

    use async_trait::async_trait;

    use crate::error::ContractClientResult;

    #[async_trait]
    pub trait BlockFetcher {
        async fn subscribe_new_block_height<
            C: FnMut(usize) -> F + Send,
            F: Future<Output = ContractClientResult<()>> + Send,
        >(
            &self,
            cb: C,
        ) -> ContractClientResult<()>;
    }

    pub trait ChainProviderBuilder {
        type Service: BlockFetcher + Send + Sync;

        fn build_chain_provider(&self) -> Self::Service;
    }
}
