use std::path::Path;

use crate::executor::{Executor, ExecutorError};
use sqlx::{
    migrate::MigrateDatabase, sqlite::SqliteRow, Column, ColumnIndex, Decode, Encode, Pool, Row,
    Sqlite, SqlitePool,
};

async fn create_db(path: &str) -> Option<Pool<Sqlite>> {
    ensure_file_exists(path).ok()?;
    let pool = SqlitePool::connect(path).await.ok()?;
    Some(pool)
}

async fn create_batch_table(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS BATCH (
        [timestamp] INTEGER PRIMARY KEY NOT NULL,
        [arguments] TEXT NOT NULL
    );",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn insert_batch_record(
    timestamp: u64,
    arguments: &str,
    pool: &Pool<Sqlite>,
) -> Result<(), sqlx::Error> {
    let _ = sqlx::query("INSERT INTO BATCH(timestamp, arguments) VALUES (?,?);")
        .bind(timestamp as i64)
        .bind(arguments)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn new() -> Option<Batch> {
    let pool = create_db("./batch.db").await?;
    create_batch_table(&pool).await.ok()?;
    Some(Batch::new(pool))
}

pub struct Batch {
    timestamp: u64,
    table: String,
    pool: Pool<Sqlite>,
}

impl Batch {
    fn new(pool: Pool<Sqlite>) -> Self {
        let timestamp = unix_now();
        let table = format!("batch_{}", timestamp);

        Self {
            timestamp,
            table,
            pool,
        }
    }

    pub async fn record(&self, arguments: &str) -> Result<(), sqlx::Error> {
        insert_batch_record(self.timestamp, arguments, &self.pool).await
    }

    pub fn run(&self, mut exe: Executor, rows: Vec<String>) {
        for row in rows {
            match exe.run(&row) {
                Ok(out) => self.write_success(&row, out),
                Err(err) => self.write_error(&row, err),
            }
        }
    }

    pub fn write_error(&self, row: &str, error: ExecutorError) {}
    pub fn write_success(&self, row: &str, output: Vec<u8>) {}
}

fn unix_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Get the current time
    let current_time = SystemTime::now();

    // Calculate the duration since the UNIX epoch
    let duration_since_epoch = current_time
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards!");

    // Get the UNIX timestamp as a u64 value
    let unix_timestamp = duration_since_epoch.as_secs();
    unix_timestamp
}

pub fn ensure_file_exists<P: AsRef<Path>>(path: P) -> Result<(), std::io::Error> {
    let meta = std::fs::metadata(path.as_ref());

    if meta.is_err() || meta.is_ok_and(|f| !f.is_file()) {
        std::fs::File::create(path.as_ref())?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    pub async fn creates_new_db() {
        let _ = std::fs::remove_file("./test.db");
        let pool = create_db("./test.db").await;
        assert!(pool.is_some());
        let _ = std::fs::remove_file("./test.db");
    }
}
