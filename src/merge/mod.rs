pub mod extractor;
pub mod matcher;

/// A comment marker extracted from Confluence storage XML.
#[derive(Debug, Clone, PartialEq)]
pub struct CommentMarker {
    /// The entire XML element (e.g., `<ac:inline-comment-marker ac:ref="uuid">text</ac:inline-comment-marker>`)
    pub full_match: String,
    /// The ac:ref UUID value
    pub ac_ref: String,
    /// Text wrapped by the marker (empty string for self-closing tags)
    pub anchor_text: String,
    /// Byte offset in original content
    pub position: usize,
}

/// Decision for a single comment marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentDecision {
    Keep,
    Drop,
}
