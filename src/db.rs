use anyhow::Result;
use libsql_client::Statement;
use worker::{console_error, Env};

#[derive(Debug)]
pub struct Db {
    client: libsql_client::Client,
}

impl Db {
    pub fn new(env: &Env) -> Option<Self> {
        libsql_client::Client::from_workers_env(env).map_or_else(
            |e| {
                console_error!("Cannot connect to db: {e}");
                None
            },
            |client| Some(Self { client }),
        )
    }

    pub async fn get_video_file_id(&self, url: &str) -> Result<Option<String>> {
        let result = self
            .client
            .execute(Statement::with_args(
                "SELECT (file_id) FROM videos WHERE url = ? LIMIT 1",
                &[url],
            ))
            .await?;

        if result.rows.is_empty() {
            return Ok(None);
        }

        result.rows[0]
            .try_get::<&str>(0)
            .map(|it| Some(it.to_string()))
    }

    pub async fn insert_video_file_id(&self, url: &str, file_id: &str) -> Result<bool> {
        Ok(self
            .client
            .execute(Statement::with_args(
                "INSERT INTO videos VALUES (?, ?)",
                &[url, file_id],
            ))
            .await?
            .rows_affected
            == 1)
    }
}
