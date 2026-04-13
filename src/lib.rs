pub mod cli;
pub mod config;
pub mod confluence;
pub mod converter;
pub mod error;
pub mod llm;
pub mod merge;

use std::sync::Arc;

use cli::{Cli, Commands};
use config::{CliOverrides, Config};
use confluence::client::update_page_with_retry;
use confluence::{extract_page_id, ConfluenceApi, ConfluenceClient};
use converter::{Converter, MarkdownConverter};
use error::{AppError, ConfigError};
use llm::AnthropicClient;

/// Result of executing a CLI command, used by the output formatting layer.
#[derive(Debug)]
pub enum CommandResult {
    Update {
        page_url: String,
        comments_kept: usize,
        comments_dropped: usize,
    },
    Upload {
        page_url: String,
    },
    Convert {
        output_dir: String,
        files: Vec<String>,
    },
}

pub async fn run(cli: Cli) -> Result<CommandResult, AppError> {
    match cli.command {
        Commands::Update {
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

            // Validate API key is present for update (requires LLM)
            let api_key = config.anthropic_api_key.clone().ok_or_else(|| {
                AppError::Config(ConfigError::Missing {
                    name: "ANTHROPIC_API_KEY",
                })
            })?;

            // 1. Convert markdown to storage XML
            let markdown = std::fs::read_to_string(&markdown_path).map_err(AppError::Io)?;
            let converter = MarkdownConverter::default();
            let convert_result = converter.convert(&markdown).await?;

            // 2. Build Confluence client and fetch existing page
            let client = ConfluenceClient::new(
                &config.confluence_url,
                &config.confluence_username,
                &config.confluence_api_token,
            );
            let page_id = extract_page_id(&page_url)?;
            let page = client.get_page(&page_id).await?;
            let old_content = &page.body.storage.value;

            // 3. Run comment-preserving merge
            let llm_client = Arc::new(AnthropicClient::new(
                api_key,
                config.anthropic_model.clone(),
            ));
            let merge_result = merge::merge(
                old_content,
                &convert_result.storage_xml,
                llm_client,
                config.anthropic_concurrency,
            )
            .await?;

            tracing::info!(
                kept = merge_result.kept,
                dropped = merge_result.dropped,
                llm_evaluated = merge_result.llm_evaluated,
                "Merge complete"
            );

            // 4. Upload diagram attachments
            for att in &convert_result.attachments {
                client
                    .upload_attachment(
                        &page_id,
                        &att.filename,
                        att.content.clone(),
                        &att.content_type,
                    )
                    .await?;
                tracing::debug!(filename = %att.filename, "Uploaded attachment");
            }

            // 5. Update page content (with retry-on-409)
            update_page_with_retry(&client, &page_id, &merge_result.content, 3).await?;

            tracing::info!(page_id = %page_id, "Page updated successfully");

            Ok(CommandResult::Update {
                page_url,
                comments_kept: merge_result.kept,
                comments_dropped: merge_result.dropped,
            })
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

            // 1. Convert markdown to storage XML
            let markdown = std::fs::read_to_string(&markdown_path).map_err(AppError::Io)?;
            let converter = MarkdownConverter::default();
            let convert_result = converter.convert(&markdown).await?;

            // 2. Build Confluence client
            let client = ConfluenceClient::new(
                &config.confluence_url,
                &config.confluence_username,
                &config.confluence_api_token,
            );
            let page_id = extract_page_id(&page_url)?;

            // 3. Upload diagram attachments
            for att in &convert_result.attachments {
                client
                    .upload_attachment(
                        &page_id,
                        &att.filename,
                        att.content.clone(),
                        &att.content_type,
                    )
                    .await?;
                tracing::debug!(filename = %att.filename, "Uploaded attachment");
            }

            // 4. Update page (with retry-on-409, no LLM)
            update_page_with_retry(&client, &page_id, &convert_result.storage_xml, 3).await?;

            tracing::info!(page_id = %page_id, "Page uploaded successfully");

            Ok(CommandResult::Upload { page_url })
        }
        Commands::Convert {
            markdown_path,
            output_dir,
        } => {
            // No Config::load() needed -- convert does not require Confluence credentials
            let markdown = std::fs::read_to_string(&markdown_path).map_err(AppError::Io)?;
            let converter = MarkdownConverter::default();
            let convert_result = converter.convert(&markdown).await?;

            std::fs::create_dir_all(&output_dir).map_err(AppError::Io)?;

            // Write storage XML
            let xml_path = output_dir.join("page.xml");
            std::fs::write(&xml_path, &convert_result.storage_xml).map_err(AppError::Io)?;
            let mut files = vec![xml_path.to_string_lossy().to_string()];

            // Write SVG attachments
            for att in &convert_result.attachments {
                let att_path = output_dir.join(&att.filename);
                std::fs::write(&att_path, &att.content).map_err(AppError::Io)?;
                files.push(att_path.to_string_lossy().to_string());
                tracing::debug!(filename = %att.filename, "Wrote attachment");
            }

            tracing::info!(
                output_dir = %output_dir.display(),
                file_count = files.len(),
                "Conversion complete"
            );

            Ok(CommandResult::Convert {
                output_dir: output_dir.to_string_lossy().to_string(),
                files,
            })
        }
    }
}
