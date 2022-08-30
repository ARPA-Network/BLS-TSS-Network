use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::path::PathBuf;

use crate::node::contract_client::types::SIGNATURE_TASK_EXCLUSIVE_WINDOW;
use crate::node::error::errors::{GroupError, NodeInfoError, NodeResult};

use super::api::{
    BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, NodeInfoFetcher,
    NodeInfoUpdater,
};
use super::cache::InMemoryNodeInfoCache;
use super::types::{DKGStatus, Member, RandomnessTask, Task};
use super::utils::format_now_date;
use super::{cache::InMemoryGroupInfoCache, types::Group};
use log::info;
use rusqlite::Connection;
use thiserror::Error;
use threshold_bls::curve::bls12381::{Scalar, G1};
use threshold_bls::group::Element;
use threshold_bls::sig::Share;

#[derive(Debug)]
struct GroupInfo {
    group_info_id: usize,
    group_id: usize,
    cache: InMemoryGroupInfoCache,
}

#[derive(Debug)]
struct GroupInfoRecord {
    id: usize,
    share: Option<Vec<u8>>,
    group_id: usize,
    dkg_status: usize,
    self_index: usize,
    dkg_start_block_height: usize,
}

#[derive(Debug)]
struct NodeInfo {
    node_info_id: usize,
    cache: InMemoryNodeInfoCache,
}

#[derive(Debug)]
struct NodeInfoRecord {
    id: usize,
    id_address: String,
    node_rpc_endpoint: String,
    dkg_private_key: Option<Vec<u8>>,
    dkg_public_key: Option<Vec<u8>>,
}

impl NodeInfoRecord {
    fn into_node_info(self) -> NodeInfo {
        NodeInfo {
            node_info_id: self.id,
            cache: self.into(),
        }
    }
}

impl From<NodeInfoRecord> for InMemoryNodeInfoCache {
    fn from(node_info: NodeInfoRecord) -> Self {
        InMemoryNodeInfoCache {
            id_address: node_info.id_address,
            node_rpc_endpoint: Some(node_info.node_rpc_endpoint),
            dkg_private_key: node_info
                .dkg_private_key
                .as_ref()
                .map(|bytes| bincode::deserialize(bytes).unwrap()),
            dkg_public_key: node_info
                .dkg_public_key
                .as_ref()
                .map(|bytes| bincode::deserialize(bytes).unwrap()),
        }
    }
}

impl GroupInfoRecord {
    fn into_group_info(self, group: GroupRecord) -> GroupInfo {
        let group_id = group.id;
        let cache = InMemoryGroupInfoCache {
            share: self
                .share
                .as_ref()
                .map(|bytes| bincode::deserialize(bytes).unwrap()),
            group: group.into(),
            dkg_status: self.dkg_status.into(),
            self_index: self.self_index,
            dkg_start_block_height: self.dkg_start_block_height,
        };

        GroupInfo {
            group_info_id: self.id,
            group_id,
            cache,
        }
    }
}

#[derive(Debug)]
pub struct GroupRecord {
    id: usize,
    index: usize,
    epoch: usize,
    size: usize,
    threshold: usize,
    state: bool,
    public_key: Option<Vec<u8>>,
    members: String,
    committers: String,
}

