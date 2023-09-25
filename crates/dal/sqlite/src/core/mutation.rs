use std::collections::BTreeMap;

use arpa_core::{format_now_date, DKGStatus};
use arpa_dal::BLSResultCacheState;
use entity::{
    group_info, node_info, op_randomness_result, op_randomness_task, randomness_result,
    randomness_task,
};
use ethers_core::types::Address;
use sea_orm::{ActiveModelTrait, DbBackend, DbConn, DbErr, FromQueryResult, Set, Statement};

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

pub struct RandomnessTaskMutation;

impl RandomnessTaskMutation {
    #[allow(clippy::too_many_arguments)]
    pub async fn add_task(
        db: &DbConn,
        request_id: Vec<u8>,
        subscription_id: i32,
        group_index: i32,
        request_type: i32,
        params: Vec<u8>,
        requester: String,
        seed: Vec<u8>,
        request_confirmations: i32,
        callback_gas_limit: i32,
        callback_max_gas_price: Vec<u8>,
        assignment_block_height: i32,
    ) -> Result<randomness_task::ActiveModel, DbErr> {
        randomness_task::ActiveModel {
            request_id: Set(request_id),
            subscription_id: Set(subscription_id),
            group_index: Set(group_index),
            request_type: Set(request_type),
            params: Set(params),
            requester: Set(requester),
            seed: Set(seed),
            request_confirmations: Set(request_confirmations),
            callback_gas_limit: Set(callback_gas_limit),
            callback_max_gas_price: Set(callback_max_gas_price),
            assignment_block_height: Set(assignment_block_height),
            create_at: Set(format_now_date()),
            update_at: Set(format_now_date()),
            state: Set(0),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn fetch_available_tasks(
        db: &DbConn,
        group_index: i32,
        assignment_block_height: i32,
    ) -> Result<Vec<randomness_task::Model>, DbErr> {
        randomness_task::Model::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                r#"update randomness_task set state = 1, update_at = $1 where state = 0 and (group_index = $2 or assignment_block_height < $3) 
                returning *"#,
                vec![format_now_date().into(), group_index.into(), assignment_block_height.into()],
            ))
            .all(db).await
    }
}

pub struct RandomnessResultMutation;

