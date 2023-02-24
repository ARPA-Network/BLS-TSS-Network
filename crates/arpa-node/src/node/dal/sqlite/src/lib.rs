pub mod core;
use crate::core::GroupMutation;
use crate::core::GroupQuery;
use crate::core::NodeMutation;
use crate::core::NodeQuery;
use crate::core::RandomnessTaskMutation;
use crate::core::RandomnessTaskQuery;
use arpa_node_core::Group;
use arpa_node_core::Member;
use arpa_node_core::RANDOMNESS_TASK_EXCLUSIVE_WINDOW;
use arpa_node_core::{address_to_string, format_now_date, RandomnessTask, Task};
use arpa_node_dal::cache::InMemoryGroupInfoCache;
use arpa_node_dal::cache::InMemoryNodeInfoCache;
use arpa_node_dal::error::DataAccessResult;
use arpa_node_dal::error::GroupError;
use arpa_node_dal::error::RandomnessTaskError;
use arpa_node_dal::ContextInfoUpdater;
use arpa_node_dal::NodeInfoUpdater;
use arpa_node_dal::{
    error::DataAccessError, BLSTasksFetcher, BLSTasksUpdater, DKGOutput, GroupInfoFetcher,
    GroupInfoUpdater, NodeInfoFetcher,
};
use async_trait::async_trait;
use entity::group_info;
use entity::node_info;
use ethers_core::types::Address;
use ethers_core::types::U256;
use ethers_core::utils::hex;
use log::LevelFilter;
pub use migration::Migrator;
use migration::MigratorTrait;
use sea_orm::ConnectionTrait;
use sea_orm::DatabaseBackend;
use sea_orm::DbConn;
use sea_orm::QueryResult;
use sea_orm::Statement;
use sea_orm::{ConnectOptions, DatabaseConnection, DbErr};
use std::collections::BTreeMap;
use std::{marker::PhantomData, sync::Arc, time::Duration};
use threshold_bls::group::Curve;
use threshold_bls::group::PairingCurve;
use threshold_bls::serialize::point_to_hex;
use threshold_bls::sig::Share;
mod test_helper;
use std::str;
use thiserror::Error;
use threshold_bls::group::Element;

pub type DBResult<A> = Result<A, DBError>;

#[derive(Debug, Error, PartialEq)]
pub enum DBError {
    #[error("there is no node record yet, please run node with new-run mode")]
    NoNodeRecord,
    #[error(transparent)]
    DbError(#[from] DbErr),
    #[error(transparent)]
    GroupError(#[from] GroupError),
}

impl From<DBError> for DataAccessError {
    fn from(e: DBError) -> Self {
        DataAccessError::DBError(anyhow::Error::from(e))
    }
}

#[derive(Default, Debug, Clone)]
pub struct SqliteDB<C: PairingCurve> {
    connection: Arc<DatabaseConnection>,
    c: PhantomData<C>,
}

impl<C: PairingCurve> SqliteDB<C> {
    pub async fn build(
        db_path: &str,
        signing_key: &[u8],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut opt = ConnectOptions::new(format!("sqlite://{}?mode=rwc", db_path));
        opt.max_connections(100)
            .min_connections(5)
            .connect_timeout(Duration::from_secs(8))
            .idle_timeout(Duration::from_secs(8))
            .max_lifetime(Duration::from_secs(8))
            .sqlx_logging(true)
            .sqlx_logging_level(LevelFilter::Debug)
            .sqlcipher_key(format!("\"x'{}'\"", hex::encode(signing_key)));

        let connection = sea_orm::Database::connect(opt).await?;

        let db = SqliteDB {
            connection: Arc::new(connection),
            c: PhantomData,
        };

        db.integrity_check().await?;

        Migrator::up(&db.connection, None).await?;

        Ok(db)
    }

    pub fn new(connection: Arc<DatabaseConnection>) -> Self {
        SqliteDB {
            connection,
            c: PhantomData,
        }
    }

    pub fn get_node_info_client(&self) -> NodeInfoDBClient<C> {
        NodeInfoDBClient {
            db_client: Arc::new(self.clone()),
            node_info_cache: None,
            node_info_cache_model: None,
        }
    }

    pub fn get_group_info_client(&self) -> GroupInfoDBClient<C> {
        GroupInfoDBClient {
            db_client: Arc::new(self.clone()),
            group_info_cache: None,
            group_info_cache_model: None,
        }
    }

    pub fn get_bls_tasks_client<T: Task>(&self) -> BLSTasksDBClient<T, C> {
        BLSTasksDBClient {
            db_client: Arc::new(self.clone()),
            bls_tasks: PhantomData,
        }
    }

    pub async fn integrity_check(&self) -> DBResult<String> {
        let query_res: Option<QueryResult> = self
            .connection
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA integrity_check;".to_owned(),
            ))
            .await?;

        query_res
            .unwrap()
            .try_get::<String>("", "integrity_check")
            .map_err(|e| e.into())
    }
}

