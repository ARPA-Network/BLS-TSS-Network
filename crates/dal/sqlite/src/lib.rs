mod group;
mod node;
mod result;
mod task;
mod test_helper;
mod types;
pub use crate::group::GroupInfoDBClient;
pub use crate::node::NodeInfoDBClient;
pub use crate::result::OPSignatureResultDBClient;
pub use crate::result::SignatureResultDBClient;
pub use crate::task::BLSTasksDBClient;
pub use crate::task::OPBLSTasksDBClient;
pub use crate::types::DBError;
pub use crate::types::DBResult;
pub use crate::types::SqliteDB;
use arpa_core::RandomnessTask;
use arpa_dal::cache::RandomnessResultCache;
use arpa_dal::error::DataAccessError;
use arpa_dal::error::DataAccessResult;
use arpa_dal::BLSTasksHandler;
use arpa_dal::GroupInfoHandler;
use arpa_dal::NodeInfoHandler;
use arpa_dal::SignatureResultCacheHandler;
use ethers_core::utils::hex;
use log::LevelFilter;
use migration::Migrator;
use migration::MigratorTrait;
use migration::SelectStatement;
use migration::UpdateStatement;
use result::BaseSignatureResultDBClient;
use sea_orm::ConnectOptions;
use sea_orm::ConnectionTrait;
use sea_orm::DatabaseBackend;
use sea_orm::FromQueryResult;
use sea_orm::QueryResult;
use sea_orm::Statement;
use std::time::Duration;
use task::BaseBLSTasksDBClient;
use threshold_bls::group::Curve;

pub const OP_MAINNET_CHAIN_ID: usize = 10;
pub const OP_GOERLI_TESTNET_CHAIN_ID: usize = 420;
pub const OP_DEVNET_CHAIN_ID: usize = 901;
pub const BASE_MAINNET_CHAIN_ID: usize = 8453;
pub const BASE_GOERLI_TESTNET_CHAIN_ID: usize = 84531;

impl SqliteDB {
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

        let db = SqliteDB { connection };

        db.integrity_check().await.map_err(|e|
            format!("Node identity is different from the database, please check the (account)cipher key. Original error: {:?}", e.to_string()))?;

        Migrator::up(&db.connection, None).await?;

        Ok(db)
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

    pub fn build_randomness_tasks_cache(
        &self,
        chain_id: usize,
    ) -> DataAccessResult<Box<dyn BLSTasksHandler<RandomnessTask>>> {
        match chain_id {
            0 => Ok(Box::new(self.get_bls_tasks_client::<RandomnessTask>())),
            OP_MAINNET_CHAIN_ID | OP_GOERLI_TESTNET_CHAIN_ID | OP_DEVNET_CHAIN_ID => {
                Ok(Box::new(self.get_op_bls_tasks_client::<RandomnessTask>()))
            }
            BASE_MAINNET_CHAIN_ID | BASE_GOERLI_TESTNET_CHAIN_ID => {
                Ok(Box::new(self.get_base_bls_tasks_client::<RandomnessTask>()))
            }
            _ => Err(DataAccessError::InvalidChainId(chain_id)),
        }
    }

    pub async fn build_randomness_result_cache(
        &self,
        chain_id: usize,
    ) -> DataAccessResult<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>> {
        match chain_id {
            0 => Ok(Box::new(self.get_randomness_result_client().await?)),
            OP_MAINNET_CHAIN_ID | OP_GOERLI_TESTNET_CHAIN_ID | OP_DEVNET_CHAIN_ID => {
                Ok(Box::new(self.get_op_randomness_result_client().await?))
            }
            BASE_MAINNET_CHAIN_ID | BASE_GOERLI_TESTNET_CHAIN_ID => {
                Ok(Box::new(self.get_base_randomness_result_client().await?))
            }
            _ => Err(DataAccessError::InvalidChainId(chain_id)),
        }
    }

    pub(crate) async fn execute_update_statement(
        &self,
        stmt: &UpdateStatement,
    ) -> DataAccessResult<()> {
        let builder = self.connection.get_database_backend();

        self.connection
            .execute(builder.build(stmt))
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;

        Ok(())
    }

