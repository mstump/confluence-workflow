use clap::Parser;
use confluence_agent::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    confluence_agent::run(cli).await
}
