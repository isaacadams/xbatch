mod batch;
mod cli;
mod executor;

#[tokio::main]
async fn main() {
    cli::run().await;
}
