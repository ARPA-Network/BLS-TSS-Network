use std::time::Duration;

use migration::{sea_orm::ConnectOptions, Migrator, MigratorTrait};
use sea_orm_migration::prelude::*;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // cli::run_cli(migration::Migrator).await;
    let database_url = "test_cipher.sqlite";

    let mut opt = ConnectOptions::new(format!("sqlite://{}?mode=rwc", database_url));
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true);

    let connection = sea_orm::Database::connect(opt).await?;
    Migrator::up(&connection, None).await?;

    Ok(())
}