#[derive(Clone)]
pub struct NodeInfoDBClient<C: PairingCurve> {
    db_client: Arc<SqliteDB<C>>,
    node_info_cache_model: Option<node_info::Model>,
    node_info_cache: Option<InMemoryNodeInfoCache<C>>,
}

impl<C: PairingCurve> std::fmt::Debug for NodeInfoDBClient<C> {
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

impl<C: PairingCurve> NodeInfoDBClient<C> {
    pub async fn refresh_current_node_info(&mut self) -> DBResult<()> {
        let conn = &self.db_client.connection;
        let node_info = NodeQuery::find_current_node_info(conn).await?.unwrap();

        let node_info_cache = InMemoryNodeInfoCache::rebuild(
            node_info.id_address.parse().unwrap(),
            node_info.node_rpc_endpoint.clone(),
            bincode::deserialize(&node_info.dkg_private_key).unwrap(),
            bincode::deserialize(&node_info.dkg_public_key).unwrap(),
        );

        node_info_cache.refresh_context_entry();

        self.node_info_cache = Some(node_info_cache);

        self.node_info_cache_model = Some(node_info);

        Ok(())
    }

    pub async fn save_node_info(
        &mut self,
        id_address: Address,
        node_rpc_endpoint: String,
        dkg_private_key: C::Scalar,
        dkg_public_key: C::G2,
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

impl<C: PairingCurve> ContextInfoUpdater for NodeInfoDBClient<C> {
    fn refresh_context_entry(&self) {
        if let Some(cache) = &self.node_info_cache {
            cache.refresh_context_entry();
        }
    }
}

#[derive(Clone)]
pub struct GroupInfoDBClient<C: PairingCurve> {
    db_client: Arc<SqliteDB<C>>,
    group_info_cache_model: Option<group_info::Model>,
    group_info_cache: Option<InMemoryGroupInfoCache<C>>,
}

impl<C: PairingCurve> std::fmt::Debug for GroupInfoDBClient<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupInfo")
            .field("share", &"ignored")
            .field(
                "group",
                &self.group_info_cache.as_ref().map(|e| e.get_group()),
            )
            .field(
                "dkg_status",
                &self.group_info_cache.as_ref().map(|e| e.get_dkg_status()),
            )
            .field(
                "self_index",
                &self.group_info_cache.as_ref().map(|e| e.get_self_index()),
            )
            .field(
                "dkg_start_block_height",
                &self
                    .group_info_cache
                    .as_ref()
                    .map(|e| e.get_dkg_start_block_height()),
            )
            .finish()
    }
}

impl<C: PairingCurve> GroupInfoDBClient<C> {
    pub async fn refresh_current_group_info(&mut self) -> DBResult<()> {
        let conn = &self.db_client.connection;
        let group_info = GroupQuery::find_current_group_info(conn)
            .await?
            .ok_or(GroupError::NoGroupTask)?;

        let group = Group {
            index: group_info.index as usize,
            epoch: group_info.epoch as usize,
            size: group_info.size as usize,
            threshold: group_info.threshold as usize,
            state: group_info.state == 1,
            public_key: group_info
                .public_key
                .as_ref()
                .map(|bytes| bincode::deserialize(bytes).unwrap()),
            members: serde_json::from_str(&group_info.members).unwrap(),
            committers: group_info
                .committers
                .as_ref()
                .map_or(vec![], |str| serde_json::from_str(str).unwrap()),
            c: PhantomData,
        };

        let group_info_cache = InMemoryGroupInfoCache::rebuild(
            group_info
                .share
                .as_ref()
                .map(|bytes| bincode::deserialize(bytes).unwrap()),
            group,
            (group_info.dkg_status as usize).into(),
            group_info.self_member_index as usize,
            group_info.dkg_start_block_height as usize,
        );

        group_info_cache.refresh_context_entry();

        self.group_info_cache = Some(group_info_cache);

        self.group_info_cache_model = Some(group_info);

        Ok(())
    }