impl From<GroupRecord> for Group {
    fn from(gr: GroupRecord) -> Self {
        Group {
            index: gr.index,
            epoch: gr.epoch,
            size: gr.size,
            threshold: gr.threshold,
            state: gr.state,
            public_key: gr
                .public_key
                .map(|bytes| bincode::deserialize(&bytes).unwrap()),
            members: serde_json::from_str(&gr.members).unwrap(),
            committers: serde_json::from_str(&gr.committers).unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct BLSTasksDBClient<T: Task> {
    db_client: DBClient,
    bls_tasks: PhantomData<T>,
}

pub type DBResult<A> = Result<A, DBError>;

#[derive(Debug, Error, PartialEq)]
pub enum DBError {
    #[error(transparent)]
    DBError(#[from] rusqlite::Error),

    #[error(transparent)]
    NodeInfoError(#[from] NodeInfoError),

    #[error(transparent)]
    GroupError(#[from] GroupError),
}

pub fn init_tables(db_path: PathBuf) -> DBResult<()> {
    let db = DBClient::new(db_path);
    db.init_build()
}

#[derive(Debug)]
struct DBClient {
    db_path: PathBuf,
}

impl DBClient {
    fn new(path: PathBuf) -> Self {
        DBClient { db_path: path }
    }

    fn get_connection(&self) -> DBResult<Connection> {
        match self.db_path.as_os_str().to_str().unwrap() {
            "memory" => Ok(Connection::open_in_memory()?),
            _ => Ok(Connection::open(&self.db_path)?),
        }
    }

    fn init_build(&self) -> DBResult<()> {
        let conn = self.get_connection()?;

        // conn.execute(
        //     "create table if not exists t_block_info (
        //          id integer primary key,
        //          current_block_height integer not null unique,
        //          date text
        //      )",
        //     [],
        // )?;
        conn.execute(
            "create table if not exists t_node_info (
             id integer primary key,
             id_address text,
             node_rpc_endpoint text,
             dkg_private_key blob,
             dkg_public_key blob,
             date text
         )",
            [],
        )?;
        conn.execute(
            "create table if not exists t_group (
             id integer primary key,
             g_index integer not null,
             epoch integer not null,
             size integer not null,
             threshold integer not null,
             state integer not null,
             public_key blob,
             members text,
             committers text,
             date text
         )",
            [],
        )?;
        conn.execute(
            "create table if not exists t_group_info (
             id integer primary key,
             share blob,
             group_id integer not null,
             dkg_status integer not null,
             self_index integer not null,
             dkg_start_block_height integer not null,
             date text
         )",
            [],
        )?;
        // conn.execute(
        //     "create table if not exists t_dkg_task (
        //          id integer primary key,
        //          g_index integer not null,
        //          epoch integer not null,
        //          size integer not null,
        //          threshold integer not null,
        //          members text,
        //          assignment_block_height integer not null,
        //          coordinator_address text not null,
        //          state integer not null,
        //          date text
        //      )",
        //     [],
        // )?;
        conn.execute(
            "create table if not exists t_randomness_task (
             id integer primary key,
             t_index integer not null,
             g_index integer not null,
             message text not null,
             assignment_block_height integer not null,
             state integer not null,
             date text
         )",
            [],
        )?;

        Ok(())
    }
}

pub struct NodeInfoDBClient {
    db_client: DBClient,
    node_info_cache: Option<NodeInfo>,
}

impl NodeInfoDBClient {
    pub fn new(db_path: PathBuf) -> Self {
        NodeInfoDBClient {
            db_client: DBClient::new(db_path),
            node_info_cache: None,
        }
    }

    pub fn save_node_info(
        &mut self,
        id_address: String,
        node_rpc_endpoint: String,
    ) -> DBResult<()> {
        let mut conn = self.db_client.get_connection()?;
        let tx = conn.transaction().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        let dkg_private_key: Option<Vec<u8>> = None;
        let dkg_public_key: Option<Vec<u8>> = None;

        tx.execute(
            "insert into t_node_info(id_address, node_rpc_endpoint, dkg_private_key, dkg_public_key, date) 
            values(?1, ?2, ?3, ?4, ?5)",
            (id_address, node_rpc_endpoint, dkg_private_key, dkg_public_key, format_now_date()),
        )
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        tx.commit().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_node_info()?;
        Ok(())
    }

    pub fn refresh_current_node_info(&mut self) -> DBResult<()> {
        let conn = self.db_client.get_connection()?;
        let mut stmt = conn.prepare(
            "select id, id_address, node_rpc_endpoint, dkg_private_key, dkg_public_key
                from t_node_info order 
                by id desc limit 1",
        )?;
        let node_info_record = stmt
            .query_row([], |row| {
                Ok(NodeInfoRecord {
                    id: row.get(0)?,
                    id_address: row.get(1)?,
                    node_rpc_endpoint: row.get(2)?,
                    dkg_private_key: row.get(3)?,
                    dkg_public_key: row.get(4)?,
                })
            })
            .map_err(|_| NodeInfoError::NoNodeRecord)?;

        self.node_info_cache = Some(node_info_record.into_node_info());

        Ok(())
    }
}

impl NodeInfoUpdater for NodeInfoDBClient {
    fn set_node_rpc_endpoint(&mut self, node_rpc_endpoint: String) -> NodeResult<()> {
        let node_info = self.node_info_cache.as_ref().unwrap();
        let node_info_id = node_info.node_info_id;

        let mut conn = self.db_client.get_connection()?;
        let tx = conn.transaction().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_node_info()?;

        tx.execute(
            "update t_node_info set node_rpc_endpoint = (?1) where id = (?2)",
            (node_rpc_endpoint, node_info_id),
        )
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        tx.commit().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_node_info()?;
        Ok(())
    }

    fn set_dkg_key_pair(&mut self, dkg_private_key: Scalar, dkg_public_key: G1) -> NodeResult<()> {
        let node_info = self.node_info_cache.as_ref().unwrap();
        let node_info_id = node_info.node_info_id;

        let mut conn = self.db_client.get_connection()?;
        let tx = conn.transaction().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_node_info()?;

        tx.execute(
            "update t_node_info set dkg_private_key = (?1), dkg_public_key = (?2) where id = (?3)",
            (
                Some(bincode::serialize(&dkg_private_key).unwrap()),
                Some(bincode::serialize(&dkg_public_key).unwrap()),
                node_info_id,
            ),
        )
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        tx.commit().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_node_info()?;
        Ok(())
    }
}

impl NodeInfoFetcher for NodeInfoDBClient {
    fn get_id_address(&self) -> &str {
        let node_info = &self.node_info_cache.as_ref().unwrap().cache;
        &node_info.id_address
    }

    fn get_node_rpc_endpoint(&self) -> NodeResult<&str> {
        let node_info = &self.node_info_cache.as_ref().unwrap().cache;
        node_info
            .node_rpc_endpoint
            .as_ref()
            .map(|e| e as &str)
            .ok_or_else(|| NodeInfoError::NoRpcEndpoint.into())
    }

    fn get_dkg_private_key(&self) -> NodeResult<&Scalar> {
        let node_info = &self.node_info_cache.as_ref().unwrap().cache;
        node_info
            .dkg_private_key
            .as_ref()
            .ok_or_else(|| NodeInfoError::NoDKGKeyPair.into())
    }

    fn get_dkg_public_key(&self) -> NodeResult<&G1> {
        let node_info = &self.node_info_cache.as_ref().unwrap().cache;
        node_info
            .dkg_public_key
            .as_ref()
            .ok_or_else(|| NodeInfoError::NoDKGKeyPair.into())
    }
}

pub struct GroupInfoDBClient {
    db_client: DBClient,
    group_info_cache: Option<GroupInfo>,
}

impl GroupInfoDBClient {
    pub fn new(db_path: PathBuf) -> Self {
        GroupInfoDBClient {
            db_client: DBClient::new(db_path),
            group_info_cache: None,
        }
    }

    pub fn refresh_current_group_info(&mut self) -> DBResult<()> {
        let conn = self.db_client.get_connection()?;
        let mut stmt = conn.prepare(
            "select id, share, group_id, dkg_status, self_index, dkg_start_block_height 
                from t_group_info order 
                by id desc limit 1",
        )?;
        let group_info_record = stmt
            .query_row([], |row| {
                Ok(GroupInfoRecord {
                    id: row.get(0)?,
                    share: row.get(1)?,
                    group_id: row.get(2)?,
                    dkg_status: row.get(3)?,
                    self_index: row.get(4)?,
                    dkg_start_block_height: row.get(5)?,
                })
            })
            .map_err(|_| GroupError::NoGroupTask)?;

        let mut stmt = conn.prepare(
            "select g_index, epoch, size, threshold, state, public_key, members, committers
                        from t_group where id = (?)",
        )?;
        let group = stmt
            .query_row([group_info_record.group_id], |row| {
                Ok(GroupRecord {
                    id: group_info_record.group_id,
                    index: row.get(0)?,
                    epoch: row.get(1)?,
                    size: row.get(2)?,
                    threshold: row.get(3)?,
                    state: row.get(4)?,
                    public_key: row.get(5)?,
                    members: row.get(6)?,
                    committers: row.get(7)?,
                })
            })
            .map_err(|_| GroupError::GroupNotExisted)?;

        self.group_info_cache = Some(group_info_record.into_group_info(group));

        Ok(())
    }

    fn only_has_group_task(&self) -> DBResult<()> {
        self.group_info_cache
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| GroupError::NoGroupTask.into())
    }
}

impl GroupInfoFetcher for GroupInfoDBClient {
    fn get_index(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();
        let group = &group_info.cache.group;

        Ok(group.index)
    }

