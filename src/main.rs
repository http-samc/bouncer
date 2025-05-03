use bouncer::start_with_config;
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    config: String,
}

#[tokio::main]
async fn main() {
    // Initialize tracing with DEBUG level
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Parse command line arguments
    let args = Args::parse();

    // Start the server with the config file
    start_with_config(&args.config).await;
}