    fn only_has_group_task(&self) -> DBResult<()> {
        self.group_info_cache
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| GroupError::NoGroupTask.into())
    }

    pub fn get_connection(&self) -> &DbConn {
        &self.db_client.connection
    }
}

impl<C: PairingCurve> ContextInfoUpdater for GroupInfoDBClient<C> {
    fn refresh_context_entry(&self) {
        if let Some(cache) = &self.group_info_cache {
            cache.refresh_context_entry();
        }
    }
}

#[derive(Debug, Clone)]
pub struct BLSTasksDBClient<T: Task, C: PairingCurve> {
    db_client: Arc<SqliteDB<C>>,
    bls_tasks: PhantomData<T>,
}

impl<C: PairingCurve> BLSTasksDBClient<RandomnessTask, C> {
    pub fn get_connection(&self) -> &DbConn {
        &self.db_client.connection
    }
}

impl<C: PairingCurve> NodeInfoFetcher<C> for NodeInfoDBClient<C> {
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

    fn get_dkg_public_key(&self) -> DataAccessResult<&C::G2> {
        self.node_info_cache.as_ref().unwrap().get_dkg_public_key()
    }
}

#[async_trait]
impl<C: PairingCurve + Sync + Send> NodeInfoUpdater<C> for NodeInfoDBClient<C> {
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
        dkg_public_key: C::G2,
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

impl<C: PairingCurve> GroupInfoFetcher<C> for GroupInfoDBClient<C> {
    fn get_group(&self) -> DataAccessResult<&Group<C>> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_group()
    }

    fn get_index(&self) -> DataAccessResult<usize> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_index()
    }

    fn get_epoch(&self) -> DataAccessResult<usize> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_epoch()
    }

    fn get_size(&self) -> DataAccessResult<usize> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_size()
    }

    fn get_threshold(&self) -> DataAccessResult<usize> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_threshold()
    }

    fn get_state(&self) -> DataAccessResult<bool> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_state()
    }

    fn get_self_index(&self) -> DataAccessResult<usize> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_self_index()
    }

    fn get_public_key(&self) -> DataAccessResult<&C::G2> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_public_key()
    }

    fn get_secret_share(&self) -> DataAccessResult<&Share<C::Scalar>> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_secret_share()
    }

    fn get_members(&self) -> DataAccessResult<&BTreeMap<Address, Member<C>>> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_members()
    }

    fn get_member(&self, id_address: Address) -> DataAccessResult<&arpa_node_core::Member<C>> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_member(id_address)
    }

    fn get_committers(&self) -> DataAccessResult<Vec<Address>> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_committers()
    }

    fn get_dkg_start_block_height(&self) -> DataAccessResult<usize> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_dkg_start_block_height()
    }

    fn get_dkg_status(&self) -> DataAccessResult<arpa_node_core::DKGStatus> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.get_dkg_status()
    }

    fn is_committer(&self, id_address: Address) -> DataAccessResult<bool> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info.is_committer(id_address)
    }
}