    fn get_epoch(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();
        let group = &group_info.cache.group;

        Ok(group.epoch)
    }

    fn get_size(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();
        let group = &group_info.cache.group;

        Ok(group.size)
    }

    fn get_threshold(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();
        let group = &group_info.cache.group;

        Ok(group.threshold)
    }

    fn get_state(&self) -> NodeResult<bool> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();
        let group = &group_info.cache.group;

        Ok(group.state)
    }

    fn get_public_key(&self) -> NodeResult<&G1> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();
        let group = &group_info.cache.group;

        group
            .public_key
            .as_ref()
            .ok_or(GroupError::GroupNotExisted)
            .map_err(|e| e.into())
    }

    fn get_secret_share(&self) -> NodeResult<&Share<Scalar>> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        group_info
            .cache
            .share
            .as_ref()
            .ok_or(GroupError::GroupNotReady)
            .map_err(|e| e.into())
    }

    fn get_member(&self, id_address: &str) -> NodeResult<&Member> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();
        let group = &group_info.cache.group;

        group
            .members
            .get(id_address)
            .ok_or(GroupError::GroupNotExisted)
            .map_err(|e| e.into())
    }

    fn get_committers(&self) -> NodeResult<Vec<&str>> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();
        let group = &group_info.cache.group;

        Ok(group
            .committers
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>())
    }

    fn get_dkg_start_block_height(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        Ok(group_info.cache.dkg_start_block_height)
    }

    fn get_dkg_status(&self) -> NodeResult<DKGStatus> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();

        Ok(group_info.cache.dkg_status)
    }

    fn is_committer(&self, id_address: &str) -> NodeResult<bool> {
        self.only_has_group_task()?;

        let group_info = self.group_info_cache.as_ref().unwrap();
        let group = &group_info.cache.group;

        Ok(group.committers.contains(&id_address.to_string()))
    }
}

