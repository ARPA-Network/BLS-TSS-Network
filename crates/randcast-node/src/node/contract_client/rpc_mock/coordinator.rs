use self::coordinator_stub::transactions_client::TransactionsClient as CoordinatorTransactionsClient;
use self::coordinator_stub::views_client::ViewsClient as CoordinatorViewsClient;
use self::coordinator_stub::{BlsKeysReply, PublishRequest};
use crate::node::contract_client::coordinator::{CoordinatorTransactions, CoordinatorViews};
use crate::node::error::{NodeError, NodeResult};
use crate::node::ServiceClient;
use async_trait::async_trait;
use dkg_core::{
    primitives::{BundledJustification, BundledResponses, BundledShares},
    BoardPublisher,
};
use log::info;
use thiserror::Error;
use threshold_bls::curve::bls12381::Curve;
use tonic::metadata::MetadataValue;
use tonic::Request;

pub mod coordinator_stub {
    include!("../../../../stub/coordinator.rs");
}

pub struct MockCoordinatorClient {
    id_address: String,
    coordinator_address: String,
    index: usize,
    epoch: usize,
}

impl MockCoordinatorClient {
    pub fn new(
        coordinator_address: String,
        id_address: String,
        index: usize,
        epoch: usize,
    ) -> Self {
        MockCoordinatorClient {
            id_address,
            coordinator_address,
            index,
            epoch,
        }
    }

    fn set_metadata<T>(&self, req: &mut Request<T>) {
        req.metadata_mut().insert(
            "index",
            MetadataValue::from_str(&self.index.to_string()).unwrap(),
        );

        req.metadata_mut().insert(
            "epoch",
            MetadataValue::from_str(&self.epoch.to_string()).unwrap(),
        );
    }
}

type TransactionsClient = CoordinatorTransactionsClient<tonic::transport::Channel>;

#[async_trait]
impl ServiceClient<TransactionsClient> for MockCoordinatorClient {
    async fn prepare_service_client(&self) -> NodeResult<TransactionsClient> {
        TransactionsClient::connect(format!("{}{}", "http://", self.coordinator_address.clone()))
            .await
            .map_err(|err| err.into())
    }
}

type ViewsClient = CoordinatorViewsClient<tonic::transport::Channel>;

#[async_trait]
impl ServiceClient<ViewsClient> for MockCoordinatorClient {
    async fn prepare_service_client(&self) -> NodeResult<ViewsClient> {
        ViewsClient::connect(format!("{}{}", "http://", self.coordinator_address.clone()))
            .await
            .map_err(|err| err.into())
    }
}

#[async_trait]
impl CoordinatorTransactions for MockCoordinatorClient {
    async fn publish(&self, value: Vec<u8>) -> NodeResult<()> {
        let mut request = Request::new(PublishRequest {
            id_address: self.id_address.to_string(),
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
    async fn get_shares(&self) -> NodeResult<Vec<Vec<u8>>> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_shares(request)
            .await
            .map(|r| r.into_inner().shares)
            .map_err(|status| status.into())
    }

    async fn get_responses(&self) -> NodeResult<Vec<Vec<u8>>> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_responses(request)
            .await
            .map(|r| r.into_inner().responses)
            .map_err(|status| status.into())
    }

    async fn get_justifications(&self) -> NodeResult<Vec<Vec<u8>>> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_justifications(request)
            .await
            .map(|r| r.into_inner().justifications)
            .map_err(|status| status.into())
    }

    async fn get_participants(&self) -> NodeResult<Vec<String>> {
        let mut request: Request<()> = Request::new(());

        self.set_metadata(&mut request);

        let mut views_client = ServiceClient::<ViewsClient>::prepare_service_client(self).await?;

        views_client
            .get_participants(request)
            .await
            .map(|r| r.into_inner().participants)
            .map_err(|status| status.into())
    }

    async fn get_bls_keys(&self) -> NodeResult<(usize, Vec<Vec<u8>>)> {
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

    async fn in_phase(&self) -> NodeResult<usize> {
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

#[derive(Debug, Error)]
pub enum DKGContractError {
    #[error(transparent)]
    SerializationError(#[from] bincode::Error),
    #[error(transparent)]
    PublishingError(#[from] NodeError),
}

#[async_trait]
impl BoardPublisher<Curve> for MockCoordinatorClient {
    type Error = DKGContractError;

    async fn publish_shares(&mut self, shares: BundledShares<Curve>) -> Result<(), Self::Error> {
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
        justifications: BundledJustification<Curve>,
    ) -> Result<(), Self::Error> {
        let serialized = bincode::serialize(&justifications)?;
        self.publish(serialized).await.map_err(|e| e.into())
    }
}
