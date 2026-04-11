pub mod renderer;

use async_trait::async_trait;

use crate::error::ConversionError;

/// A single attachment produced during conversion (e.g., rendered diagram SVG).
#[derive(Debug, Clone)]
pub struct Attachment {
    pub filename: String,
    pub content: Vec<u8>,
    pub content_type: String,
}

/// Result of converting Markdown to Confluence storage format.
#[derive(Debug, Clone)]
pub struct ConvertResult {
    pub storage_xml: String,
    pub attachments: Vec<Attachment>,
}

/// Trait for converting Markdown content to Confluence storage XML.
///
/// Follows the same async trait pattern established by `ConfluenceApi` in Phase 1.
#[async_trait]
pub trait Converter: Send + Sync {
    async fn convert(&self, markdown: &str) -> Result<ConvertResult, ConversionError>;
}

#[cfg(test)]
mod tests;
