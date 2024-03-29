pub use sea_orm_migration::prelude::*;

mod m20220920_000001_create_node_info_table;
mod m20220920_000002_create_group_info_table;
mod m20220920_000003_create_randomness_task_table;
mod m20220920_000004_create_randomness_task_index;
mod m20230612_000005_create_randomness_result_table;
mod m20230612_000006_create_randomness_result_index;
mod m20230815_000007_create_op_randomness_task_table;
mod m20230815_000008_create_op_randomness_task_index;
mod m20230815_000009_create_op_randomness_result_table;
mod m20230815_000010_create_op_randomness_result_index;
mod m20230911_000011_update_randomness_result_table;
mod m20230911_000012_update_op_randomness_result_table;
mod m20231115_000013_create_base_randomness_task_table;
mod m20231115_000014_create_base_randomness_task_index;
mod m20231115_000015_create_base_randomness_result_table;
mod m20231115_000016_create_base_randomness_result_index;
mod m20240129_000017_create_redstone_randomness_task_table;
mod m20240129_000018_create_redstone_randomness_task_index;
mod m20240129_000019_create_redstone_randomness_result_table;
mod m20240129_000020_create_redstone_randomness_result_index;
mod m20240312_000021_create_loot_randomness_task_table;
mod m20240312_000022_create_loot_randomness_task_index;
mod m20240312_000023_create_loot_randomness_result_table;
mod m20240312_000024_create_loot_randomness_result_index;
mod m20240318_000025_create_taiko_randomness_task_table;
mod m20240318_000026_create_taiko_randomness_task_index;
mod m20240318_000027_create_taiko_randomness_result_table;
mod m20240318_000028_create_taiko_randomness_result_index;

pub use m20220920_000001_create_node_info_table::NodeInfo;
pub use m20220920_000002_create_group_info_table::GroupInfo;
pub use m20220920_000003_create_randomness_task_table::RandomnessTask;
pub use m20230612_000005_create_randomness_result_table::RandomnessResult;
pub use m20230815_000007_create_op_randomness_task_table::OPRandomnessTask;
pub use m20230815_000009_create_op_randomness_result_table::OPRandomnessResult;
pub use m20230911_000011_update_randomness_result_table::RandomnessResultNewColumn;
pub use m20230911_000012_update_op_randomness_result_table::OPRandomnessResultNewColumn;
pub use m20231115_000013_create_base_randomness_task_table::BaseRandomnessTask;
pub use m20231115_000015_create_base_randomness_result_table::BaseRandomnessResult;
pub use m20240129_000017_create_redstone_randomness_task_table::RedstoneRandomnessTask;
pub use m20240129_000019_create_redstone_randomness_result_table::RedstoneRandomnessResult;
pub use m20240312_000021_create_loot_randomness_task_table::LootRandomnessTask;
pub use m20240312_000023_create_loot_randomness_result_table::LootRandomnessResult;
pub use m20240318_000025_create_taiko_randomness_task_table::TaikoRandomnessTask;
pub use m20240318_000027_create_taiko_randomness_result_table::TaikoRandomnessResult;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220920_000001_create_node_info_table::Migration),
            Box::new(m20220920_000002_create_group_info_table::Migration),
            Box::new(m20220920_000003_create_randomness_task_table::Migration),
            Box::new(m20220920_000004_create_randomness_task_index::Migration),
            Box::new(m20230612_000005_create_randomness_result_table::Migration),
            Box::new(m20230612_000006_create_randomness_result_index::Migration),
            Box::new(m20230815_000007_create_op_randomness_task_table::Migration),
            Box::new(m20230815_000008_create_op_randomness_task_index::Migration),
            Box::new(m20230815_000009_create_op_randomness_result_table::Migration),
            Box::new(m20230815_000010_create_op_randomness_result_index::Migration),
            Box::new(m20230911_000011_update_randomness_result_table::Migration),
            Box::new(m20230911_000012_update_op_randomness_result_table::Migration),
            Box::new(m20231115_000013_create_base_randomness_task_table::Migration),
            Box::new(m20231115_000014_create_base_randomness_task_index::Migration),
            Box::new(m20231115_000015_create_base_randomness_result_table::Migration),
            Box::new(m20231115_000016_create_base_randomness_result_index::Migration),
            Box::new(m20240129_000017_create_redstone_randomness_task_table::Migration),
            Box::new(m20240129_000018_create_redstone_randomness_task_index::Migration),
            Box::new(m20240129_000019_create_redstone_randomness_result_table::Migration),
            Box::new(m20240129_000020_create_redstone_randomness_result_index::Migration),
            Box::new(m20240312_000021_create_loot_randomness_task_table::Migration),
            Box::new(m20240312_000022_create_loot_randomness_task_index::Migration),
            Box::new(m20240312_000023_create_loot_randomness_result_table::Migration),
            Box::new(m20240312_000024_create_loot_randomness_result_index::Migration),
            Box::new(m20240318_000025_create_taiko_randomness_task_table::Migration),
            Box::new(m20240318_000026_create_taiko_randomness_task_index::Migration),
            Box::new(m20240318_000027_create_taiko_randomness_result_table::Migration),
            Box::new(m20240318_000028_create_taiko_randomness_result_index::Migration),
        ]
    }
}

impl Migrator {}
