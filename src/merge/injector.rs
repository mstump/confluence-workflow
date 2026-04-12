use std::collections::HashSet;

use regex::Regex;
use std::sync::LazyLock;

use crate::merge::matcher::{find_matching_section, Section};
use crate::merge::CommentMarker;

/// Regex to find the end of an opening `<p>` or `<p ...>` tag.
static P_OPEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<p(\s[^>]*)?>").expect("p open regex must compile")
});

/// Inject surviving comment markers back into new content XML.
///
/// Strategy (per CONTEXT.md Decision 4):
/// 1. Exact anchor text match: find anchor_text in new content, wrap with ac:inline-comment-marker
/// 2. Fallback: find matching section by heading, inject at start of first `<p>` in that section
/// 3. If no match at all: log warning, drop the marker (do not corrupt XML)
pub fn inject_markers(
    new_content: &str,
    markers: &[CommentMarker],
    old_sections: &[Section],
    // new_sections is no longer used directly: sections are recomputed from the
    // mutating result string each iteration (WR-03 fix). The parameter is kept
    // for API compatibility with callers that already compute and pass it.
    _new_sections: &[Section],
) -> String {
    if markers.is_empty() {
        return new_content.to_string();
    }

    let mut result = new_content.to_string();
    // Track which anchor text strings have been injected to prevent double-injection
    let mut injected_anchors: HashSet<String> = HashSet::new();

    // Process markers — we need to be careful about shifting offsets when we insert.
    // Process in reverse order by the position we'll insert at to avoid offset shifts.
    // However since we're searching by text content (not byte offset), we process
    // forward and track what's been injected.
    for marker in markers {
        let wrapper_open = format!(
            r#"<ac:inline-comment-marker ac:ref="{}">"#,
            marker.ac_ref
        );
        let wrapper_close = "</ac:inline-comment-marker>";

        // Re-extract sections from the current (already-mutated) result so that
        // byte offsets and content strings are always fresh. This prevents WR-03:
        // earlier insertions shift byte positions and invalidate section content
        // substrings derived from the original new_content string.
        let current_sections = crate::merge::matcher::extract_sections(&result);

        // Strategy 1: Exact anchor text match (non-empty anchor text only).
        // Narrow the search to the section that originally contained this marker
        // to prevent WR-01: choosing the wrong occurrence when the same anchor
        // text appears in an earlier, unrelated section.
        if !marker.anchor_text.is_empty() && !injected_anchors.contains(&marker.anchor_text) {
            // Find which old section held this marker, then find the matching
            // new section by heading within the current (mutated) result.
            let old_section = old_sections
                .iter()
                .find(|s| marker.position >= s.start_offset && marker.position < s.end_offset);

            let section_found = if let Some(old_sec) = old_section {
                find_matching_section(&old_sec.heading, &current_sections)
            } else {
                None
            };

            // Determine the byte range to search within: prefer the specific
            // matched section; fall back to the whole result if no section match.
            let (search_start, search_end) = if let Some(sec) = section_found {
                (sec.start_offset, sec.end_offset)
            } else {
                (0, result.len())
            };

            let search_slice = &result[search_start..search_end];
            if let Some(rel_pos) = search_slice.find(&marker.anchor_text) {
                let abs_pos = search_start + rel_pos;
                let end = abs_pos + marker.anchor_text.len();
                let wrapped = format!(
                    "{}{}{}",
                    wrapper_open, marker.anchor_text, wrapper_close
                );
                result = format!("{}{}{}", &result[..abs_pos], wrapped, &result[end..]);
                injected_anchors.insert(marker.anchor_text.clone());
                continue;
            }
        }

        // Strategy 2: Section-start fallback.
        // Find which old section contained this marker.
        let old_section = old_sections
            .iter()
            .find(|s| marker.position >= s.start_offset && marker.position < s.end_offset);

        if let Some(old_sec) = old_section {
            // Find matching new section by heading in the current (mutated) sections.
            if let Some(new_sec) = find_matching_section(&old_sec.heading, &current_sections) {
                let section_slice = &result[new_sec.start_offset..new_sec.end_offset];
                // Find first <p> or <p ...> in that section.
                if let Some(p_match) = P_OPEN_RE.find(section_slice) {
                    let insert_pos = new_sec.start_offset + p_match.end();
                    // For self-closing markers (empty anchor text), insert self-closing element.
                    if marker.anchor_text.is_empty() {
                        let self_closing = format!(
                            r#"<ac:inline-comment-marker ac:ref="{}"/>"#,
                            marker.ac_ref
                        );
                        result.insert_str(insert_pos, &self_closing);
                    } else {
                        // WR-02: anchor text was NOT found in new content (Strategy 1 failed).
                        // Inserting the stale anchor text would corrupt the document.
                        // Use a self-closing marker to preserve the comment thread reference
                        // without injecting text that does not exist in the updated document.
                        let self_closing = format!(
                            r#"<ac:inline-comment-marker ac:ref="{}"/>"#,
                            marker.ac_ref
                        );
                        result.insert_str(insert_pos, &self_closing);
                    }
                    injected_anchors.insert(marker.anchor_text.clone());
                    continue;
                }
            }
        }

        // Strategy 3: Drop with warning
        tracing::warn!(
            ac_ref = %marker.ac_ref,
            "Cannot re-inject comment — no anchor text match or section match in new content"
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_marker(ac_ref: &str, anchor_text: &str, position: usize) -> CommentMarker {
        let full_match = if anchor_text.is_empty() {
            format!(r#"<ac:inline-comment-marker ac:ref="{}"/>"#, ac_ref)
        } else {
            format!(
                r#"<ac:inline-comment-marker ac:ref="{}">{}</ac:inline-comment-marker>"#,
                ac_ref, anchor_text
            )
        };
        CommentMarker {
            full_match,
            ac_ref: ac_ref.to_string(),
            anchor_text: anchor_text.to_string(),
            position,
        }
    }

    fn make_sections(html: &str) -> Vec<Section> {
        crate::merge::matcher::extract_sections(html)
    }

    #[test]
    fn test_inject_exact_anchor_text_match() {
        let new_content = "<h2>Title</h2><p>Some marked text here</p>";
        let old_content = r#"<h2>Title</h2><p>Some <ac:inline-comment-marker ac:ref="abc">marked</ac:inline-comment-marker> text here</p>"#;
        let markers = vec![make_marker("abc", "marked", 19)];
        let old_sections = make_sections(old_content);
        let new_sections = make_sections(new_content);

        let result = inject_markers(new_content, &markers, &old_sections, &new_sections);
        assert!(
            result.contains(r#"<ac:inline-comment-marker ac:ref="abc">marked</ac:inline-comment-marker>"#),
            "Should wrap anchor text with marker. Got: {}",
            result
        );
        // Verify the surrounding text is preserved
        assert!(result.contains("Some "));
        assert!(result.contains(" text here"));
    }

    #[test]
    fn test_inject_multiple_markers() {
        let new_content = "<h2>Title</h2><p>First word and second word here</p>";
        let old_content = r#"<h2>Title</h2><p><ac:inline-comment-marker ac:ref="aaa">First</ac:inline-comment-marker> word and <ac:inline-comment-marker ac:ref="bbb">second</ac:inline-comment-marker> word here</p>"#;
        let markers = vec![
            make_marker("aaa", "First", 19),
            make_marker("bbb", "second", 100),
        ];
        let old_sections = make_sections(old_content);
        let new_sections = make_sections(new_content);

        let result = inject_markers(new_content, &markers, &old_sections, &new_sections);
        assert!(
            result.contains(r#"<ac:inline-comment-marker ac:ref="aaa">First</ac:inline-comment-marker>"#),
            "First marker should be injected. Got: {}",
            result
        );
        assert!(
            result.contains(r#"<ac:inline-comment-marker ac:ref="bbb">second</ac:inline-comment-marker>"#),
            "Second marker should be injected. Got: {}",
            result
        );
    }

    #[test]
    fn test_inject_fallback_to_section_start() {
        // Anchor text "oldword" not found in new content, but section heading matches
        let old_content = "<h2>Title</h2><p>Some oldword text</p>";
        let new_content = "<h2>Title</h2><p>Completely new text</p>";
        let markers = vec![make_marker("abc", "oldword", 19)];
        let old_sections = make_sections(old_content);
        let new_sections = make_sections(new_content);

        let result = inject_markers(new_content, &markers, &old_sections, &new_sections);
        // Anchor text "oldword" is not present in new content, so Strategy-1 fails.
        // Strategy-2 should inject a self-closing marker (preserving the comment thread
        // reference without corrupting the document with stale anchor text).
        assert!(
            result.contains(r#"<ac:inline-comment-marker ac:ref="abc"/>"#),
            "Marker should be injected as self-closing via section fallback. Got: {}",
            result
        );
    }

    #[test]
    fn test_inject_no_match_drops_with_warning() {
        // Old section heading doesn't match any new section
        let old_content = "<h2>Removed</h2><p>Old text with marker</p>";
        let new_content = "<h2>Different</h2><p>New content</p>";
        let markers = vec![make_marker("abc", "marker", 5)];
        let old_sections = make_sections(old_content);
        let new_sections = make_sections(new_content);

        let result = inject_markers(new_content, &markers, &old_sections, &new_sections);
        // Should return new content unchanged (marker dropped)
        assert_eq!(result, new_content);
    }

    #[test]
    fn test_inject_empty_keep_list_returns_unchanged() {
        let new_content = "<h2>Title</h2><p>Content</p>";
        let old_sections = make_sections("<h2>Title</h2><p>Content</p>");
        let new_sections = make_sections(new_content);

        let result = inject_markers(new_content, &[], &old_sections, &new_sections);
        assert_eq!(result, new_content);
    }

    #[test]
    fn test_inject_no_double_injection() {
        // Anchor text "word" appears multiple times in new content
        let new_content = "<h2>Title</h2><p>word and word again</p>";
        let old_content = r#"<h2>Title</h2><p><ac:inline-comment-marker ac:ref="abc">word</ac:inline-comment-marker> and word again</p>"#;
        let markers = vec![make_marker("abc", "word", 19)];
        let old_sections = make_sections(old_content);
        let new_sections = make_sections(new_content);

        let result = inject_markers(new_content, &markers, &old_sections, &new_sections);
        // Count occurrences of the marker
        let count = result.matches(r#"ac:ref="abc""#).count();
        assert_eq!(
            count, 1,
            "Should inject marker only once even though anchor text appears twice. Got: {}",
            result
        );
    }

    #[test]
    fn test_inject_produces_valid_xml_structure() {
        let new_content = "<h2>Title</h2><p>Some text here</p>";
        let old_content = r#"<h2>Title</h2><p>Some <ac:inline-comment-marker ac:ref="abc">text</ac:inline-comment-marker> here</p>"#;
        let markers = vec![make_marker("abc", "text", 19)];
        let old_sections = make_sections(old_content);
        let new_sections = make_sections(new_content);

        let result = inject_markers(new_content, &markers, &old_sections, &new_sections);
        // Verify open and close tags are balanced
        let open_count = result.matches("<ac:inline-comment-marker").count();
        let close_count = result.matches("</ac:inline-comment-marker>").count();
        assert_eq!(
            open_count, close_count,
            "Open and close marker tags should be balanced. Got: {}",
            result
        );
        // Verify the overall structure is preserved
        assert!(result.starts_with("<h2>"));
        assert!(result.ends_with("</p>"));
    }

    #[test]
    fn test_inject_self_closing_marker_uses_section_fallback() {
        // Self-closing markers have empty anchor text, so they can't use exact match
        let old_content = r#"<h2>Title</h2><p>Text <ac:inline-comment-marker ac:ref="abc"/> more</p>"#;
        let new_content = "<h2>Title</h2><p>New paragraph text</p>";
        let markers = vec![make_marker("abc", "", 19)];
        let old_sections = make_sections(old_content);
        let new_sections = make_sections(new_content);

        let result = inject_markers(new_content, &markers, &old_sections, &new_sections);
        assert!(
            result.contains(r#"<ac:inline-comment-marker ac:ref="abc"/>"#),
            "Self-closing marker should be re-injected via section fallback. Got: {}",
            result
        );
    }
}