impl GroupInfoUpdater for GroupInfoDBClient {
    fn update_dkg_status(
        &mut self,
        index: usize,
        epoch: usize,
        dkg_status: super::types::DKGStatus,
    ) -> NodeResult<bool> {
        self.only_has_group_task()?;

        self.refresh_current_group_info()?;
        let group_info = self.group_info_cache.as_ref().unwrap();

        let group_info_id = group_info.group_info_id;

        let group = &group_info.cache.group;

        if index == group.index && epoch == group.epoch {
            let mut conn = self.db_client.get_connection()?;
            let tx = conn.transaction().map_err(|e| {
                let e: DBError = e.into();
                e
            })?;

            tx.execute(
                "update t_group_info set dkg_status = (?1) where id = (?2)",
                [dkg_status.to_usize(), group_info_id],
            )
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;

            info!("dkg_status transfered to {:?} and saved", dkg_status);

            tx.commit().map_err(|e| {
                let e: DBError = e.into();
                e
            })?;

            self.refresh_current_group_info()?;
            return Ok(true);
        }

        Ok(false)
    }

    fn save_task_info(&mut self, self_index: usize, task: super::types::DKGTask) -> NodeResult<()> {
        let mut conn = self.db_client.get_connection()?;
        let tx = conn.transaction().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        let members: BTreeMap<String, Member> = task
            .members
            .iter()
            .map(|(address, index)| {
                let member = Member {
                    index: *index,
                    id_address: address.to_string(),
                    rpc_endpint: None,
                    partial_public_key: None,
                };
                (address.to_string(), member)
            })
            .collect();

        let public_key: Option<Vec<u8>> = None;

        let committers: Vec<String> = vec![];

        tx.execute(
            "insert into t_group(g_index, epoch, size, threshold, state, public_key, members, committers, date) 
            values(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (task.group_index,task.epoch,task.size,task.threshold,false,public_key,
                serde_json::to_string(&members).unwrap(),serde_json::to_string(&committers).unwrap(),format_now_date()),
        )
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        let group_id = tx.last_insert_rowid();

        let share: Option<Vec<u8>> = None;

        tx.execute(
            "insert into t_group_info(share, group_id, dkg_status, self_index, dkg_start_block_height, date) values(?1, ?2, ?3, ?4, ?5, ?6)",
            (share, group_id, DKGStatus::None.to_usize(),self_index,task.assignment_block_height,format_now_date()),
        )
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        tx.commit().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_group_info()?;
        Ok(())
    }

    fn save_output(
        &mut self,
        index: usize,
        epoch: usize,
        output: dkg_core::primitives::DKGOutput<threshold_bls::curve::bls12381::Curve>,
    ) -> NodeResult<(
        threshold_bls::curve::bls12381::G1,
        threshold_bls::curve::bls12381::G1,
        Vec<String>,
    )> {
        self.only_has_group_task()?;

        self.refresh_current_group_info()?;
        let group_info = self.group_info_cache.as_ref().unwrap();

        let group_info_id = group_info.group_info_id;

        let group_id = group_info.group_id;

        let mut group = group_info.cache.group.clone();

        if group.index != index {
            return Err(GroupError::GroupIndexObsolete(group.index).into());
        }

        if group.epoch != epoch {
            return Err(GroupError::GroupEpochObsolete(group.epoch).into());
        }

        if group.state {
            return Err(GroupError::GroupAlreadyReady.into());
        }

        let mut conn = self.db_client.get_connection()?;
        let tx = conn.transaction().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

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
            .map(|(id_address, _)| id_address.to_string())
            .collect::<Vec<_>>();

        group
            .members
            .retain(|node, _| !disqualified_nodes.contains(node));

        let public_key = *output.public.public_key();

        let mut partial_public_key = G1::new();

        for (_, member) in group.members.iter_mut() {
            if let Some(node) = output
                .qual
                .nodes
                .iter()
                .find(|node| member.index == node.id() as usize)
            {
                if let Some(rpc_endpoint) = node.get_rpc_endpoint() {
                    member.rpc_endpint = Some(rpc_endpoint.to_string());
                }
            }

            member.partial_public_key = Some(output.public.eval(member.index as u32).value);

            if group_info.cache.self_index == member.index {
                partial_public_key = member.partial_public_key.unwrap();
            }
        }

        tx.execute(
            "update t_group_info set share = (?1) where id = (?2)",
            (
                Some(bincode::serialize(&output.share).unwrap()),
                group_info_id,
            ),
        )
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        tx.execute(
            "update t_group set size = (?1), public_key = (?2), members = (?3) where id = (?4)",
            (
                qualified_node_indices.len(),
                Some(bincode::serialize(&public_key).unwrap()),
                serde_json::to_string(&group.members).unwrap(),
                group_id,
            ),
        )
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        tx.commit().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_group_info()?;
        Ok((public_key, partial_public_key, disqualified_nodes))
    }

    fn save_committers(
        &mut self,
        index: usize,
        epoch: usize,
        committer_indices: Vec<String>,
    ) -> NodeResult<()> {
        self.only_has_group_task()?;

        self.refresh_current_group_info()?;
        let group_info = self.group_info_cache.as_ref().unwrap();

        let group_id = group_info.group_id;

        let group = &group_info.cache.group;

        if group.index != index {
            return Err(GroupError::GroupIndexObsolete(group.index).into());
        }

        if group.epoch != epoch {
            return Err(GroupError::GroupEpochObsolete(group.epoch).into());
        }

        if group.state {
            return Err(GroupError::GroupAlreadyReady.into());
        }

        let mut conn = self.db_client.get_connection()?;
        let tx = conn.transaction().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        tx.execute(
            "update t_group set state = (?1) and committers = (?2) where id = (?3)",
            (
                true,
                serde_json::to_string(&committer_indices).unwrap(),
                group_id,
            ),
        )
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        tx.commit().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.refresh_current_group_info()?;
        Ok(())
    }
}

impl BLSTasksDBClient<RandomnessTask> {
    pub fn new(db_path: PathBuf) -> Self {
        BLSTasksDBClient {
            db_client: DBClient::new(db_path),
            bls_tasks: PhantomData,
        }
    }
}

impl BLSTasksFetcher<RandomnessTask> for BLSTasksDBClient<RandomnessTask> {
    fn contains(&self, task_index: usize) -> NodeResult<bool> {
        let conn = self.db_client.get_connection()?;
        let mut stmt = conn
            .prepare("select id from t_randomness_task where id = (?1)")
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;
        stmt.exists([task_index]).map_err(|e| {
            let e: DBError = e.into();
            e.into()
        })
    }