    pub(crate) async fn query_all_statement<S: FromQueryResult>(
        &self,
        stmt: &SelectStatement,
    ) -> DataAccessResult<Vec<S>> {
        let builder = self.connection.get_database_backend();

        S::find_by_statement(builder.build(stmt))
            .all(&self.connection)
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e.into()
            })
    }

    pub(crate) async fn query_one_statement<S: FromQueryResult>(
        &self,
        stmt: &SelectStatement,
    ) -> DataAccessResult<Option<S>> {
        let builder = self.connection.get_database_backend();

        S::find_by_statement(builder.build(stmt))
            .one(&self.connection)
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e.into()
            })
    }
}

impl<PC: Curve + 'static> NodeInfoHandler<PC> for NodeInfoDBClient<PC> {}
impl<PC: Curve + 'static> GroupInfoHandler<PC> for GroupInfoDBClient<PC> {}
impl BLSTasksHandler<RandomnessTask> for BLSTasksDBClient<RandomnessTask> {}
impl BLSTasksHandler<RandomnessTask> for OPBLSTasksDBClient<RandomnessTask> {}
impl BLSTasksHandler<RandomnessTask> for BaseBLSTasksDBClient<RandomnessTask> {}
impl SignatureResultCacheHandler<RandomnessResultCache>
    for SignatureResultDBClient<RandomnessResultCache>
{
}
impl SignatureResultCacheHandler<RandomnessResultCache>
    for OPSignatureResultDBClient<RandomnessResultCache>
{
}
impl SignatureResultCacheHandler<RandomnessResultCache>
    for BaseSignatureResultDBClient<RandomnessResultCache>
{
}

#[cfg(test)]
pub mod sqlite_tests {
    use crate::test_helper;
    use crate::SqliteDB;
    use arpa_core::DKGStatus;
    use arpa_core::DKGTask;
    use arpa_core::RandomnessRequestType;
    use arpa_core::RandomnessTask;
    use arpa_core::DEFAULT_RANDOMNESS_TASK_EXCLUSIVE_WINDOW;
    use arpa_core::PLACEHOLDER_ADDRESS;
    use arpa_dal::BLSTasksFetcher;
    use arpa_dal::BLSTasksUpdater;
    use arpa_dal::GroupInfoFetcher;
    use arpa_dal::GroupInfoUpdater;
    use arpa_dal::NodeInfoFetcher;
    use arpa_dal::NodeInfoUpdater;
    use ethers_core::types::Address;
    use ethers_core::types::U256;
    use std::{fs, path::PathBuf};
    use threshold_bls::curve::bn254::G2Curve;
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

    pub async fn build_sqlite_db() -> Result<SqliteDB, Box<dyn std::error::Error>> {
        SqliteDB::build(DB_PATH, CIPHER_KEY.as_bytes()).await
    }

    #[tokio::test]
    async fn test_build_db() {
        setup();

        let db = build_sqlite_db().await;

        assert!(db.is_ok());

        teardown();
    }

    #[tokio::test]
    async fn test_integrity_check() {
        setup();

        let db = build_sqlite_db().await.unwrap();

        let res = db.integrity_check().await.unwrap();

        assert_eq!("ok".to_owned(), res);

        teardown();
    }

    #[tokio::test]
    async fn test_save_node_info() {
        setup();

        let db = build_sqlite_db().await.unwrap();

        let mut db = db.get_node_info_client::<G2Curve>();

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

        let db = build_sqlite_db().await.unwrap();

        let mut db = db.get_node_info_client::<G2Curve>();

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

        let db = build_sqlite_db().await.unwrap();

        let mut db = db.get_node_info_client::<G2Curve>();

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

        let db = build_sqlite_db().await.unwrap();

        let mut db = db.get_group_info_client::<G2Curve>();

        if let Ok(res) = db.refresh_current_group_info().await {
            assert_eq!(res, false);
        } else {
            panic!("should not fail");
        }

        teardown();
    }