#[async_trait]
impl<PC: PairingCurve + Sync + Send> GroupInfoUpdater<PC> for GroupInfoDBClient<PC> {
    async fn save_task_info(
        &mut self,
        self_index: usize,
        task: arpa_node_core::DKGTask,
    ) -> DataAccessResult<()> {
        let members: BTreeMap<Address, Member<PC>> = task
            .members
            .iter()
            .enumerate()
            .map(|(index, address)| {
                let member = Member {
                    index,
                    id_address: *address,
                    rpc_endpoint: None,
                    partial_public_key: None,
                };
                (*address, member)
            })
            .collect();

        GroupMutation::save_task_info(
            self.get_connection(),
            task.group_index as i32,
            task.epoch as i32,
            task.size as i32,
            task.threshold as i32,
            self_index as i32,
            task.assignment_block_height as i32,
            serde_json::to_string(&members).unwrap(),
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_group_info().await?;

        Ok(())
    }

    async fn save_output<C: Curve>(
        &mut self,
        index: usize,
        epoch: usize,
        output: DKGOutput<C>,
    ) -> DataAccessResult<(PC::G2, PC::G2, Vec<Address>)> {
        self.only_has_group_task()?;

        let self_index = self.group_info_cache.as_ref().unwrap().get_self_index()?;

        let mut group = self.group_info_cache.as_ref().unwrap().get_group()?.clone();

        if group.index != index {
            return Err(GroupError::GroupIndexObsolete(group.index).into());
        }

        if group.epoch != epoch {
            return Err(GroupError::GroupEpochObsolete(group.epoch).into());
        }

        if group.state {
            return Err(GroupError::GroupAlreadyReady.into());
        }

        // every member index is started from 0
        let qualified_node_indices = output
            .qual
            .nodes
            .iter()
            .map(|node| node.id() as usize)
            .collect::<Vec<_>>();

        let disqualified_nodes = group
            .members
            .iter()
            .filter(|(_, member)| !qualified_node_indices.contains(&member.index))
            .map(|(id_address, _)| *id_address)
            .collect::<Vec<_>>();

        group.remove_disqualified_nodes(&disqualified_nodes);

        let public_key: PC::G2 =
            bincode::deserialize(&bincode::serialize(&output.public.public_key())?)?;

        let mut partial_public_key = PC::G2::new();

        for (_, member) in group.members.iter_mut() {
            if let Some(node) = output
                .qual
                .nodes
                .iter()
                .find(|node| member.index == node.id() as usize)
            {
                if let Some(rpc_endpoint) = node.get_rpc_endpoint() {
                    member.rpc_endpoint = Some(rpc_endpoint.to_string());
                }
            }

            let member_partial_public_key = bincode::deserialize(&bincode::serialize(
                &output.public.eval(member.index as u32).value,
            )?)?;
            member.partial_public_key = Some(member_partial_public_key);

            if self_index == member.index {
                partial_public_key = member.partial_public_key.clone().unwrap();
            }
        }

        GroupMutation::update_dkg_output(
            self.get_connection(),
            self.group_info_cache_model.to_owned().unwrap(),
            qualified_node_indices.len() as i32,
            bincode::serialize(&public_key).unwrap(),
            bincode::serialize(&output.share).unwrap(),
            serde_json::to_string(&group.members).unwrap(),
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_group_info().await?;

        Ok((public_key, partial_public_key, disqualified_nodes))
    }

    async fn update_dkg_status(
        &mut self,
        index: usize,
        epoch: usize,
        dkg_status: arpa_node_core::DKGStatus,
    ) -> DataAccessResult<bool> {
        self.only_has_group_task()?;

        let current_dkg_status = self.group_info_cache.as_ref().unwrap().get_dkg_status()?;

        let group = self.group_info_cache.as_ref().unwrap().get_group()?;

        if group.index != index {
            return Err(GroupError::GroupIndexObsolete(group.index).into());
        }

        if group.epoch != epoch {
            return Err(GroupError::GroupEpochObsolete(group.epoch).into());
        }

        if current_dkg_status == dkg_status {
            return Ok(false);
        }

        GroupMutation::update_dkg_status(
            self.get_connection(),
            self.group_info_cache_model.to_owned().unwrap(),
            dkg_status.to_usize() as i32,
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_group_info().await?;

        Ok(true)
    }

    async fn save_committers(
        &mut self,
        index: usize,
        epoch: usize,
        committer_indices: Vec<Address>,
    ) -> DataAccessResult<()> {
        self.only_has_group_task()?;

        let group = self.group_info_cache.as_ref().unwrap().get_group()?;

        if group.index != index {
            return Err(GroupError::GroupIndexObsolete(group.index).into());
        }

        if group.epoch != epoch {
            return Err(GroupError::GroupEpochObsolete(group.epoch).into());
        }

        if group.state {
            return Err(GroupError::GroupAlreadyReady.into());
        }

        GroupMutation::update_committers(
            self.get_connection(),
            self.group_info_cache_model.to_owned().unwrap(),
            serde_json::to_string(&committer_indices).unwrap(),
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_group_info().await?;

        Ok(())
    }
}

#[async_trait]
impl<C: PairingCurve + Sync + Send> BLSTasksFetcher<RandomnessTask>
    for BLSTasksDBClient<RandomnessTask, C>
{
    async fn contains(&self, task_request_id: &[u8]) -> DataAccessResult<bool> {
        let conn = &self.db_client.connection;
        let task = RandomnessTaskQuery::select_by_request_id(conn, task_request_id)
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;
        Ok(task.is_some())
    }

    async fn get(&self, task_request_id: &[u8]) -> DataAccessResult<RandomnessTask> {
        let conn = &self.db_client.connection;
        let task = RandomnessTaskQuery::select_by_request_id(conn, task_request_id)
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;

        task.map(|model| RandomnessTask {
            request_id: model.request_id,
            seed: U256::from_big_endian(&model.message),
            group_index: model.group_index as usize,
            request_confirmations: model.request_confirmations as usize,
            assignment_block_height: model.assignment_block_height as usize,
        })
        .ok_or_else(|| {
            RandomnessTaskError::NoRandomnessTask(format!("{:?}", task_request_id)).into()
        })
    }

    async fn is_handled(&self, task_request_id: &[u8]) -> DataAccessResult<bool> {
        let conn = &self.db_client.connection;
        let task = RandomnessTaskQuery::select_by_request_id(conn, task_request_id)
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;

        Ok(task.is_some() && task.unwrap().state == 1)
    }
}

#[async_trait]
impl<C: PairingCurve + Sync + Send> BLSTasksUpdater<RandomnessTask>
    for BLSTasksDBClient<RandomnessTask, C>
{
    async fn add(&mut self, task: RandomnessTask) -> DataAccessResult<()> {
        let mut seed_bytes = vec![0u8; 32];
        task.seed.to_big_endian(&mut seed_bytes);

        RandomnessTaskMutation::add_task(
            self.get_connection(),
            task.request_id,
            task.group_index as i32,
            task.request_confirmations as i32,
            task.assignment_block_height as i32,
            seed_bytes,
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        Ok(())
    }

    async fn check_and_get_available_tasks(
        &mut self,
        current_block_height: usize,
        current_group_index: usize,
    ) -> DataAccessResult<Vec<RandomnessTask>> {
        RandomnessTaskMutation::fetch_available_tasks(
            self.get_connection(),
            current_group_index as i32,
            (current_block_height - RANDOMNESS_TASK_EXCLUSIVE_WINDOW) as i32,
        )
        .await
        .map(|models| {
            models
                .into_iter()
                .map(|model| RandomnessTask {
                    request_id: model.request_id,
                    seed: U256::from_big_endian(&model.message),
                    group_index: model.group_index as usize,
                    request_confirmations: model.request_confirmations as usize,
                    assignment_block_height: model.assignment_block_height as usize,
                })
                .collect::<Vec<_>>()
        })
        .map_err(|e| {
            let e: DBError = e.into();
            let e: DataAccessError = e.into();
            e
        })
    }
}

#[cfg(test)]
pub mod sqlite_tests {
    use crate::test_helper;
    use crate::DBError;
    use crate::SqliteDB;
    use arpa_node_core::DKGStatus;
    use arpa_node_core::DKGTask;
    use arpa_node_core::RandomnessTask;
    use arpa_node_dal::error::GroupError;
    use arpa_node_dal::BLSTasksFetcher;
    use arpa_node_dal::BLSTasksUpdater;
    use arpa_node_dal::GroupInfoFetcher;
    use arpa_node_dal::GroupInfoUpdater;
    use arpa_node_dal::NodeInfoFetcher;
    use arpa_node_dal::NodeInfoUpdater;
    use ethers_core::types::Address;
    use ethers_core::types::U256;
    use std::{fs, path::PathBuf};
    use threshold_bls::curve::bn254::PairingCurve;
    use threshold_bls::group::PairingCurve as PC;
    use threshold_bls::schemes::bn254::G2Curve;
    use threshold_bls::schemes::bn254::G2Scheme;
    use threshold_bls::sig::Scheme;

    const DB_PATH: &str = "test.sqlite";

    const CIPHER_KEY: &str = "passphrase";

    fn setup() {
        if PathBuf::from(DB_PATH).exists() {
            fs::remove_file(DB_PATH).expect("could not remove file");
        }
    }

    fn teardown() {
        fs::remove_file(DB_PATH).expect("could not remove file");
    }

    pub async fn build_sqlite_db<C: PC>() -> Result<SqliteDB<C>, Box<dyn std::error::Error>> {
        SqliteDB::build(DB_PATH, CIPHER_KEY.as_bytes()).await
    }

    #[tokio::test]
    async fn test_build_db() {
        setup();

        let db = build_sqlite_db::<PairingCurve>().await;

        assert!(db.is_ok());

        teardown();
    }

    #[tokio::test]
    async fn test_integrity_check() {
        setup();

        let db = build_sqlite_db::<PairingCurve>().await.unwrap();

        let res = db.integrity_check().await.unwrap();

        assert_eq!("ok".to_owned(), res);

        teardown();
    }

    #[tokio::test]
    async fn test_save_node_info() {
        setup();

        let db = build_sqlite_db::<PairingCurve>().await.unwrap();

        let mut db = db.get_node_info_client();

        let id_address = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        let node_rpc_endpoint = String::from("127.0.0.1");

        let rng = &mut rand::thread_rng();

        let (private_key, public_key) = G2Scheme::keypair(rng);

        if let Err(e) = db
            .save_node_info(id_address, node_rpc_endpoint, private_key, public_key)
            .await
        {
            println!("{:?}", e);
        }

        assert_eq!(id_address, db.get_id_address().unwrap());
        assert_eq!("127.0.0.1", db.get_node_rpc_endpoint().unwrap());

        teardown();
    }

    #[tokio::test]
    async fn test_save_node_rpc_endpoint() {
        setup();

        let db = build_sqlite_db::<PairingCurve>().await.unwrap();

        let mut db = db.get_node_info_client();

        let id_address = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        let node_rpc_endpoint = String::from("127.0.0.1");

        let rng = &mut rand::thread_rng();

        let (private_key, public_key) = G2Scheme::keypair(rng);

        if let Err(e) = db
            .save_node_info(id_address, node_rpc_endpoint, private_key, public_key)
            .await
        {
            println!("{:?}", e);
        }

        let node_rpc_endpoint = String::from("192.168.0.1");

        if let Err(e) = db.set_node_rpc_endpoint(node_rpc_endpoint).await {
            println!("{:?}", e);
        }

        assert_eq!("192.168.0.1", db.get_node_rpc_endpoint().unwrap());

        teardown();
    }

    #[tokio::test]
    async fn test_save_node_dkg_key_pair() {
        setup();

        let db = build_sqlite_db::<PairingCurve>().await.unwrap();

        let mut db = db.get_node_info_client();

        let id_address = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        let node_rpc_endpoint = String::from("127.0.0.1");

        let rng = &mut rand::thread_rng();

        let (private_key, public_key) = G2Scheme::keypair(rng);

        if let Err(e) = db
            .save_node_info(id_address, node_rpc_endpoint, private_key, public_key)
            .await
        {
            println!("{:?}", e);
        }

        let rng = &mut rand::thread_rng();

        let (private_key, public_key) = G2Scheme::keypair(rng);

        if let Err(e) = db.set_dkg_key_pair(private_key, public_key).await {
            println!("{:?}", e);
        }

        assert_eq!(&private_key, db.get_dkg_private_key().unwrap());
        assert_eq!(&public_key, db.get_dkg_public_key().unwrap());

        teardown();
    }

    #[tokio::test]
    async fn test_get_current_group_info_when_no_task() {
        setup();

        let db = build_sqlite_db::<PairingCurve>().await.unwrap();

        let mut db = db.get_group_info_client();

        if let Err(e) = db.refresh_current_group_info().await {
            let ee: DBError = GroupError::NoGroupTask.into();
            assert_eq!(ee, e);
        } else {
            panic!("there should not be a result");
        }

        teardown();
    }

    #[tokio::test]
    async fn test_save_grouping_task_info() {
        setup();
        let db = build_sqlite_db::<PairingCurve>().await.unwrap();

        let mut db = db.get_group_info_client();
        let member_1 = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        let member_2 = "0x0000000000000000000000000000000000000002"
            .parse()
            .unwrap();
        let member_3 = "0x0000000000000000000000000000000000000003"
            .parse()
            .unwrap();
        let members: Vec<Address> = [member_1, member_2, member_3].to_vec();

        let task = DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 3,
            members,
            assignment_block_height: 100,
            coordinator_address: "0x00000000000000000000000000000000000000c1"
                .parse()
                .unwrap(),
        };

        if let Err(e) = db.save_task_info(1, task).await {
            println!("{:?}", e);
        }

        let res = db.group_info_cache.unwrap();
        println!("{:?}", res);

        assert_eq!(3, res.get_size().unwrap());
        assert_eq!(3, res.get_threshold().unwrap());
        assert_eq!(0, res.get_member(member_1).unwrap().index);
        assert_eq!(1, res.get_member(member_2).unwrap().index);
        assert_eq!(2, res.get_member(member_3).unwrap().index);

        teardown();
    }

    #[tokio::test]
    async fn test_update_dkg_status() {
        setup();
        let db = build_sqlite_db::<PairingCurve>().await.unwrap();

        let mut db = db.get_group_info_client();
        let member_1 = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        let member_2 = "0x0000000000000000000000000000000000000002"
            .parse()
            .unwrap();
        let member_3 = "0x0000000000000000000000000000000000000003"
            .parse()
            .unwrap();
        let members: Vec<Address> = [member_1, member_2, member_3].to_vec();

        let task = DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 3,
            members,
            assignment_block_height: 100,
            coordinator_address: "0x00000000000000000000000000000000000000c1"
                .parse()
                .unwrap(),
        };

        if let Err(e) = db.save_task_info(1, task).await {
            println!("{:?}", e);
        }

        if let Err(e) = db.update_dkg_status(1, 1, DKGStatus::InPhase).await {
            println!("{:?}", e);
        }

        let res = db.group_info_cache.unwrap();
        println!("{:?}", res);

        assert_eq!(DKGStatus::InPhase, res.get_dkg_status().unwrap());

        teardown();
    }

    #[tokio::test]
    async fn test_save_output() {
        setup();
        let db = build_sqlite_db::<PairingCurve>().await.unwrap();

        let mut db = db.get_group_info_client();
        let member_1 = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        let member_2 = "0x0000000000000000000000000000000000000002"
            .parse()
            .unwrap();
        let member_3 = "0x0000000000000000000000000000000000000003"
            .parse()
            .unwrap();
        let members: Vec<Address> = [member_1, member_2, member_3].to_vec();

        let task = DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 3,
            members,
            assignment_block_height: 100,
            coordinator_address: "0x00000000000000000000000000000000000000c1"
                .parse()
                .unwrap(),
        };

        if let Err(e) = db.save_task_info(0, task).await {
            println!("{:?}", e);
        }

        let (t, n) = (3, 3);

        println!("nodes setup... t: {} n: {}", t, n);

        let rng = &mut rand::thread_rng();

        let (mut board, phase0s) = test_helper::setup::<G2Curve, G2Scheme, _>(n, t, rng);

        let mut outputs = test_helper::run_dkg::<G2Curve, G2Scheme>(&mut board, phase0s).await;

        let output = outputs.remove(0);

        if let Err(e) = db.save_output(1, 1, output.clone()).await {
            println!("{:?}", e);
        }

        let res = db.group_info_cache.unwrap();
        println!("{:?}", res);

        assert_eq!(3, res.get_size().unwrap());
        assert_eq!(output.share, res.get_secret_share().unwrap().to_owned());
        assert_eq!(output.public.public_key(), res.get_public_key().unwrap());
        assert_eq!(
            Some(output.public.eval(0).value),
            res.get_member(member_1).unwrap().partial_public_key
        );
        assert_eq!(
            Some(output.public.eval(1).value),
            res.get_member(member_2).unwrap().partial_public_key
        );
        assert_eq!(
            Some(output.public.eval(2).value),
            res.get_member(member_3).unwrap().partial_public_key
        );

        teardown();
    }

    #[tokio::test]
    async fn test_add_and_get_randomness_task_with_assigned_group() {
        setup();

        let db = build_sqlite_db::<PairingCurve>().await.unwrap();

        let mut db = db.get_bls_tasks_client::<RandomnessTask>();

        let request_id = vec![1];

        let seed = U256::from_big_endian(&String::from("test task").into_bytes());

        let task = RandomnessTask {
            request_id: request_id.clone(),
            seed,
            group_index: 2,
            request_confirmations: 0,
            assignment_block_height: 100,
        };

        if let Err(e) = db.add(task.clone()).await {
            println!("{:?}", e);
        }

        assert_eq!(true, db.contains(&request_id).await.unwrap());
        assert_eq!(task, db.get(&request_id).await.unwrap());
        assert_eq!(false, db.is_handled(&request_id).await.unwrap());

        let available_tasks = db.check_and_get_available_tasks(100, 1).await.unwrap();
        assert_eq!(0, available_tasks.len());

        let available_tasks = db.check_and_get_available_tasks(100, 2).await.unwrap();
        assert_eq!(1, available_tasks.len());
        assert_eq!(request_id, available_tasks[0].request_id);
        assert_eq!(seed, available_tasks[0].seed);
        assert_eq!(2, available_tasks[0].group_index);
        assert_eq!(100, available_tasks[0].assignment_block_height);

        assert_eq!(true, db.contains(&request_id).await.unwrap());
        assert_eq!(task, db.get(&request_id).await.unwrap());
        assert_eq!(true, db.is_handled(&request_id).await.unwrap());

        let available_tasks = db.check_and_get_available_tasks(100, 2).await.unwrap();
        assert_eq!(0, available_tasks.len());

        teardown();
    }

    #[tokio::test]
    async fn test_add_and_get_randomness_task_over_exclusive_window() {
        setup();

        let db = build_sqlite_db::<PairingCurve>().await.unwrap();

        let mut db = db.get_bls_tasks_client::<RandomnessTask>();

        let request_id = vec![1];

        let seed = U256::from_big_endian(&String::from("test task").into_bytes());

        let task = RandomnessTask {
            request_id: request_id.clone(),
            seed,
            group_index: 2,
            request_confirmations: 0,
            assignment_block_height: 100,
        };

        if let Err(e) = db.add(task.clone()).await {
            println!("{:?}", e);
        }

        assert_eq!(true, db.contains(&request_id).await.unwrap());
        assert_eq!(task, db.get(&request_id).await.unwrap());
        assert_eq!(false, db.is_handled(&request_id).await.unwrap());

        let available_tasks = db.check_and_get_available_tasks(130, 1).await.unwrap();
        assert_eq!(0, available_tasks.len());

        let available_tasks = db.check_and_get_available_tasks(131, 1).await.unwrap();
        assert_eq!(1, available_tasks.len());
        assert_eq!(request_id, available_tasks[0].request_id);
        assert_eq!(seed, available_tasks[0].seed);
        assert_eq!(2, available_tasks[0].group_index);
        assert_eq!(100, available_tasks[0].assignment_block_height);

        assert_eq!(true, db.contains(&request_id).await.unwrap());
        assert_eq!(task, db.get(&request_id).await.unwrap());
        assert_eq!(true, db.is_handled(&request_id).await.unwrap());

        let available_tasks = db.check_and_get_available_tasks(131, 1).await.unwrap();
        assert_eq!(0, available_tasks.len());

        teardown();
    }
}
