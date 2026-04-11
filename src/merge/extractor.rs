use regex::Regex;
use std::sync::LazyLock;

use super::CommentMarker;

/// Combined regex matching both paired and self-closing inline comment markers.
static MARKER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<ac:inline-comment-marker\b[^>]*?/>|<ac:inline-comment-marker\b[^>]*?>.*?</ac:inline-comment-marker>")
        .expect("marker regex must compile")
});

/// Regex to extract the ac:ref UUID attribute value.
static AC_REF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"ac:ref="([^"]+)""#).expect("ac:ref regex must compile")
});

/// Regex to extract anchor text from paired (non-self-closing) markers.
static ANCHOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<ac:inline-comment-marker\b[^>]*?>(.*?)</ac:inline-comment-marker>")
        .expect("anchor regex must compile")
});

/// Extract all inline comment markers from Confluence storage XML.
///
/// Returns markers in document order with byte offsets, ac:ref UUIDs,
/// and anchor text (empty string for self-closing markers).
pub fn extract_markers(content: &str) -> Vec<CommentMarker> {
    MARKER_RE
        .find_iter(content)
        .map(|m| {
            let full_match = m.as_str().to_string();
            let position = m.start();

            let ac_ref = AC_REF_RE
                .captures(&full_match)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            let anchor_text = ANCHOR_RE
                .captures(&full_match)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            CommentMarker {
                full_match,
                ac_ref,
                anchor_text,
                position,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_paired_marker() {
        let xml = r#"<p>Some text <ac:inline-comment-marker ac:ref="abc-123">highlighted</ac:inline-comment-marker> more text</p>"#;
        let markers = extract_markers(xml);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].ac_ref, "abc-123");
        assert_eq!(markers[0].anchor_text, "highlighted");
        assert_eq!(
            markers[0].full_match,
            r#"<ac:inline-comment-marker ac:ref="abc-123">highlighted</ac:inline-comment-marker>"#
        );
        assert_eq!(markers[0].position, 13); // byte offset of "<ac:inline..." after "<p>Some text "
    }

    #[test]
    fn test_extract_self_closing_marker() {
        let xml =
            r#"<p>Text <ac:inline-comment-marker ac:ref="def-456"/> after</p>"#;
        let markers = extract_markers(xml);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].ac_ref, "def-456");
        assert_eq!(markers[0].anchor_text, "");
    }

    #[test]
    fn test_extract_multiple_markers() {
        let xml = r#"<p><ac:inline-comment-marker ac:ref="aaa">first</ac:inline-comment-marker> gap <ac:inline-comment-marker ac:ref="bbb">second</ac:inline-comment-marker></p>"#;
        let markers = extract_markers(xml);
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].ac_ref, "aaa");
        assert_eq!(markers[1].ac_ref, "bbb");
        // Document order preserved
        assert!(markers[0].position < markers[1].position);
    }

    #[test]
    fn test_extract_multiline_anchor_text() {
        let xml = "<p><ac:inline-comment-marker ac:ref=\"multi\">\nline one\nline two\n</ac:inline-comment-marker></p>";
        let markers = extract_markers(xml);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].ac_ref, "multi");
        assert_eq!(markers[0].anchor_text, "\nline one\nline two\n");
    }

    #[test]
    fn test_extract_no_markers() {
        let xml = "<p>No markers here</p>";
        let markers = extract_markers(xml);
        assert!(markers.is_empty());
    }

    #[test]
    fn test_extract_preserves_byte_offset() {
        let prefix = "AAAA"; // 4 bytes
        let xml = format!(
            r#"{}<ac:inline-comment-marker ac:ref="pos">text</ac:inline-comment-marker>"#,
            prefix
        );
        let markers = extract_markers(&xml);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].position, 4);
    }
}
