use super::{provider::NONCE, WalletSigner};
use crate::ethers::builders::ContractCall;
use crate::TransactionCaller;
use crate::{
    contract_stub::controller::{CommitDkgParams, Controller, DkgTaskFilter},
    controller::{
        ControllerClientBuilder, ControllerLogs, ControllerTransactions, ControllerViews,
    },
    error::{ContractClientError, ContractClientResult},
    NonceManager, ServiceClient,
};
use arpa_node_core::{ChainIdentity, DKGTask, GeneralChainIdentity, Node};
use async_trait::async_trait;
use ethers::prelude::*;
use log::info;
use std::{convert::TryFrom, future::Future, sync::Arc, time::Duration};

pub struct ControllerClient {
    controller_address: Address,
    signer: Arc<WalletSigner>,
}

impl ControllerClient {
    pub fn new(controller_address: Address, identity: &GeneralChainIdentity) -> Self {
        let provider = Provider::<Http>::try_from(identity.get_provider_rpc_endpoint())
            .unwrap()
            .interval(Duration::from_millis(3000));

        // instantiate the client with the wallet
        let signer = Arc::new(SignerMiddleware::new(
            provider,
            identity
                .get_signer()
                .clone()
                .with_chain_id(identity.get_chain_id() as u32),
        ));

        ControllerClient {
            controller_address,
            signer,
        }
    }
}

impl ControllerClientBuilder for GeneralChainIdentity {
    type Service = ControllerClient;

    fn build_controller_client(&self) -> ControllerClient {
        ControllerClient::new(self.get_contract_address(), self)
    }
}

type ControllerContract = Controller<WalletSigner>;

#[async_trait]
impl ServiceClient<ControllerContract> for ControllerClient {
    async fn prepare_service_client(&self) -> ContractClientResult<ControllerContract> {
        let controller_contract = Controller::new(self.controller_address, self.signer.clone());

        Ok(controller_contract)
    }
}

#[async_trait]
impl TransactionCaller for ControllerClient {
    async fn call_contract_function(
        &self,
        info: &str,
        call: ContractCall<WalletSigner, ()>,
    ) -> ContractClientResult<H256> {
        let pending_tx = call.send().await.map_err(|e| {
            let e: ContractClientError = e.into();
            e
        })?;

        info!(
            "Calling contract function {}: {:?}",
            info,
            pending_tx.tx_hash()
        );

        let receipt = pending_tx
            .await
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?
            .ok_or(ContractClientError::NoTransactionReceipt)?;

        if receipt.status == Some(U64::from(0)) {
            return Err(ContractClientError::TransactionFailed);
        } else {
            info!("Transaction successful({}), receipt: {:?}", info, receipt);
        }

        Ok(receipt.transaction_hash)
    }
}

#[async_trait]
impl NonceManager for ControllerClient {
    async fn increment_or_initialize_nonce(&self) -> ContractClientResult<u64> {
        let mut nonce = NONCE.lock().await;
        if *nonce == -1 {
            let tx_count = self
                .signer
                .get_transaction_count(self.signer.address(), None)
                .await?;
            *nonce = tx_count.as_u64() as i64;
        } else {
            *nonce += 1;
        }

        Ok(*nonce as u64)
    }
}

#[async_trait]
impl ControllerTransactions for ControllerClient {
    async fn node_register(&self, id_public_key: Vec<u8>) -> ContractClientResult<H256> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        let nonce = self.increment_or_initialize_nonce().await?;
        let mut call = controller_contract.node_register(id_public_key.into());
        call.tx.set_nonce(nonce);

        self.call_contract_function("node_register", call).await
    }

    async fn commit_dkg(
        &self,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<Address>,
    ) -> ContractClientResult<H256> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        let nonce = self.increment_or_initialize_nonce().await?;
        let mut call = controller_contract.commit_dkg(CommitDkgParams {
            group_index: group_index.into(),
            group_epoch: group_epoch.into(),
            public_key: public_key.into(),
            partial_public_key: partial_public_key.into(),
            disqualified_nodes,
        });
        call.tx.set_nonce(nonce);

        self.call_contract_function("commit_dkg", call).await
    }

    async fn post_process_dkg(
        &self,
        group_index: usize,
        group_epoch: usize,
    ) -> ContractClientResult<H256> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        let nonce = self.increment_or_initialize_nonce().await?;
        let mut call = controller_contract.post_process_dkg(group_index.into(), group_epoch.into());
        call.tx.set_nonce(nonce);

        self.call_contract_function("post_process_dkg", call).await
    }
}

#[allow(unused_variables)]
#[async_trait]
impl ControllerViews for ControllerClient {
    async fn get_node(&self, id_address: Address) -> ContractClientResult<Node> {
        let controller_contract =
            ServiceClient::<ControllerContract>::prepare_service_client(self).await?;

        let res = controller_contract
            .get_node(id_address)
            .call()
            .await
            .map(|n| Node {
                id_address: n.id_address,
                id_public_key: n.dkg_public_key.to_vec(),
                state: n.state,
                pending_until_block: n.pending_until_block.as_usize(),
                staking: n.staking,
            })
            .map_err(|e| {
                let e: ContractClientError = e.into();
                e
            })?;

        Ok(res)
    }
}

#[async_trait]
impl ControllerLogs for ControllerClient {
    async fn subscribe_dkg_task<
        C: FnMut(DKGTask) -> F + Send,
        F: Future<Output = ContractClientResult<()>> + Send,
    >(
        &self,
        mut cb: C,
    ) -> ContractClientResult<()> {
        let contract = Controller::new(self.controller_address, self.signer.clone());

        let events: Event<WalletSigner, DkgTaskFilter> = contract
            .event::<DkgTaskFilter>()
            .from_block(BlockNumber::Latest);

        // turn the stream into a stream of events
        let mut stream = events.stream().await?.with_meta();

        while let Some(Ok(evt)) = stream.next().await {
            let (
                DkgTaskFilter {
                    group_index,
                    epoch,
                    size,
                    threshold,
                    members,
                    assignment_block_height: _,
                    coordinator_address,
                },
                meta,
            ) = evt;

            info!(
                "Received DKG task: group_index: {}, epoch: {}, size: {}, threshold: {}, members: {:?}, coordinator_address: {}, block_number: {}",
                group_index, epoch, size, threshold, members, coordinator_address, meta.block_number
            );

            let task = DKGTask {
                group_index: group_index.as_usize(),
                epoch: epoch.as_usize(),
                size: size.as_usize(),
                threshold: threshold.as_usize(),
                members,
                assignment_block_height: meta.block_number.as_usize(),
                coordinator_address,
            };
            cb(task).await?;
        }
        Err(ContractClientError::FetchingDkgTaskError)
    }
}
