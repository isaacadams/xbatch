use crate::{executor::Executor, result};
use sqlx::{sqlite::SqliteConnectOptions, Pool, Sqlite, SqlitePool};

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

pub async fn show(id: u64) -> Result<(String, Vec<String>), sqlx::Error> {
    let pool = setup().await?;
    let mut txn = pool.begin().await?;

    let header = sqlx::query("SELECT * FROM BATCH WHERE timestamp = ?")
        .bind(id as i64)
        .fetch_one(txn.as_mut())
        .await?;

    let results = sqlx::query("SELECT * FROM RESULT WHERE batch_id = ?")
        .bind(id as i64)
        .fetch_all(txn.as_mut())
        .await?;

    txn.commit().await?;

    let header = result::row_to_string(&header).join(",");

    Ok((header, result::ResultSet::new(results).to_csv_rows()))
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
        [stdout] TEXT,
        [stderr] TEXT,
        [exit_code] INTEGER,
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
    let _ = sqlx::query(
        "INSERT INTO 
        BATCH(timestamp, arguments) 
        VALUES (?,?);",
    )
    .bind(timestamp as i64)
    .bind(arguments)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn new(exe: Executor) -> Option<Batch> {
    let pool = setup().await.ok()?;
    Some(Batch::new(exe, pool))
}

pub struct Batch {
    timestamp: u64,
    exe: Executor,
    pool: Pool<Sqlite>,
}

impl Batch {
    fn new(exe: Executor, pool: Pool<Sqlite>) -> Self {
        let timestamp = unix_now();

        Self {
            timestamp,
            exe,
            pool,
        }
    }

    pub async fn record(&self, arguments: &str) -> Result<(), sqlx::Error> {
        insert_batch_record(self.timestamp, arguments, &self.pool).await
    }

    pub async fn run(&mut self, row: String) -> Result<(), sqlx::Error> {
        match self.exe.run(&row) {
            Ok(output) => {
                self.write(
                    &row,
                    Some(String::from_utf8_lossy(&output.stdout).as_ref()),
                    Some(String::from_utf8_lossy(&output.stderr).as_ref()),
                    output.status.code(),
                    None,
                )
                .await
            }
            Err(err) => {
                self.write(&row, None, None, None, Some(&format!("{}", err)))
                    .await
            }
        }?;

        Ok(())
    }

    pub async fn write(
        &self,
        row: &str,
        stdout: Option<&str>,
        stderr: Option<&str>,
        exit_code: Option<i32>,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let _ = sqlx::query(
            "INSERT INTO 
            RESULT ([batch_id],[input],[stdout],[stderr],[exit_code],[error]) 
            VALUES (?,?,?,?,?,?)",
        )
        .bind(self.timestamp as i64)
        .bind(row)
        .bind(stdout)
        .bind(stderr)
        .bind(exit_code)
        .bind(error)
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
