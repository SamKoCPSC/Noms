//! Private PostgreSQL endpoints for tests in both the library and application.
//!
//! pgtemp 0.7.1 releases its chosen TCP port before running initdb. Concurrent
//! starts can reuse it, and its default URI then connects to a different test's
//! cluster. Disable TCP and use the socket inside each cluster's unique directory.
//! Keep the handle alive until every connection to that cluster is finished.

use sqlx::Connection;
use std::time::Duration;

pub struct TestDatabase {
    inner: pgtemp::PgTempDB,
}

impl TestDatabase {
    pub async fn start() -> Self {
        Self::from_builder(pgtemp::PgTempDB::builder()).await
    }

    async fn from_builder(builder: pgtemp::PgTempDBBuilder) -> Self {
        let database = Self {
            inner: builder
                .with_config_param("listen_addresses", "")
                .start_async()
                .await,
        };
        database.wait_until_ready().await;
        database
    }

    async fn wait_until_ready(&self) {
        let uri = self.connection_uri();
        let connection = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                match sqlx::PgConnection::connect(&uri).await {
                    Ok(connection) => break connection,
                    Err(error) => {
                        let starting = match &error {
                            sqlx::Error::Io(cause) => matches!(
                                cause.kind(),
                                std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                            ),
                            sqlx::Error::Database(cause) => cause.code().as_deref() == Some("57P03"),
                            _ => false,
                        };
                        assert!(starting, "temporary PostgreSQL startup failed: {error}");
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            }
        })
        .await
        .expect("temporary PostgreSQL socket did not become ready within 10 seconds");
        connection
            .close()
            .await
            .expect("failed to close the temporary PostgreSQL readiness connection");
    }

    /// The same URI works with SQLx, libpq/psql and local browser fixtures.
    pub fn connection_uri(&self) -> String {
        let directory = self.inner.data_dir();
        let socket = directory.to_str().expect("test socket path must be UTF-8");
        let socket =
            percent_encoding::utf8_percent_encode(socket, percent_encoding::NON_ALPHANUMERIC);
        let mut uri = url::Url::parse(&self.inner.connection_uri()).expect("valid pgtemp URI");
        // libpq URI query values require percent encoding; form encoding would
        // turn a space into '+' and connect to the wrong socket path.
        uri.set_query(Some(&format!("host={socket}")));
        uri.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    async fn identity(pool: &PgPool) -> (String, Option<String>, String) {
        sqlx::query_as(
            "SELECT current_setting('data_directory'), inet_server_addr()::text,
                    current_setting('listen_addresses')",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn reused_port_keeps_ddl_rows_and_shutdown_isolated() {
        let port = pgtemp::PgTempDB::builder().get_port_or_set_random();
        let (first, second) = tokio::join!(
            TestDatabase::from_builder(pgtemp::PgTempDB::builder().with_port(port)),
            TestDatabase::from_builder(pgtemp::PgTempDB::builder().with_port(port)),
        );
        let first_pool = PgPool::connect(&first.connection_uri()).await.unwrap();
        let second_pool = PgPool::connect(&second.connection_uri()).await.unwrap();
        let first_identity = identity(&first_pool).await;
        let second_identity = identity(&second_pool).await;
        assert_ne!(first_identity.0, second_identity.0);
        assert_eq!(first_identity.1, None);
        assert_eq!(second_identity.1, None);
        assert_eq!(first_identity.2, "");
        assert_eq!(second_identity.2, "");
        assert_eq!(
            std::fs::canonicalize(first_identity.0).unwrap(),
            std::fs::canonicalize(first.inner.data_dir()).unwrap()
        );
        assert_eq!(
            std::fs::canonicalize(second_identity.0).unwrap(),
            std::fs::canonicalize(second.inner.data_dir()).unwrap()
        );
        for (pool, value) in [(&first_pool, "first"), (&second_pool, "second")] {
            sqlx::query("CREATE TABLE isolation_probe (value TEXT NOT NULL)")
                .execute(pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO isolation_probe VALUES ($1)")
                .bind(value)
                .execute(pool)
                .await
                .unwrap();
        }
        let value: String = sqlx::query_scalar("SELECT value FROM isolation_probe")
            .fetch_one(&first_pool)
            .await
            .unwrap();
        assert_eq!(value, "first");
        first_pool.close().await;
        drop(first);
        let value: String = sqlx::query_scalar("SELECT value FROM isolation_probe")
            .fetch_one(&second_pool)
            .await
            .unwrap();
        assert_eq!(value, "second");
        second_pool.close().await;
    }

    #[tokio::test]
    async fn socket_uri_with_spaces_round_trips_through_sqlx_and_psql() {
        // macOS TMPDIR is deeply nested; keep this Unix socket path below its length limit.
        let directory =
            std::path::Path::new("/tmp").join(format!("noms socket {}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir(&directory).unwrap();
        let database = TestDatabase::from_builder(
            pgtemp::PgTempDB::builder().with_data_dir_prefix(&directory),
        )
        .await;
        let pool = PgPool::connect(&database.connection_uri()).await.unwrap();
        assert_eq!(identity(&pool).await.1, None);
        let output = std::process::Command::new("psql")
            .arg(database.connection_uri())
            .args([
                "-XAt",
                "--set",
                "ON_ERROR_STOP=1",
                "-c",
                "SELECT inet_server_addr() IS NULL",
            ])
            .output()
            .expect("psql is required for database fixture tests");
        assert!(
            output.status.success(),
            "socket URI failed in psql: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "t");
        pool.close().await;
        drop(database);
        std::fs::remove_dir(directory).unwrap();
    }
}
