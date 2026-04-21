use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Output format for command results.
#[derive(Debug, Clone, PartialEq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text output (default)
    Human,
    /// Machine-readable JSON output
    Json,
}

/// Convert and upload Markdown to Confluence.
#[derive(Debug, Parser)]
#[command(name = "confluence-workflow", about = "Convert and upload Markdown to Confluence")]
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

    /// Anthropic API key (for update command's LLM merge)
    #[arg(long, env = "ANTHROPIC_API_KEY")]
    pub anthropic_api_key: Option<String>,

    /// Path to PlantUML executable or JAR
    #[arg(long, env = "PLANTUML_PATH")]
    pub plantuml_path: Option<String>,

    /// Path to mermaid-cli executable (mmdc)
    #[arg(long, env = "MERMAID_PATH")]
    pub mermaid_path: Option<String>,

    /// Enable debug logging
    #[arg(long, short)]
    pub verbose: bool,

    /// Output format (human or json)
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,

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
