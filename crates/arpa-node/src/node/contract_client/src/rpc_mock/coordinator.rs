use crate::coordinator::{
    CoordinatorClientBuilder, CoordinatorTransactions, CoordinatorViews, DKGContractError,
};
use crate::error::ContractClientResult;
use crate::ServiceClient;

use crate::rpc_stub::coordinator::transactions_client::TransactionsClient as CoordinatorTransactionsClient;
use crate::rpc_stub::coordinator::views_client::ViewsClient as CoordinatorViewsClient;
use crate::rpc_stub::coordinator::{BlsKeysReply, PublishRequest};
use arpa_node_core::{address_to_string, ChainIdentity, MockChainIdentity};
use async_trait::async_trait;
use dkg_core::{
    primitives::{BundledJustification, BundledResponses, BundledShares},
    BoardPublisher,
};
use ethers::types::Address;
use log::info;
use threshold_bls::group::Curve;
use tonic::Request;

pub struct MockCoordinatorClient {
    id_address: Address,
    rpc_endpoint: String,
    coordinator_address: String,
}

impl MockCoordinatorClient {
    pub fn new(rpc_endpoint: String, coordinator_address: String, id_address: Address) -> Self {
        MockCoordinatorClient {
            id_address,
            rpc_endpoint,
            coordinator_address,
        }
    }

    fn set_metadata<T>(&self, req: &mut Request<T>) {
        req.metadata_mut()
            .insert("address", self.coordinator_address.parse().unwrap());
    }
}

impl<C: Curve + 'static> CoordinatorClientBuilder<C> for MockChainIdentity {
    type Service = MockCoordinatorClient;

    fn build_coordinator_client(&self, contract_address: Address) -> MockCoordinatorClient {
        MockCoordinatorClient::new(
            self.get_provider_rpc_endpoint().to_string(),
            address_to_string(contract_address),
            self.get_id_address(),
        )
    }
}

type TransactionsClient = CoordinatorTransactionsClient<tonic::transport::Channel>;

#[async_trait]
impl ServiceClient<TransactionsClient> for MockCoordinatorClient {
    async fn prepare_service_client(&self) -> ContractClientResult<TransactionsClient> {
        TransactionsClient::connect(format!("{}{}", "http://", self.rpc_endpoint.clone()))
            .await
            .map_err(|err| err.into())
    }
}

type ViewsClient = CoordinatorViewsClient<tonic::transport::Channel>;

#[async_trait]
impl ServiceClient<ViewsClient> for MockCoordinatorClient {
    async fn prepare_service_client(&self) -> ContractClientResult<ViewsClient> {
        ViewsClient::connect(format!("{}{}", "http://", self.rpc_endpoint.clone()))
            .await
            .map_err(|err| err.into())
    }
}

#[async_trait]
impl CoordinatorTransactions for MockCoordinatorClient {
    async fn publish(&self, value: Vec<u8>) -> ContractClientResult<()> {
        let mut request = Request::new(PublishRequest {
            id_address: address_to_string(self.id_address),
            value,
        });

        self.set_metadata(&mut request);

        let mut transactions_client =
            ServiceClient::<TransactionsClient>::prepare_service_client(self).await?;

        transactions_client
            .publish(request)
            .await
            .map(|r| r.into_inner())
            .map_err(|status| status.into())
    }
}

#[async_trait]
impl CoordinatorViews for MockCoordinatorClient {
    async fn get_shares(&self) -> ContractClientResult<Vec<Vec<u8>>> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_shares(request)
            .await
            .map(|r| r.into_inner().shares)
            .map_err(|status| status.into())
    }

    async fn get_responses(&self) -> ContractClientResult<Vec<Vec<u8>>> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_responses(request)
            .await
            .map(|r| r.into_inner().responses)
            .map_err(|status| status.into())
    }

    async fn get_justifications(&self) -> ContractClientResult<Vec<Vec<u8>>> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_justifications(request)
            .await
            .map(|r| r.into_inner().justifications)
            .map_err(|status| status.into())
    }

    async fn get_participants(&self) -> ContractClientResult<Vec<Address>> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_participants(request)
            .await
            .map(|r| {
                r.into_inner()
                    .participants
                    .iter()
                    .map(|p| p.parse().unwrap())
                    .collect::<Vec<Address>>()
            })
            .map_err(|status| status.into())
    }

    async fn get_bls_keys(&self) -> ContractClientResult<(usize, Vec<Vec<u8>>)> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_bls_keys(request)
            .await
            .map(|r| {
                let BlsKeysReply {
                    threshold,
                    bls_keys,
                } = r.into_inner();
                (threshold as usize, bls_keys)
            })
            .map_err(|status| status.into())
    }

    async fn in_phase(&self) -> ContractClientResult<usize> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .in_phase(request)
            .await
            .map(|r| r.into_inner().phase as usize)
            .map_err(|status| status.into())
    }
}

#[async_trait]
impl<C: Curve + 'static> BoardPublisher<C> for MockCoordinatorClient {
    type Error = DKGContractError;

    async fn publish_shares(&mut self, shares: BundledShares<C>) -> Result<(), Self::Error> {
        info!("called publish_shares");
        let serialized = bincode::serialize(&shares)?;
        self.publish(serialized).await.map_err(|e| e.into())
    }

    async fn publish_responses(&mut self, responses: BundledResponses) -> Result<(), Self::Error> {
        info!("called publish_responses");
        let serialized = bincode::serialize(&responses)?;
        self.publish(serialized).await.map_err(|e| e.into())
    }

    async fn publish_justifications(
        &mut self,
        justifications: BundledJustification<C>,
    ) -> Result<(), Self::Error> {
        let serialized = bincode::serialize(&justifications)?;
        self.publish(serialized).await.map_err(|e| e.into())
    }
}
