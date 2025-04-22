use clap::Parser;
use bouncer::start_with_config;

#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    config: String,
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Args::parse();

    // Start the server with the config file
    start_with_config(&args.config).await;
}
