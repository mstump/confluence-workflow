pub mod cli;
pub mod config;
pub mod confluence;
pub mod converter;
pub mod error;
pub mod llm;
pub mod merge;

use cli::{Cli, Commands};
use config::{CliOverrides, Config};
use confluence::{extract_page_id, ConfluenceClient};
use confluence::client::update_page_with_retry;
use error::AppError;

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Update { .. } => {
            println!("update command: not yet implemented");
        }
        Commands::Upload {
            markdown_path,
            page_url,
        } => {
            let overrides = CliOverrides {
                confluence_url: cli.confluence_url,
                confluence_username: cli.confluence_username,
                confluence_api_token: cli.confluence_token,
                anthropic_api_key: None,
            };
            let config = Config::load(&overrides)?;
            let client = ConfluenceClient::new(
                &config.confluence_url,
                &config.confluence_username,
                &config.confluence_api_token,
            );
            let page_id = extract_page_id(&page_url)?;
            let markdown = std::fs::read_to_string(&markdown_path)
                .map_err(AppError::Io)?;
            // Phase 1: upload raw markdown content as storage XML placeholder
            // Converter is built in Phase 2 — for now upload the markdown text directly
            update_page_with_retry(&client, &page_id, &markdown, 3).await?;
            tracing::info!(
                "Successfully uploaded {} to page {}",
                markdown_path.display(),
                page_id
            );
            println!("Uploaded {} to {}", markdown_path.display(), page_url);
        }
        Commands::Convert { .. } => {
            println!("convert command: not yet implemented");
        }
    }
    Ok(())
}
