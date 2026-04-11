use crate::merge::CommentMarker;
use crate::merge::matcher::Section;

/// Inject surviving comment markers back into new content XML.
///
/// Strategy (per CONTEXT.md Decision 4):
/// 1. Exact anchor text match: find anchor_text in new content, wrap with ac:inline-comment-marker
/// 2. Fallback: find matching section by heading, inject at start of first `<p>` in that section
/// 3. If no match at all: log warning, drop the marker (do not corrupt XML)
pub fn inject_markers(
    new_content: &str,
    _markers: &[CommentMarker],
    _old_sections: &[Section],
    _new_sections: &[Section],
) -> String {
    // Stub — to be implemented in Task 2
    new_content.to_string()
}
