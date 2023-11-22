use crate::types::DBError;
use crate::types::DBResult;
use crate::types::SqliteDB;
use arpa_core::Group;
use arpa_core::Member;
use arpa_core::{format_now_date, DKGStatus};
use arpa_dal::cache::InMemoryGroupInfoCache;
use arpa_dal::error::DataAccessResult;
use arpa_dal::error::GroupError;
use arpa_dal::ContextInfoUpdater;
use arpa_dal::{DKGOutput, GroupInfoFetcher, GroupInfoUpdater};
use async_trait::async_trait;
use entity::group_info;
use entity::prelude::GroupInfo;
use ethers_core::types::Address;
use sea_orm::{ActiveModelTrait, DbConn, DbErr, EntityTrait, QueryOrder, Set};
use std::collections::BTreeMap;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::Curve;
use threshold_bls::group::Element;
use threshold_bls::sig::Share;

#[derive(Clone)]
pub struct GroupInfoDBClient<C: Curve> {
    pub(crate) db_client: Arc<SqliteDB>,
    pub(crate) group_info_cache_model: Option<group_info::Model>,
    pub(crate) group_info_cache: Option<InMemoryGroupInfoCache<C>>,
}

impl SqliteDB {
    pub fn get_group_info_client<C: Curve>(&self) -> GroupInfoDBClient<C> {
        GroupInfoDBClient {
            db_client: Arc::new(self.clone()),
            group_info_cache: None,
            group_info_cache_model: None,
        }
    }
}

impl<C: Curve> std::fmt::Debug for GroupInfoDBClient<C> {
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

impl<C: Curve> GroupInfoDBClient<C> {
    pub async fn refresh_current_group_info(&mut self) -> DBResult<bool> {
        let conn = &self.db_client.connection;

        match GroupQuery::find_current_group_info(conn).await? {
            Some(group_info) => {
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

                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn get_group_info_cache(&self) -> DataAccessResult<&InMemoryGroupInfoCache<C>> {
        self.group_info_cache
            .as_ref()
            .ok_or_else(|| GroupError::NoGroupTask.into())
    }

    pub fn get_connection(&self) -> &DbConn {
        &self.db_client.connection
    }
}

impl<C: Curve> ContextInfoUpdater for GroupInfoDBClient<C> {
    fn refresh_context_entry(&self) {
        if let Some(cache) = &self.group_info_cache {
            cache.refresh_context_entry();
        }
    }
}

impl<C: Curve> GroupInfoFetcher<C> for GroupInfoDBClient<C> {
    fn get_group(&self) -> DataAccessResult<&Group<C>> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_group()
    }

    fn get_index(&self) -> DataAccessResult<usize> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_index()
    }

    fn get_epoch(&self) -> DataAccessResult<usize> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_epoch()
    }

    fn get_size(&self) -> DataAccessResult<usize> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_size()
    }

    fn get_threshold(&self) -> DataAccessResult<usize> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_threshold()
    }

    fn get_state(&self) -> DataAccessResult<bool> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_state()
    }

    fn get_self_index(&self) -> DataAccessResult<usize> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_self_index()
    }

    fn get_public_key(&self) -> DataAccessResult<&C::Point> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_public_key()
    }

    fn get_secret_share(&self) -> DataAccessResult<&Share<C::Scalar>> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_secret_share()
    }

    fn get_members(&self) -> DataAccessResult<&BTreeMap<Address, Member<C>>> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_members()
    }

    fn get_member(&self, id_address: Address) -> DataAccessResult<&arpa_core::Member<C>> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_member(id_address)
    }

    fn get_committers(&self) -> DataAccessResult<Vec<Address>> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_committers()
    }

    fn get_dkg_start_block_height(&self) -> DataAccessResult<usize> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_dkg_start_block_height()
    }

    fn get_dkg_status(&self) -> DataAccessResult<arpa_core::DKGStatus> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.get_dkg_status()
    }

    fn is_committer(&self, id_address: Address) -> DataAccessResult<bool> {
        let group_info_cache = self.get_group_info_cache()?;

        group_info_cache.is_committer(id_address)
    }
}

#[async_trait]
impl<C: Curve + Sync + Send> GroupInfoUpdater<C> for GroupInfoDBClient<C> {
    async fn save_task_info(
        &mut self,
        self_index: usize,
        task: arpa_core::DKGTask,
    ) -> DataAccessResult<()> {
        let members: BTreeMap<Address, Member<C>> = task
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

    async fn save_output(
        &mut self,
        index: usize,
        epoch: usize,
        output: DKGOutput<C>,
    ) -> DataAccessResult<(C::Point, C::Point, Vec<Address>)> {
        let group_info_cache = self.get_group_info_cache()?;

        let self_index = group_info_cache.get_self_index()?;

        let mut group = group_info_cache.get_group()?.clone();

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

        let public_key = output.public.public_key().clone();

        let mut partial_public_key = C::Point::new();

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
        dkg_status: arpa_core::DKGStatus,
    ) -> DataAccessResult<bool> {
        let group_info_cache = self.get_group_info_cache()?;

        let current_dkg_status = group_info_cache.get_dkg_status()?;

        let group = group_info_cache.get_group()?;

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
        let group_info_cache = self.get_group_info_cache()?;

        let group = group_info_cache.get_group()?;

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

pub struct GroupQuery;

impl GroupQuery {
    pub async fn find_current_group_info(db: &DbConn) -> Result<Option<group_info::Model>, DbErr> {
        GroupInfo::find()
            .order_by_desc(group_info::Column::Id)
            .one(db)
            .await
    }
}

pub struct GroupMutation;

impl GroupMutation {
    #[allow(clippy::too_many_arguments)]
    pub async fn save_task_info(
        db: &DbConn,
        index: i32,
        epoch: i32,
        size: i32,
        threshold: i32,
        self_index: i32,
        dkg_start_block_height: i32,
        members: String,
    ) -> Result<group_info::ActiveModel, DbErr> {
        group_info::ActiveModel {
            index: Set(index),
            epoch: Set(epoch),
            size: Set(size),
            threshold: Set(threshold),
            state: Set(0),
            members: Set(members),
            dkg_status: Set(DKGStatus::None.to_usize() as i32),
            self_member_index: Set(self_index),
            dkg_start_block_height: Set(dkg_start_block_height),
            create_at: Set(format_now_date()),
            update_at: Set(format_now_date()),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn update_dkg_status(
        db: &DbConn,
        model: group_info::Model,
        dkg_status: i32,
    ) -> Result<group_info::Model, DbErr> {
        let mut group_info: group_info::ActiveModel = model.into();

        group_info.dkg_status = Set(dkg_status);

        group_info.update_at = Set(format_now_date());

        group_info.update(db).await
    }

    pub async fn update_dkg_output(
        db: &DbConn,
        model: group_info::Model,
        size: i32,
        public_key: Vec<u8>,
        share: Vec<u8>,
        members: String,
    ) -> Result<group_info::Model, DbErr> {
        let mut group_info: group_info::ActiveModel = model.into();

        group_info.size = Set(size);
        group_info.public_key = Set(Some(public_key));
        group_info.share = Set(Some(share));
        group_info.members = Set(members);

        group_info.update_at = Set(format_now_date());

        group_info.update(db).await
    }

    pub async fn update_committers(
        db: &DbConn,
        model: group_info::Model,
        committers: String,
    ) -> Result<group_info::Model, DbErr> {
        let mut group_info: group_info::ActiveModel = model.into();

        group_info.state = Set(1);
        group_info.committers = Set(Some(committers));

        group_info.update_at = Set(format_now_date());

        group_info.update(db).await
    }
}