    #[tokio::test]
    async fn test_save_grouping_task_info() {
        setup();
        let db = build_sqlite_db().await.unwrap();

        let mut db = db.get_group_info_client::<G2Curve>();
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
        let db = build_sqlite_db().await.unwrap();

        let mut db = db.get_group_info_client::<G2Curve>();
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
        let db = build_sqlite_db().await.unwrap();

        let mut db = db.get_group_info_client::<G2Curve>();
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

        let randomness_task_exclusive_window = 10;

        let db = build_sqlite_db().await.unwrap();

        let mut db = db.get_bls_tasks_client::<RandomnessTask>();

        let request_id = vec![1];

        let seed = U256::from_big_endian(&String::from("test task").into_bytes());

        let task = RandomnessTask {
            request_id: request_id.clone(),
            subscription_id: 0,
            group_index: 2,
            request_type: RandomnessRequestType::Randomness,
            params: vec![],
            requester: PLACEHOLDER_ADDRESS,
            seed,
            request_confirmations: 0,
            callback_gas_limit: 0,
            callback_max_gas_price: 0.into(),
            assignment_block_height: 100,
        };

        if let Err(e) = db.add(task.clone()).await {
            println!("{:?}", e);
        }

        assert_eq!(true, db.contains(&request_id).await.unwrap());
        assert_eq!(task, db.get(&request_id).await.unwrap());
        assert_eq!(false, db.is_handled(&request_id).await.unwrap());

        let available_tasks = db
            .check_and_get_available_tasks(100, 1, randomness_task_exclusive_window)
            .await
            .unwrap();
        assert_eq!(0, available_tasks.len());

        let available_tasks = db
            .check_and_get_available_tasks(100, 2, randomness_task_exclusive_window)
            .await
            .unwrap();
        assert_eq!(1, available_tasks.len());
        assert_eq!(request_id, available_tasks[0].request_id);
        assert_eq!(seed, available_tasks[0].seed);
        assert_eq!(2, available_tasks[0].group_index);
        assert_eq!(100, available_tasks[0].assignment_block_height);

        assert_eq!(true, db.contains(&request_id).await.unwrap());
        assert_eq!(task, db.get(&request_id).await.unwrap());
        assert_eq!(true, db.is_handled(&request_id).await.unwrap());

        let available_tasks = db
            .check_and_get_available_tasks(100, 2, randomness_task_exclusive_window)
            .await
            .unwrap();
        assert_eq!(0, available_tasks.len());

        teardown();
    }

    #[tokio::test]
    async fn test_add_and_get_randomness_task_over_exclusive_window() {
        setup();

        let randomness_task_exclusive_window = 10;

        let db = build_sqlite_db().await.unwrap();

        let mut db = db.get_bls_tasks_client::<RandomnessTask>();

        let request_id = vec![1];

        let seed = U256::from_big_endian(&String::from("test task").into_bytes());

        let task = RandomnessTask {
            request_id: request_id.clone(),
            subscription_id: 0,
            group_index: 2,
            request_type: RandomnessRequestType::Randomness,
            params: vec![],
            requester: PLACEHOLDER_ADDRESS,
            seed,
            request_confirmations: 0,
            callback_gas_limit: 0,
            callback_max_gas_price: 0.into(),
            assignment_block_height: 100,
        };

        if let Err(e) = db.add(task.clone()).await {
            println!("{:?}", e);
        }

        assert_eq!(true, db.contains(&request_id).await.unwrap());
        assert_eq!(task, db.get(&request_id).await.unwrap());
        assert_eq!(false, db.is_handled(&request_id).await.unwrap());

        let available_tasks = db
            .check_and_get_available_tasks(
                100 + DEFAULT_RANDOMNESS_TASK_EXCLUSIVE_WINDOW,
                1,
                randomness_task_exclusive_window,
            )
            .await
            .unwrap();
        assert_eq!(0, available_tasks.len());

        let available_tasks = db
            .check_and_get_available_tasks(
                100 + DEFAULT_RANDOMNESS_TASK_EXCLUSIVE_WINDOW + 1,
                1,
                randomness_task_exclusive_window,
            )
            .await
            .unwrap();
        assert_eq!(1, available_tasks.len());
        assert_eq!(request_id, available_tasks[0].request_id);
        assert_eq!(seed, available_tasks[0].seed);
        assert_eq!(2, available_tasks[0].group_index);
        assert_eq!(100, available_tasks[0].assignment_block_height);

        assert_eq!(true, db.contains(&request_id).await.unwrap());
        assert_eq!(task, db.get(&request_id).await.unwrap());
        assert_eq!(true, db.is_handled(&request_id).await.unwrap());

        let available_tasks = db
            .check_and_get_available_tasks(
                100 + DEFAULT_RANDOMNESS_TASK_EXCLUSIVE_WINDOW + 1,
                1,
                randomness_task_exclusive_window,
            )
            .await
            .unwrap();
        assert_eq!(0, available_tasks.len());

        teardown();
    }
}
