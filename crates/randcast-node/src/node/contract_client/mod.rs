pub mod ethers;
pub mod rpc_mock;
pub mod types;

pub mod controller {
    use crate::node::{dal::types::Group, error::NodeResult};
    use async_trait::async_trait;

    #[async_trait]
    pub trait ControllerTransactions {
        async fn node_register(&self, id_public_key: Vec<u8>) -> NodeResult<()>;

        async fn commit_dkg(
            &self,
            group_index: usize,
            group_epoch: usize,
            public_key: Vec<u8>,
            partial_public_key: Vec<u8>,
            disqualified_nodes: Vec<String>,
        ) -> NodeResult<()>;

        async fn post_process_dkg(&self, group_index: usize, group_epoch: usize) -> NodeResult<()>;
    }

    #[async_trait]
    pub trait ControllerViews {
        async fn get_group(&self, group_index: usize) -> NodeResult<Group>;
    }
}

pub mod coordinator {
    use crate::node::error::NodeResult;
    use async_trait::async_trait;

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
        async fn get_participants(&self) -> NodeResult<Vec<String>>;

        /// Gets the participants' BLS keys along with the thershold of the DKG
        async fn get_bls_keys(&self) -> NodeResult<(usize, Vec<Vec<u8>>)>;

        /// Returns the current phase of the DKG.
        async fn in_phase(&self) -> NodeResult<usize>;
    }
}

pub mod adapter {
    use crate::node::{
        dal::types::{Group, GroupRelayConfirmationTaskState},
        error::NodeResult,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;

    #[async_trait]
    pub trait AdapterTransactions {
        async fn request_randomness(&self, message: &str) -> NodeResult<()>;

        async fn fulfill_randomness(
            &self,
            group_index: usize,
            signature_index: usize,
            signature: Vec<u8>,
            partial_signatures: HashMap<String, Vec<u8>>,
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
}