    fn get(&self, task_index: usize) -> NodeResult<RandomnessTask> {
        let conn = self.db_client.get_connection()?;
        let mut stmt = conn
            .prepare("select t_index, message, g_index, assignment_block_height from t_randomness_task where id = (?1)")
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;

        stmt.query_row([task_index], |row| {
            Ok(RandomnessTask {
                index: row.get(0)?,
                message: row.get(1)?,
                group_index: row.get(2)?,
                assignment_block_height: row.get(3)?,
            })
        })
        .map_err(|e| {
            let e: DBError = e.into();
            e.into()
        })
    }

    fn is_handled(&self, task_index: usize) -> NodeResult<bool> {
        let conn = self.db_client.get_connection()?;
        let mut stmt = conn
            .prepare("select id from t_randomness_task where id = (?1) and state = 1")
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;
        stmt.exists([task_index]).map_err(|e| {
            let e: DBError = e.into();
            e.into()
        })
    }
}

impl BLSTasksUpdater<RandomnessTask> for BLSTasksDBClient<RandomnessTask> {
    fn add(&mut self, task: RandomnessTask) -> NodeResult<()> {
        let mut conn = self.db_client.get_connection()?;
        let tx = conn.transaction().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        tx.execute(
            "insert into t_randomness_task(t_index, g_index, message, assignment_block_height, state, date) values(?1, ?2, ?3, ?4, ?5, ?6)",
            (task.index, task.group_index, task.message, task.assignment_block_height, false, format_now_date()),
        )
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        tx.commit().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        Ok(())
    }

