mod batch;
mod cli;
mod executor;
mod result;

#[tokio::main]
async fn main() {
    env_logger::init();
    cli::run().await;
}
