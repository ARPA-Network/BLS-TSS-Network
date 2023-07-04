use entity::{
    group_info,
    node_info::{self, Entity as NodeInfo},
    prelude::{GroupInfo, RandomnessResult, RandomnessTask},
    randomness_result, randomness_task,
};
use sea_orm::{ColumnTrait, DbConn, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub struct NodeQuery;

impl NodeQuery {
    pub async fn find_current_node_info(db: &DbConn) -> Result<Option<node_info::Model>, DbErr> {
        NodeInfo::find()
            .order_by_desc(node_info::Column::Id)
            .one(db)
            .await
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

pub struct RandomnessTaskQuery;

impl RandomnessTaskQuery {
    pub async fn select_by_request_id(
        db: &DbConn,
        request_id: &[u8],
    ) -> Result<Option<randomness_task::Model>, DbErr> {
        RandomnessTask::find()
            .filter(randomness_task::Column::RequestId.eq(request_id))
            .one(db)
            .await
    }
}

pub struct RandomnessResultQuery;

impl RandomnessResultQuery {
    pub async fn select_by_request_id(
        db: &DbConn,
        request_id: &[u8],
    ) -> Result<Option<randomness_result::Model>, DbErr> {
        RandomnessResult::find()
            .filter(randomness_result::Column::RequestId.eq(request_id))
            .one(db)
            .await
    }

    pub async fn select_by_state(
        db: &DbConn,
        state: i32,
    ) -> Result<Vec<randomness_result::Model>, DbErr> {
        RandomnessResult::find()
            .filter(randomness_result::Column::State.eq(state))
            .all(db)
            .await
    }
}
