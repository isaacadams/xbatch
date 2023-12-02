use crate::{
    batch,
    executor::Executor,
    result::{self, ResultSet},
};
use clap::{Parser, Subcommand};
use sqlx::{migrate::MigrateDatabase, Pool, Sqlite, SqlitePool};
use std::path::PathBuf;

/**
 - add command for generating a unique id column
 - add command for validating that a column is unique
*/
pub async fn run() {
    Cli::parse().run().await;
}

#[derive(Debug, Parser)]
#[command(name = "xbatch")]
#[command(about = "monitors the stdout and stderr of your script for each run", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    New {
        program: String,
        args: String,
    },
    Monitor {
        id: u64,
        program: String,
        args: String,
    },
    Stream {
        /// Path to sqlite file
        #[arg(short, long)]
        db: PathBuf,

        /// table name to iterate through
        #[arg(short, long)]
        table: String,
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
            Commands::New { program, args } => {
                let mut command = std::process::Command::new(&program);
                let arguments: &Vec<&str> = &args.split(" ").collect();
                command.args(arguments);
                let exe = Executor::new(command);

                let Some(b) = batch::new(None, exe).await else {
                    return;
                };

                b.record(&format!("{} {}", &program, &args)).await.unwrap();

                println!("{}", b.timestamp());
            }
            Commands::Monitor { id, program, args } => {
                let mut command = std::process::Command::new(&program);
                let arguments: &Vec<&str> = &args.split(" ").collect();
                command.args(arguments);

                log::info!("running {:?}", command);

                let exe = Executor::new(command);

                let Some(mut b) = batch::new(Some(id), exe).await else {
                    return;
                };

                for line in std::io::stdin().lines() {
                    let line = line.unwrap();
                    b.run(line).await.unwrap();
                }
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
            Commands::Stream { db, table } => {
                let db = connect(db.to_str().unwrap()).await;
                stream_rows(&table, &db).await.unwrap();
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

#[allow(dead_code)]
pub async fn select_all(table: &str, pool: &Pool<Sqlite>) -> Result<ResultSet, sqlx::Error> {
    let rows = sqlx::query(&format!("SELECT ROWID, * FROM {}", table))
        .bind(table)
        .fetch_all(pool)
        .await?;

    Ok(ResultSet::new(rows))
}

pub async fn stream_rows(table: &str, pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    use tokio_stream::StreamExt;

    let query = format!("SELECT ROWID, * FROM {}", table);
    let mut cursor = sqlx::query(&query).bind(table).fetch_many(pool);

    while let Some(row) = cursor.try_next().await? {
        match row {
            sqlx::Either::Left(_) => (),
            sqlx::Either::Right(y) => {
                let row = result::row_to_string(&y);
                println!("{}", row.join(","));
            }
        }
    }

    Ok(())
}
