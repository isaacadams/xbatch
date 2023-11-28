use crate::{batch, executor::Executor, result::ResultSet};
use clap::{Parser, Subcommand};
use sqlx::{migrate::MigrateDatabase, Pool, Sqlite, SqlitePool};
use std::path::PathBuf;

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
    Clear {
        id: u64,
    },
    Show {
        id: u64,
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

                let Some(b) = batch::new().await else {
                    return;
                };

                b.record(&format!("{} {}", &command, &args)).await.unwrap();

                let mut command = std::process::Command::new(command);
                command.arg(args);
                let exe = Executor::new(command);

                b.run(exe, rows).await.unwrap();
            }
            Commands::Fake => {
                let stdin = std::io::stdin();
                for line in stdin.lines() {
                    let line = line.unwrap();
                    let inputs = line.split(",");
                    for value in inputs {
                        if value == "amazon" {
                            panic!("I am scared of amazon.");
                        }
                    }
                    println!("{}", line);
                }
            }
            Commands::Clear { id } => {
                batch::clear(id).await.unwrap();
            }
            Commands::Show { id } => {
                let (header, results) = batch::show(id).await.unwrap();
                println!("\n\nBATCH: {}\n", header);
                for r in results {
                    println!("{:#?}", r);
                }
                println!("\n\n");
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
    let rows = sqlx::query(&format!("SELECT ROWID, * FROM {}", table))
        .bind(table)
        .fetch_all(pool)
        .await?;

    Ok(ResultSet::new(rows))
}
