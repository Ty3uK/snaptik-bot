use anyhow::Context;
use anyhow::Result;
use tokio_rusqlite::Connection;
use tokio_rusqlite::OptionalExtension;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub async fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path).await?;
        conn.call(|conn| {
            return conn.execute(
                "CREATE TABLE IF NOT EXISTS videos (url TEXT PRIMARY KEY, file_id TEXT NOT NULL)",
                [],
            );
        })
        .await?;
        return Ok(Self { conn });
    }

    pub async fn get_video(&self, url: &str) -> Result<Option<String>> {
        let url = url.to_owned();
        self.conn
            .call(|conn| {
                conn.query_row("SELECT file_id FROM videos WHERE url=?1", [url], |row| {
                    row.get(0)
                })
                .optional()
            })
            .await
            .context("Db:get_video: cannot query row from `videos` table")
    }

    pub async fn insert_video(&self, url: &str, file_id: &str) -> Result<()> {
        let url = url.to_owned();
        let file_id = file_id.to_owned();
        return self
            .conn
            .call(|conn| {
                return conn.execute(
                    "INSERT OR REPLACE INTO videos(url, file_id) VALUES (?1, ?2)",
                    [url, file_id],
                );
            })
            .await
            .map(|_| ())
            .context("Db:insert_video: cannot query `videos` table");
    }
}