    fn check_and_get_available_tasks(
        &mut self,
        current_block_height: usize,
        current_group_index: usize,
    ) -> NodeResult<Vec<RandomnessTask>> {
        let mut conn = self.db_client.get_connection()?;
        let tx = conn.transaction().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;
        let mut stmt = tx
            .prepare(
                "update t_randomness_task set state = 1 where state = 0 and (g_index = (?1) or assignment_block_height < (?2)) \
                returning t_index, message, g_index, assignment_block_height",
            )
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;

        let rows = stmt
            .query_map(
                (
                    current_group_index,
                    current_block_height - SIGNATURE_TASK_EXCLUSIVE_WINDOW,
                ),
                |row| {
                    Ok(RandomnessTask {
                        index: row.get(0)?,
                        message: row.get(1)?,
                        group_index: row.get(2)?,
                        assignment_block_height: row.get(3)?,
                    })
                },
            )
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;

        let available_tasks = rows.into_iter().map(|w| w.unwrap()).collect::<Vec<_>>();

        drop(stmt);

        tx.commit().map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        Ok(available_tasks)
    }
}

#[cfg(test)]
pub mod sqlite_tests {
    use super::{BLSTasksDBClient, DBClient, DBError, GroupInfoDBClient, NodeInfoDBClient};
    use crate::node::{
        dal::{
            api::{
                BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater,
                NodeInfoFetcher, NodeInfoUpdater,
            },
            test_helper,
            types::{DKGStatus, DKGTask, RandomnessTask},
        },
        error::errors::GroupError,
    };
    use std::{collections::BTreeMap, fs, path::PathBuf};
    use threshold_bls::{
        curve::bls12381::{self},
        schemes::bls12_381::G1Scheme,
        sig::Scheme,
    };

    const DB_PATH: &str = "test.sqlite";

    fn setup() {
        if PathBuf::from(DB_PATH).exists() {
            fs::remove_file(DB_PATH).expect("could not remove file");
        }
        let db = DBClient::new(PathBuf::from(DB_PATH));
        db.init_build().expect("error when creating tables");
    }

    fn teardown() {
        fs::remove_file(DB_PATH).expect("could not remove file");
    }

    fn build_node_info_db_client() -> NodeInfoDBClient {
        NodeInfoDBClient::new(PathBuf::from(DB_PATH))
    }

    fn build_group_info_db_client() -> GroupInfoDBClient {
        GroupInfoDBClient::new(PathBuf::from(DB_PATH))
    }

    fn build_randomness_tasks_db_client() -> BLSTasksDBClient<RandomnessTask> {
        BLSTasksDBClient::new(PathBuf::from(DB_PATH))
    }

    #[test]
    fn test_create_tables() {
        let db = DBClient::new(PathBuf::from(DB_PATH));
        db.init_build().expect("error when creating tables");
        fs::remove_file(DB_PATH).expect("could not remove file");
    }

    #[test]
    fn test_get_current_group_info_when_no_task() {
        setup();

        let mut db = build_group_info_db_client();

        if let Err(e) = db.refresh_current_group_info() {
            let ee: DBError = GroupError::NoGroupTask.into();
            assert_eq!(ee, e);
        } else {
            panic!("there should not be a result");
        }

        teardown();
    }