impl RandomnessResultMutation {
    pub async fn add(
        db: &DbConn,
        request_id: Vec<u8>,
        group_index: i32,
        message: Vec<u8>,
        threshold: i32,
    ) -> Result<randomness_result::ActiveModel, DbErr> {
        randomness_result::ActiveModel {
            request_id: Set(request_id),
            group_index: Set(group_index),
            message: Set(message),
            threshold: Set(threshold),
            partial_signatures: Set(
                serde_json::to_string(&BTreeMap::<Address, Vec<u8>>::new()).unwrap()
            ),
            committed_times: Set(0),
            create_at: Set(format_now_date()),
            update_at: Set(format_now_date()),
            state: Set(BLSResultCacheState::NotCommitted.to_i32()),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn add_partial_signature(
        db: &DbConn,
        model: randomness_result::Model,
        member_address: Address,
        partial_signature: Vec<u8>,
    ) -> Result<randomness_result::Model, DbErr> {
        let mut partial_signatures: BTreeMap<Address, Vec<u8>> =
            serde_json::from_str(&model.partial_signatures).unwrap();

        partial_signatures.insert(member_address, partial_signature);

        let mut randomness_result: randomness_result::ActiveModel = model.into();

        randomness_result.partial_signatures =
            Set(serde_json::to_string(&partial_signatures).unwrap());

        randomness_result.update_at = Set(format_now_date());

        randomness_result.update(db).await
    }

    pub async fn update_commit_result(
        db: &DbConn,
        model: randomness_result::Model,
        status: i32,
    ) -> Result<randomness_result::Model, DbErr> {
        let mut randomness_result: randomness_result::ActiveModel = model.into();

        randomness_result.state = Set(status);

        randomness_result.update_at = Set(format_now_date());

        randomness_result.update(db).await
    }

    pub async fn incr_committed_times(
        db: &DbConn,
        model: randomness_result::Model,
    ) -> Result<randomness_result::Model, DbErr> {
        let committed_times = model.committed_times + 1;
        let mut randomness_result: randomness_result::ActiveModel = model.into();
        randomness_result.committed_times = Set(committed_times);
        randomness_result.update_at = Set(format_now_date());
        randomness_result.update(db).await
    }
}

pub struct OPRandomnessTaskMutation;

impl OPRandomnessTaskMutation {
    #[allow(clippy::too_many_arguments)]
    pub async fn add_task(
        db: &DbConn,
        request_id: Vec<u8>,
        subscription_id: i32,
        group_index: i32,
        request_type: i32,
        params: Vec<u8>,
        requester: String,
        seed: Vec<u8>,
        request_confirmations: i32,
        callback_gas_limit: i32,
        callback_max_gas_price: Vec<u8>,
        assignment_block_height: i32,
    ) -> Result<op_randomness_task::ActiveModel, DbErr> {
        op_randomness_task::ActiveModel {
            request_id: Set(request_id),
            subscription_id: Set(subscription_id),
            group_index: Set(group_index),
            request_type: Set(request_type),
            params: Set(params),
            requester: Set(requester),
            seed: Set(seed),
            request_confirmations: Set(request_confirmations),
            callback_gas_limit: Set(callback_gas_limit),
            callback_max_gas_price: Set(callback_max_gas_price),
            assignment_block_height: Set(assignment_block_height),
            create_at: Set(format_now_date()),
            update_at: Set(format_now_date()),
            state: Set(0),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn fetch_available_tasks(
        db: &DbConn,
        group_index: i32,
        assignment_block_height: i32,
    ) -> Result<Vec<op_randomness_task::Model>, DbErr> {
        op_randomness_task::Model::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                r#"update op_randomness_task set state = 1, update_at = $1 where state = 0 and (group_index = $2 or assignment_block_height < $3) 
                returning *"#,
                vec![format_now_date().into(), group_index.into(), assignment_block_height.into()],
            ))
            .all(db).await
    }
}

pub struct OPRandomnessResultMutation;

impl OPRandomnessResultMutation {
    pub async fn add(
        db: &DbConn,
        request_id: Vec<u8>,
        group_index: i32,
        message: Vec<u8>,
        threshold: i32,
    ) -> Result<op_randomness_result::ActiveModel, DbErr> {
        op_randomness_result::ActiveModel {
            request_id: Set(request_id),
            group_index: Set(group_index),
            message: Set(message),
            threshold: Set(threshold),
            partial_signatures: Set(
                serde_json::to_string(&BTreeMap::<Address, Vec<u8>>::new()).unwrap()
            ),
            committed_times: Set(0),
            create_at: Set(format_now_date()),
            update_at: Set(format_now_date()),
            state: Set(BLSResultCacheState::NotCommitted.to_i32()),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn add_partial_signature(
        db: &DbConn,
        model: op_randomness_result::Model,
        member_address: Address,
        partial_signature: Vec<u8>,
    ) -> Result<op_randomness_result::Model, DbErr> {
        let mut partial_signatures: BTreeMap<Address, Vec<u8>> =
            serde_json::from_str(&model.partial_signatures).unwrap();

        partial_signatures.insert(member_address, partial_signature);

        let mut randomness_result: op_randomness_result::ActiveModel = model.into();

        randomness_result.partial_signatures =
            Set(serde_json::to_string(&partial_signatures).unwrap());

        randomness_result.update_at = Set(format_now_date());

        randomness_result.update(db).await
    }

    pub async fn update_commit_result(
        db: &DbConn,
        model: op_randomness_result::Model,
        status: i32,
    ) -> Result<op_randomness_result::Model, DbErr> {
        let mut randomness_result: op_randomness_result::ActiveModel = model.into();

        randomness_result.state = Set(status);

        randomness_result.update_at = Set(format_now_date());

        randomness_result.update(db).await
    }

    pub async fn incr_committed_times(
        db: &DbConn,
        model: op_randomness_result::Model,
    ) -> Result<op_randomness_result::Model, DbErr> {
        let committed_times = model.committed_times + 1;
        let mut randomness_result: op_randomness_result::ActiveModel = model.into();
        randomness_result.committed_times = Set(committed_times);
        randomness_result.update_at = Set(format_now_date());
        randomness_result.update(db).await
    }
}
