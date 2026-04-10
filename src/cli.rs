use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Convert and upload Markdown to Confluence.
#[derive(Debug, Parser)]
#[command(name = "confluence-agent", about = "Convert and upload Markdown to Confluence")]
pub struct Cli {
    /// Confluence base URL
    #[arg(long, env = "CONFLUENCE_URL")]
    pub confluence_url: Option<String>,

    /// Confluence username (email address)
    #[arg(long, env = "CONFLUENCE_USERNAME")]
    pub confluence_username: Option<String>,

    /// Confluence API token
    #[arg(long, env = "CONFLUENCE_API_TOKEN")]
    pub confluence_token: Option<String>,

    /// Enable debug logging
    #[arg(long, short)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Convert markdown and merge with existing page, preserving inline comments
    #[command(about = "Convert markdown and merge with existing page, preserving inline comments")]
    Update {
        /// Path to the Markdown file
        markdown_path: PathBuf,
        /// URL of the Confluence page to update
        page_url: String,
    },

    /// Convert markdown and overwrite page directly (no LLM merge)
    #[command(about = "Convert markdown and overwrite page directly (no LLM merge)")]
    Upload {
        /// Path to the Markdown file
        markdown_path: PathBuf,
        /// URL of the Confluence page to overwrite
        page_url: String,
    },

    /// Convert markdown to Confluence storage XML locally
    #[command(about = "Convert markdown to Confluence storage XML locally")]
    Convert {
        /// Path to the Markdown file
        markdown_path: PathBuf,
        /// Directory to write output files
        output_dir: PathBuf,
    },
}
