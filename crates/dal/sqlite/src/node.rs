use crate::types::DBError;
use crate::types::DBResult;
use crate::types::SqliteDB;
use arpa_core::{address_to_string, format_now_date};
use arpa_dal::cache::InMemoryNodeInfoCache;
use arpa_dal::error::DataAccessResult;
use arpa_dal::ContextInfoUpdater;
use arpa_dal::NodeInfoFetcher;
use arpa_dal::NodeInfoUpdater;
use async_trait::async_trait;
use entity::node_info;
use entity::prelude::NodeInfo;
use ethers_core::types::Address;
use sea_orm::{ActiveModelTrait, DbConn, DbErr, EntityTrait, QueryOrder, Set};
use std::sync::Arc;
use threshold_bls::group::Curve;
use threshold_bls::serialize::point_to_hex;

#[derive(Clone)]
pub struct NodeInfoDBClient<C: Curve> {
    pub(crate) db_client: Arc<SqliteDB>,
    pub(crate) node_info_cache_model: Option<node_info::Model>,
    pub(crate) node_info_cache: Option<InMemoryNodeInfoCache<C>>,
}

impl SqliteDB {
    pub fn get_node_info_client<C: Curve>(&self) -> NodeInfoDBClient<C> {
        NodeInfoDBClient {
            db_client: Arc::new(self.clone()),
            node_info_cache: None,
            node_info_cache_model: None,
        }
    }
}

impl<C: Curve> std::fmt::Debug for NodeInfoDBClient<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeInfo")
            .field(
                "id_address",
                &self.node_info_cache.as_ref().map(|e| e.get_id_address()),
            )
            .field(
                "node_rpc_endpoint",
                &self
                    .node_info_cache
                    .as_ref()
                    .map(|e| e.get_node_rpc_endpoint()),
            )
            .field("dkg_private_key", &"ignored")
            .field(
                "dkg_public_key",
                &self
                    .node_info_cache
                    .as_ref()
                    .map(|e| e.get_dkg_public_key().ok().map(point_to_hex)),
            )
            .finish()
    }
}

impl<C: Curve> NodeInfoDBClient<C> {
    pub async fn refresh_current_node_info(&mut self) -> DBResult<bool> {
        let conn = &self.db_client.connection;
        match NodeQuery::find_current_node_info(conn).await? {
            Some(node_info) => {
                let node_info_cache = InMemoryNodeInfoCache::rebuild(
                    node_info.id_address.parse().unwrap(),
                    node_info.node_rpc_endpoint.clone(),
                    bincode::deserialize(&node_info.dkg_private_key).unwrap(),
                    bincode::deserialize(&node_info.dkg_public_key).unwrap(),
                );

                node_info_cache.refresh_context_entry();

                self.node_info_cache = Some(node_info_cache);

                self.node_info_cache_model = Some(node_info);

                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub async fn save_node_info(
        &mut self,
        id_address: Address,
        node_rpc_endpoint: String,
        dkg_private_key: C::Scalar,
        dkg_public_key: C::Point,
    ) -> DBResult<()> {
        let conn = self.get_connection();

        let model = node_info::Model {
            id: 0,
            id_address: address_to_string(id_address),
            node_rpc_endpoint,
            dkg_private_key: bincode::serialize(&dkg_private_key).unwrap(),
            dkg_public_key: bincode::serialize(&dkg_public_key).unwrap(),
            create_at: format_now_date(),
            update_at: format_now_date(),
        };

        NodeMutation::create_node_info(conn, model).await?;

        self.refresh_current_node_info().await?;

        Ok(())
    }

    pub fn get_connection(&self) -> &DbConn {
        &self.db_client.connection
    }
}

impl<C: Curve> ContextInfoUpdater for NodeInfoDBClient<C> {
    fn refresh_context_entry(&self) {
        if let Some(cache) = &self.node_info_cache {
            cache.refresh_context_entry();
        }
    }
}

impl<C: Curve> NodeInfoFetcher<C> for NodeInfoDBClient<C> {
    fn get_id_address(&self) -> DataAccessResult<Address> {
        self.node_info_cache.as_ref().unwrap().get_id_address()
    }

    fn get_node_rpc_endpoint(&self) -> DataAccessResult<&str> {
        self.node_info_cache
            .as_ref()
            .unwrap()
            .get_node_rpc_endpoint()
    }

    fn get_dkg_private_key(&self) -> DataAccessResult<&C::Scalar> {
        self.node_info_cache.as_ref().unwrap().get_dkg_private_key()
    }

    fn get_dkg_public_key(&self) -> DataAccessResult<&C::Point> {
        self.node_info_cache.as_ref().unwrap().get_dkg_public_key()
    }
}

#[async_trait]
impl<C: Curve + Sync + Send> NodeInfoUpdater<C> for NodeInfoDBClient<C> {
    async fn set_node_rpc_endpoint(&mut self, node_rpc_endpoint: String) -> DataAccessResult<()> {
        NodeMutation::update_node_rpc_endpoint(
            self.get_connection(),
            self.node_info_cache_model.to_owned().unwrap(),
            node_rpc_endpoint,
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_node_info().await?;

        Ok(())
    }

    async fn set_dkg_key_pair(
        &mut self,
        dkg_private_key: C::Scalar,
        dkg_public_key: C::Point,
    ) -> DataAccessResult<()> {
        NodeMutation::update_node_dkg_key_pair(
            self.get_connection(),
            self.node_info_cache_model.to_owned().unwrap(),
            bincode::serialize(&dkg_private_key).unwrap(),
            bincode::serialize(&dkg_public_key).unwrap(),
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_node_info().await?;

        Ok(())
    }
}

pub struct NodeQuery;

impl NodeQuery {
    pub async fn find_current_node_info(db: &DbConn) -> Result<Option<node_info::Model>, DbErr> {
        NodeInfo::find()
            .order_by_desc(node_info::Column::Id)
            .one(db)
            .await
    }
}

pub struct NodeMutation;

impl NodeMutation {
    pub async fn create_node_info(
        db: &DbConn,
        model: node_info::Model,
    ) -> Result<node_info::ActiveModel, DbErr> {
        node_info::ActiveModel {
            id_address: Set(model.id_address.to_owned()),
            node_rpc_endpoint: Set(model.node_rpc_endpoint.to_owned()),
            dkg_private_key: Set(model.dkg_private_key.to_owned()),
            dkg_public_key: Set(model.dkg_public_key.to_owned()),
            create_at: Set(model.create_at.to_owned()),
            update_at: Set(model.update_at.to_owned()),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn update_node_rpc_endpoint(
        db: &DbConn,
        model: node_info::Model,
        node_rpc_endpoint: String,
    ) -> Result<node_info::Model, DbErr> {
        let mut node_info: node_info::ActiveModel = model.into();

        node_info.node_rpc_endpoint = Set(node_rpc_endpoint);

        node_info.update_at = Set(format_now_date());

        node_info.update(db).await
    }

    pub async fn update_node_dkg_key_pair(
        db: &DbConn,
        model: node_info::Model,
        dkg_private_key: Vec<u8>,
        dkg_public_key: Vec<u8>,
    ) -> Result<node_info::Model, DbErr> {
        let mut node_info: node_info::ActiveModel = model.into();

        node_info.dkg_private_key = Set(dkg_private_key);

        node_info.dkg_public_key = Set(dkg_public_key);

        node_info.update_at = Set(format_now_date());

        node_info.update(db).await
    }
}