    #[test]
    fn test_save_task_info() {
        setup();
        let mut db = build_group_info_db_client();
        let mut members: BTreeMap<String, usize> = BTreeMap::new();
        members.insert("0x1".to_string(), 0);
        members.insert("0x2".to_string(), 1);
        members.insert("0x3".to_string(), 2);

        let task = DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 3,
            members,
            assignment_block_height: 100,
            coordinator_address: "0xcoordinator".to_string(),
        };

        if let Err(e) = db.save_task_info(1, task) {
            println!("{:?}", e);
        }

        if let Err(e) = db.refresh_current_group_info() {
            println!("{:?}", e);
        }

        let res = db.group_info_cache.unwrap();
        println!("{:?}", res);

        assert_eq!(3, res.cache.get_size().unwrap());
        assert_eq!(3, res.cache.get_threshold().unwrap());
        assert_eq!(0, res.cache.get_member("0x1").unwrap().index);
        assert_eq!(1, res.cache.get_member("0x2").unwrap().index);
        assert_eq!(2, res.cache.get_member("0x3").unwrap().index);

        teardown();
    }

    #[test]
    fn test_update_dkg_status() {
        setup();
        let mut db = build_group_info_db_client();
        let mut members: BTreeMap<String, usize> = BTreeMap::new();
        members.insert("0x1".to_string(), 0);
        members.insert("0x2".to_string(), 1);
        members.insert("0x3".to_string(), 2);

        let task = DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 3,
            members,
            assignment_block_height: 100,
            coordinator_address: "0xcoordinator".to_string(),
        };

        if let Err(e) = db.save_task_info(1, task) {
            println!("{:?}", e);
        }

        if let Err(e) = db.update_dkg_status(1, 1, DKGStatus::InPhase) {
            println!("{:?}", e);
        }

        if let Err(e) = db.refresh_current_group_info() {
            println!("{:?}", e);
        }

        let res = db.group_info_cache.unwrap();
        println!("{:?}", res);

        assert_eq!(DKGStatus::InPhase, res.cache.dkg_status);

