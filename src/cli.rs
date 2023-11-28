use clap::{Parser, Subcommand};
use sqlx::{
    migrate::MigrateDatabase, sqlite::SqliteRow, Column, ColumnIndex, Decode, Encode, Pool, Row,
    Sqlite, SqlitePool,
};
use std::{
    io::{Stdout, Write},
    os::windows::io::AsHandle,
    path::PathBuf,
};

use crate::{batch, executor::Executor};

pub async fn run() {
    Cli::parse().run().await;
}

#[derive(Debug, Parser)]
#[command(name = "iter")]
#[command(about = "", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Run {
        /// Path to sqlite file
        #[arg(short, long)]
        db: PathBuf,

        /// table name to iterate through
        #[arg(short, long)]
        table: String,

        command: String,

        args: String,
    },
    Fake,
}

impl Cli {
    pub async fn run(self) {
        let command = self.command;

        match command {
            Commands::Run {
                db,
                table,
                command,
                args,
            } => {
                let db = connect(db.to_str().unwrap()).await;
                let result = select_all(&table, &db).await.unwrap();

                let rows: Vec<String> = result.to_csv_rows();

                dbg!(&rows);
                dbg!(&command);

                let Some(b) = batch::new().await else {
                    return;
                };

                b.record(&format!("{} {}", &command, &args)).await.unwrap();

                let mut command = std::process::Command::new(command);
                command.arg(args);
                let exe = Executor::new(command);

                b.run(exe, rows);
            }
            Commands::Fake => {
                let stdin = std::io::stdin();
                for line in stdin.lines() {
                    dbg!(line);
                }
            }
        };
    }
}

pub async fn connect<P: AsRef<str>>(path: P) -> Pool<Sqlite> {
    let path = path.as_ref();
    let exists = Sqlite::database_exists(path).await.ok();
    assert!(exists == Some(true), "specified database does not exist");

    let Ok(pool) = SqlitePool::connect(path).await else {
        panic!("failed to establish a connection with specified database");
    };

    pool
}

pub async fn select_all(table: &str, pool: &Pool<Sqlite>) -> Result<ResultSet, sqlx::Error> {
    let rows = sqlx::query(&format!("SELECT * FROM {}", table))
        .bind(table)
        .fetch_all(pool)
        .await?;

    Ok(ResultSet { rows })
}

pub struct ResultSet {
    rows: Vec<SqliteRow>,
}

impl ResultSet {
    pub fn print(&self) {
        self.rows.iter().for_each(|r| {
            for c in r.columns() {
                let value = r.get::<&str, usize>(c.ordinal());
                dbg!(value);
            }
        });
    }

    pub fn to_csv(self) {
        let mut csv = String::new();

        self.rows.iter().for_each(|r| {
            let values: Vec<&str> = r
                .columns()
                .iter()
                .map(|c| r.get::<&str, usize>(c.ordinal()))
                .collect();

            let row = values.join(",");
            csv.push_str(&row);
            csv.push('\n');
        });

        println!("{}", csv);
    }

    pub fn to_csv_rows(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|r| {
                let values: Vec<&str> = r
                    .columns()
                    .iter()
                    .map(|c| r.get::<&str, usize>(c.ordinal()))
                    .collect();

                values.join(",")
            })
            .collect()
    }
}