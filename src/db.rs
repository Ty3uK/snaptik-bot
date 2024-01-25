use anyhow::{anyhow, Result};
use serde::Deserialize;
use worker::{console_error, D1Database, Env};

pub struct Db {
    db: D1Database,
}

#[derive(Debug, Deserialize)]
pub struct Video {
    pub url: String,
    pub file_id: String,
}

impl Db {
    pub fn new(env: &Env) -> Option<Self> {
        env.d1("DB").map_or_else(
            |err| {
                console_error!("{err}");
                None
            },
            |db| Some(Self { db }),
        )
    }

    pub async fn get_video(&self, url: &str) -> Result<Option<Video>> {
        self.db
            .prepare("SELECT * FROM videos WHERE url = ?1")
            .bind(&[url.into()])
            .map_err(|err| anyhow!(err.to_string()))?
            .first::<Video>(None)
            .await
            .map_err(|err| anyhow!(err.to_string()))
    }

    pub async fn insert_video(&self, url: &str, file_id: &str) -> Result<bool> {
        self.db
            .prepare("INSERT INTO videos VALUES (?1, ?2)")
            .bind(&[url.into(), file_id.into()])
            .map_err(|err| anyhow!(err.to_string()))?
            .run()
            .await
            .map_err(|err| anyhow!(err.to_string()))
            .map(|it| it.success())
    }
}
