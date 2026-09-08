use clap::Parser;
use p2p_edge::{run_edge, EdgeConfig};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "p2p-edge", about = "P2P Network Edge Node")]
struct Args {
    #[arg(short, long, default_value = "client/config/edge.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let args = Args::parse();
    let cfg = EdgeConfig::load(&args.config)?;
    tracing::info!(node_id = %cfg.node_id, server = %cfg.server, "starting p2p-edge");
    run_edge(cfg).await
}
