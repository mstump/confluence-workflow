pub mod diagrams;
pub mod renderer;

use async_trait::async_trait;

use crate::config::DiagramConfig;
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
/// into Confluence storage format XML, rendering diagram blocks to SVG
/// attachments via async subprocesses.
pub struct MarkdownConverter {
    diagram_config: DiagramConfig,
}

impl MarkdownConverter {
    pub fn new(diagram_config: DiagramConfig) -> Self {
        Self { diagram_config }
    }
}

impl Default for MarkdownConverter {
    fn default() -> Self {
        Self::new(DiagramConfig::default())
    }
}

#[async_trait]
impl Converter for MarkdownConverter {
    async fn convert(&self, markdown: &str) -> Result<ConvertResult, ConversionError> {
        let (mut storage_xml, diagram_blocks) = renderer::ConfluenceRenderer::render(markdown);
        let mut attachments = Vec::new();

        for (i, block) in diagram_blocks.iter().enumerate() {
            let filename = format!("diagram_{i}.svg");
            let svg_bytes = match block.kind.as_str() {
                "plantuml" | "puml" => {
                    diagrams::render_plantuml(&block.content, &self.diagram_config).await?
                }
                "mermaid" => {
                    diagrams::render_mermaid(&block.content, &self.diagram_config).await?
                }
                other => {
                    return Err(ConversionError::DiagramError {
                        diagram_type: other.to_string(),
                        message: format!("Unknown diagram type: {other}"),
                    });
                }
            };

            // Replace placeholder with ac:image reference
            let placeholder = format!("<!-- DIAGRAM_PLACEHOLDER_{i} -->");
            let image_xml = format!(
                r#"<ac:image ac:alt="{kind} diagram" ac:width="100%"><ri:attachment ri:filename="{filename}" /></ac:image>"#,
                kind = block.kind,
                filename = filename,
            );
            storage_xml = storage_xml.replace(&placeholder, &image_xml);

            attachments.push(Attachment {
                filename,
                content: svg_bytes,
                content_type: "image/svg+xml".to_string(),
            });
        }

        Ok(ConvertResult {
            storage_xml,
            attachments,
        })
    }
}

#[cfg(test)]
mod tests;