        teardown();
    }

    #[tokio::test]
    async fn test_save_output() {
        setup();
        let mut db = build_group_info_db_client();
        let mut members: BTreeMap<String, usize> = BTreeMap::new();
        members.insert("0x1".to_string(), 0);
        members.insert("0x2".to_string(), 1);
        members.insert("0x3".to_string(), 2);

        let task = DKGTask {
            group_index: 1,
            epoch: 1,
            size: 3,
            threshold: 3,
            members,
            assignment_block_height: 100,
            coordinator_address: "0xcoordinator".to_string(),
        };

        if let Err(e) = db.save_task_info(0, task) {
            println!("{:?}", e);
        }

        let (t, n) = (3, 3);

        println!("nodes setup... t: {} n: {}", t, n);

        let rng = &mut rand::thread_rng();

        let (mut board, phase0s) = test_helper::setup::<bls12381::Curve, G1Scheme, _>(n, t, rng);

        let mut outputs =
            test_helper::run_dkg::<bls12381::Curve, G1Scheme>(&mut board, phase0s).await;

        let output = outputs.remove(0);

        if let Err(e) = db.save_output(1, 1, output.clone()) {
            println!("{:?}", e);
        }

        if let Err(e) = db.refresh_current_group_info() {
            println!("{:?}", e);
        }

        let res = db.group_info_cache.unwrap();
        println!("{:?}", res);

        assert_eq!(3, res.cache.get_size().unwrap());
        assert_eq!(Some(output.share), res.cache.share);
        assert_eq!(
            output.public.public_key(),
            res.cache.get_public_key().unwrap()
        );
        assert_eq!(
            Some(output.public.eval(0).value),
            res.cache.get_member("0x1").unwrap().partial_public_key
        );
        assert_eq!(
            Some(output.public.eval(1).value),
            res.cache.get_member("0x2").unwrap().partial_public_key
        );
        assert_eq!(
            Some(output.public.eval(2).value),
            res.cache.get_member("0x3").unwrap().partial_public_key
        );

        teardown();
    }

    #[test]
    fn test_add_and_get_randomness_task_with_assigned_group() {
        setup();

        let mut db = build_randomness_tasks_db_client();
        let task = RandomnessTask {
            index: 1,
            message: String::from("test task"),
            group_index: 2,
            assignment_block_height: 100,
        };

        if let Err(e) = db.add(task.clone()) {
            println!("{:?}", e);
        }

        assert_eq!(true, db.contains(1).unwrap());
        assert_eq!(task, db.get(1).unwrap());
        assert_eq!(false, db.is_handled(1).unwrap());

        let available_tasks = db.check_and_get_available_tasks(100, 1).unwrap();
        assert_eq!(0, available_tasks.len());

        let available_tasks = db.check_and_get_available_tasks(100, 2).unwrap();
        assert_eq!(1, available_tasks.len());
        assert_eq!(1, available_tasks[0].index);
        assert_eq!(String::from("test task"), available_tasks[0].message);
        assert_eq!(2, available_tasks[0].group_index);
        assert_eq!(100, available_tasks[0].assignment_block_height);

        assert_eq!(true, db.contains(1).unwrap());
        assert_eq!(task, db.get(1).unwrap());
        assert_eq!(true, db.is_handled(1).unwrap());

        let available_tasks = db.check_and_get_available_tasks(100, 2).unwrap();
        assert_eq!(0, available_tasks.len());

        teardown();
    }

    #[test]
    fn test_add_and_get_randomness_task_over_exclusive_window() {
        setup();

        let mut db = build_randomness_tasks_db_client();
        let task = RandomnessTask {
            index: 1,
            message: String::from("test task"),
            group_index: 2,
            assignment_block_height: 100,
        };

        if let Err(e) = db.add(task.clone()) {
            println!("{:?}", e);
        }

        assert_eq!(true, db.contains(1).unwrap());
        assert_eq!(task, db.get(1).unwrap());
        assert_eq!(false, db.is_handled(1).unwrap());

        let available_tasks = db.check_and_get_available_tasks(130, 1).unwrap();
        assert_eq!(0, available_tasks.len());

        let available_tasks = db.check_and_get_available_tasks(131, 1).unwrap();
        assert_eq!(1, available_tasks.len());
        assert_eq!(1, available_tasks[0].index);
        assert_eq!(String::from("test task"), available_tasks[0].message);
        assert_eq!(2, available_tasks[0].group_index);
        assert_eq!(100, available_tasks[0].assignment_block_height);

        assert_eq!(true, db.contains(1).unwrap());
        assert_eq!(task, db.get(1).unwrap());
        assert_eq!(true, db.is_handled(1).unwrap());

        let available_tasks = db.check_and_get_available_tasks(131, 1).unwrap();
        assert_eq!(0, available_tasks.len());

        teardown();
    }

    #[test]
    fn test_save_node_info() {
        setup();

        let mut db = build_node_info_db_client();

        let id_address = String::from("0x1");
        let node_rpc_endpoint = String::from("127.0.0.1");

        if let Err(e) = db.save_node_info(id_address, node_rpc_endpoint) {
            println!("{:?}", e);
        }

        assert_eq!("0x1", db.get_id_address());
        assert_eq!("127.0.0.1", db.get_node_rpc_endpoint().unwrap());

        teardown();
    }

    #[test]
    fn test_save_node_rpc_endpoint() {
        setup();

        let mut db = build_node_info_db_client();

        let id_address = String::from("0x1");
        let node_rpc_endpoint = String::from("127.0.0.1");

        if let Err(e) = db.save_node_info(id_address, node_rpc_endpoint) {
            println!("{:?}", e);
        }

        let node_rpc_endpoint = String::from("192.168.0.1");

        if let Err(e) = db.set_node_rpc_endpoint(node_rpc_endpoint) {
            println!("{:?}", e);
        }

        assert_eq!("192.168.0.1", db.get_node_rpc_endpoint().unwrap());

        teardown();
    }

    #[test]
    fn test_save_node_dkg_key_pair() {
        setup();

        let mut db = build_node_info_db_client();

        let id_address = String::from("0x1");
        let node_rpc_endpoint = String::from("127.0.0.1");

        if let Err(e) = db.save_node_info(id_address, node_rpc_endpoint) {
            println!("{:?}", e);
        }

        let rng = &mut rand::thread_rng();

        let (private_key, public_key) = G1Scheme::keypair(rng);

        if let Err(e) = db.set_dkg_key_pair(private_key, public_key) {
            println!("{:?}", e);
        }

        assert_eq!(&private_key, db.get_dkg_private_key().unwrap());
        assert_eq!(&public_key, db.get_dkg_public_key().unwrap());

        teardown();
    }
}
