use thiserror::Error;

/// Top-level application error.
#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Confluence(#[from] ConfluenceError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Configuration-related errors with actionable messages.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(
        "Missing required configuration: {name}. Set via CLI flag, environment variable, or .env file"
    )]
    Missing { name: &'static str },

    #[error("Could not determine home directory")]
    NoHomeDir,

    #[error("Failed to read configuration file {path}: {source}")]
    FileRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse JSON in {path}: {source}")]
    JsonParse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

/// Confluence API errors with actionable messages.
#[derive(Debug, Error)]
pub enum ConfluenceError {
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error("Failed to deserialize Confluence API response: {0}")]
    Deserialize(reqwest::Error),

    #[error(
        "Authentication failed. Check CONFLUENCE_USERNAME and CONFLUENCE_API_TOKEN"
    )]
    Unauthorized,

    #[error("Page not found: {0}. Check the page URL")]
    PageNotFound(String),

    #[error(
        "Version conflict on page {page_id} (attempted version {attempted_version}). \
         The page was modified by another user"
    )]
    VersionConflict {
        page_id: String,
        attempted_version: u32,
    },

    #[error(
        "Could not extract page ID from URL: {0}. \
         Expected format: https://domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Page+Title"
    )]
    InvalidPageUrl(String),

    #[error(
        "Failed to upload attachment '{filename}' to page {page_id}: HTTP {status}"
    )]
    AttachmentUpload {
        page_id: String,
        filename: String,
        status: u16,
    },

    #[error("Failed to construct multipart form: {0}")]
    Multipart(String),

    #[error("Unexpected HTTP status: {0}")]
    UnexpectedStatus(u16),
}
