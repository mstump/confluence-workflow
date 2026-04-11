pub mod renderer;

use async_trait::async_trait;

use crate::error::ConversionError;

pub use renderer::{ConfluenceRenderer, DiagramBlock};

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

/// Concrete converter that uses [`ConfluenceRenderer`] to transform Markdown
/// into Confluence storage format XML.
///
/// Diagram blocks are collected but not rendered — Plan 03 adds diagram rendering.
/// Their positions are marked with `<!-- DIAGRAM_PLACEHOLDER_N -->` comments.
pub struct MarkdownConverter;

impl MarkdownConverter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MarkdownConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Converter for MarkdownConverter {
    async fn convert(&self, markdown: &str) -> Result<ConvertResult, ConversionError> {
        let (storage_xml, _diagram_blocks) = renderer::ConfluenceRenderer::render(markdown);
        // Diagram rendering handled in Plan 03; for now return XML with placeholders
        Ok(ConvertResult {
            storage_xml,
            attachments: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests;
