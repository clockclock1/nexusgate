use clap::Parser;
use p2p_edge::{run_edge, EdgeConfig};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "p2p-edge", about = "NexusGate Edge Node")]
struct Args {
    /// Config file path (default: edge.toml beside this executable)
    #[arg(short, long, default_value_t = EdgeConfig::default_path().display().to_string())]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let args = Args::parse();
    let cfg = EdgeConfig::load_or_init(&args.config)?;
    tracing::info!(node_id = %cfg.node_id, server = %cfg.server, "starting p2p-edge");
    run_edge(cfg).await
}
