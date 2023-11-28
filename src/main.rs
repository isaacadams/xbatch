mod batch;
mod cli;
mod executor;
mod result;

#[tokio::main]
async fn main() {
    cli::run().await;
}
