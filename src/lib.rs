pub mod cli;
pub mod config;
pub mod confluence;
pub mod error;

use cli::{Cli, Commands};

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Update { .. } => {
            println!("update command: not yet implemented");
        }
        Commands::Upload { .. } => {
            println!("upload command: not yet implemented");
        }
        Commands::Convert { .. } => {
            println!("convert command: not yet implemented");
        }
    }
    Ok(())
}
