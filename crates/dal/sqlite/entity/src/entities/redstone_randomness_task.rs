//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.3

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "redstone_randomness_task")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i32,
    #[sea_orm(column_type = "Binary(BlobSize::Blob(None))")]
    pub request_id: Vec<u8>,
    pub subscription_id: i32,
    pub group_index: i32,
    pub request_type: i32,
    #[sea_orm(column_type = "Binary(BlobSize::Blob(None))")]
    pub params: Vec<u8>,
    pub requester: String,
    #[sea_orm(column_type = "Binary(BlobSize::Blob(None))")]
    pub seed: Vec<u8>,
    pub request_confirmations: i32,
    pub callback_gas_limit: i32,
    #[sea_orm(column_type = "Binary(BlobSize::Blob(None))")]
    pub callback_max_gas_price: Vec<u8>,
    pub assignment_block_height: i32,
    pub state: i32,
    pub create_at: String,
    pub update_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
