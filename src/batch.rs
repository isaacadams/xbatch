use crate::executor::{Executor, ExecutorError};
use sqlx::{
    migrate::MigrateDatabase,
    sqlite::{SqliteConnectOptions, SqliteRow},
    Column, ColumnIndex, Decode, Encode, Pool, Row, Sqlite, SqlitePool,
};
use std::path::Path;

async fn create_db(path: &str) -> Result<Pool<Sqlite>, sqlx::Error> {
    SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(path),
    )
    .await
}

async fn setup() -> Result<Pool<Sqlite>, sqlx::Error> {
    let pool = create_db("./batch.db").await?;
    create_batch_table(&pool).await?;
    create_result_table(&pool).await?;
    Ok(pool)
}

pub async fn clear(id: u64) -> Result<(), sqlx::Error> {
    let pool = setup().await?;

    let mut txn = pool.begin().await?;

    sqlx::query("DELETE FROM BATCH WHERE timestamp = ?")
        .bind(id as i64)
        .execute(txn.as_mut())
        .await?;

    sqlx::query("DELETE FROM RESULT WHERE batch_id = ?")
        .bind(id as i64)
        .execute(txn.as_mut())
        .await?;

    txn.commit().await?;

    Ok(())
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

async fn create_result_table(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS RESULT (
        [batch_id] INTEGER NOT NULL,
        [input] TEXT NOT NULL,
        [success] INTEGER,
        [output] TEXT,
        [error] TEXT
    )",
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
    let pool = setup().await.ok()?;
    Some(Batch::new(pool))
}

pub struct Batch {
    timestamp: u64,
    pool: Pool<Sqlite>,
}

impl Batch {
    fn new(pool: Pool<Sqlite>) -> Self {
        let timestamp = unix_now();

        Self { timestamp, pool }
    }

    pub async fn record(&self, arguments: &str) -> Result<(), sqlx::Error> {
        insert_batch_record(self.timestamp, arguments, &self.pool).await
    }

    pub async fn run(&self, mut exe: Executor, rows: Vec<String>) -> Result<(), sqlx::Error> {
        for row in rows {
            match exe.run(&row) {
                Ok(out) => self.write_success(&row, out).await,
                Err(err) => self.write_error(&row, err).await,
            }?;
        }

        Ok(())
    }

    pub async fn write_error(&self, row: &str, error: ExecutorError) -> Result<(), sqlx::Error> {
        let _ = sqlx::query("INSERT INTO RESULT ([batch_id],[input],[success],[output],[error]) VALUES (?,?,0,NULL,?)")
        .bind(self.timestamp as i64)
        .bind(row)
        .bind(error.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn write_success(&self, row: &str, output: Vec<u8>) -> Result<(), sqlx::Error> {
        let _ = sqlx::query(
            "INSERT INTO RESULT ([batch_id],[input],[success],[output],[error]) VALUES (?,?,1,?,NULL)",
        )
        .bind(self.timestamp as i64)
        .bind(row)
        .bind(output)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
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

#[cfg(test)]
mod test {
    use super::*;
}
