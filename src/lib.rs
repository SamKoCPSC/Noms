//! Shared vocabulary contracts, precise unit suggestions and seed tooling.
pub mod equipment;
pub mod units;
pub mod vocabulary;

#[cfg(all(feature = "server", test))]
mod test_database;

#[cfg(all(test, feature = "server"))]
mod legacy_database_repro {
    #[tokio::test]
    async fn legacy_tcp_port_reuse_reaches_the_other_cluster() {
        let first = pgtemp::PgTempDB::async_new().await;
        let second = pgtemp::PgTempDB::builder()
            .with_port(first.db_port())
            .start_async()
            .await;
        let pool = sqlx::PgPool::connect(&second.connection_uri()).await.unwrap();
        let connected_directory: String =
            sqlx::query_scalar("SELECT current_setting('data_directory')")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            std::fs::canonicalize(&connected_directory).unwrap(),
            std::fs::canonicalize(first.data_dir()).unwrap()
        );
        assert_ne!(
            std::fs::canonicalize(&connected_directory).unwrap(),
            std::fs::canonicalize(second.data_dir()).unwrap()
        );
        println!("REPRODUCED: the second legacy URI connected to the first cluster.");
        println!("This is the same address confusion that allows unrelated CREATE TABLE operations to collide.");
        pool.close().await;
    }
}
