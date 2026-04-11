use regex::Regex;
use std::sync::LazyLock;

use super::{CommentDecision, CommentMarker};

/// A heading-scoped section of Confluence storage HTML.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// Heading text (empty string for preamble before first heading).
    pub heading: String,
    /// Heading level 1-6 (0 for preamble).
    pub heading_level: u8,
    /// Full HTML content from heading tag to next heading (or end of document).
    pub content: String,
    /// Byte offset of start in original content.
    pub start_offset: usize,
    /// Byte offset of end in original content.
    pub end_offset: usize,
}

/// Regex to find heading open tags (h1-h6) and capture content up to close tag.
/// Since the `regex` crate doesn't support backreferences, we match opening tags
/// and then find the corresponding close tag programmatically.
static HEADING_OPEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<h([1-6])\b[^>]*>").expect("heading open regex must compile")
});

/// Regex to match paired inline comment marker open tags.
static MARKER_OPEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<ac:inline-comment-marker\b[^>]*?>")
        .expect("marker open regex must compile")
});

/// Regex to match self-closing inline comment markers.
static MARKER_SELF_CLOSING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<ac:inline-comment-marker\b[^>]*?/>")
        .expect("marker self-closing regex must compile")
});

/// Extract heading-scoped sections from Confluence storage HTML.
///
/// Content before the first heading becomes a preamble section with
/// `heading=""` and `heading_level=0`. Each heading's section extends
/// from that heading tag to the start of the next heading or end of document.
pub fn extract_sections(html: &str) -> Vec<Section> {
    if html.is_empty() {
        return Vec::new();
    }

    // Collect heading positions: (start_of_open_tag, level, heading_text)
    let mut headings: Vec<(usize, u8, String)> = Vec::new();

    for cap in HEADING_OPEN_RE.captures_iter(html) {
        let open_tag = cap.get(0).unwrap();
        let level: u8 = cap[1].parse().unwrap();
        let after_open = open_tag.end();

        // Find the corresponding close tag </hN>
        let close_tag = format!("</h{level}>");
        if let Some(close_pos) = html[after_open..].find(&close_tag) {
            let heading_text = html[after_open..after_open + close_pos].to_string();
            headings.push((open_tag.start(), level, heading_text));
        }
    }

    if headings.is_empty() {
        // No headings -- entire content is a preamble section
        return vec![Section {
            heading: String::new(),
            heading_level: 0,
            content: html.to_string(),
            start_offset: 0,
            end_offset: html.len(),
        }];
    }

    let mut sections = Vec::new();

    // Check for preamble before first heading
    if headings[0].0 > 0 {
        sections.push(Section {
            heading: String::new(),
            heading_level: 0,
            content: html[..headings[0].0].to_string(),
            start_offset: 0,
            end_offset: headings[0].0,
        });
    }

    // Build sections from headings
    for (i, (start, level, heading_text)) in headings.iter().enumerate() {
        let end = if i + 1 < headings.len() {
            headings[i + 1].0
        } else {
            html.len()
        };

        sections.push(Section {
            heading: heading_text.clone(),
            heading_level: *level,
            content: html[*start..end].to_string(),
            start_offset: *start,
            end_offset: end,
        });
    }

    sections
}

/// Find the first section with an exactly matching heading.
pub fn find_matching_section<'a>(heading: &str, sections: &'a [Section]) -> Option<&'a Section> {
    sections.iter().find(|s| s.heading == heading)
}

/// Strip inline comment marker tags from content, preserving anchor text.
///
/// Removes `<ac:inline-comment-marker ...>` open tags, `</ac:inline-comment-marker>` close tags,
/// and self-closing `<ac:inline-comment-marker .../>` tags.
pub fn strip_markers(content: &str) -> String {
    // First remove self-closing markers entirely
    let result = MARKER_SELF_CLOSING_RE.replace_all(content, "");
    // Remove open tags (preserving anchor text between them)
    let result = MARKER_OPEN_RE.replace_all(&result, "");
    // Remove close tags
    result.replace("</ac:inline-comment-marker>", "")
}

/// Classify a comment marker as deterministic KEEP, DROP, or ambiguous (None).
///
/// - `Some(Keep)`: marker's section content is identical in old and new (after stripping markers)
/// - `Some(Drop)`: marker's section heading has no match in new content, or marker is orphaned
/// - `None`: sections exist but content differs (needs LLM evaluation)
pub fn classify_comment(
    marker: &CommentMarker,
    old_sections: &[Section],
    new_sections: &[Section],
) -> Option<CommentDecision> {
    // Find which old section contains this marker by position
    let old_section = old_sections
        .iter()
        .find(|s| marker.position >= s.start_offset && marker.position < s.end_offset);

    let old_section = match old_section {
        Some(s) => s,
        None => return Some(CommentDecision::Drop), // orphaned marker
    };

    // Find matching new section by heading text
    let new_section = match find_matching_section(&old_section.heading, new_sections) {
        Some(s) => s,
        None => return Some(CommentDecision::Drop), // section deleted
    };

    // Compare stripped content
    let old_stripped = strip_markers(&old_section.content);
    let new_stripped = strip_markers(&new_section.content);

    if old_stripped == new_stripped {
        Some(CommentDecision::Keep) // unchanged section
    } else {
        None // ambiguous -- needs LLM
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_sections_two_headings() {
        let html = "<h2>First</h2><p>Content one</p><h2>Second</h2><p>Content two</p>";
        let sections = extract_sections(html);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading, "First");
        assert_eq!(sections[0].heading_level, 2);
        assert!(sections[0].content.contains("Content one"));
        assert_eq!(sections[1].heading, "Second");
        assert_eq!(sections[1].heading_level, 2);
        assert!(sections[1].content.contains("Content two"));
        // Offsets are correct
        assert_eq!(sections[0].start_offset, 0);
        assert_eq!(sections[0].end_offset, sections[1].start_offset);
    }

    #[test]
    fn test_extract_sections_h1_through_h6() {
        let html = "<h1>H1</h1><h2>H2</h2><h3>H3</h3><h4>H4</h4><h5>H5</h5><h6>H6</h6>";
        let sections = extract_sections(html);
        assert_eq!(sections.len(), 6);
        for (i, section) in sections.iter().enumerate() {
            assert_eq!(section.heading_level, (i + 1) as u8);
        }
    }

    #[test]
    fn test_extract_sections_preamble() {
        let html = "<p>Preamble text</p><h2>First</h2><p>Body</p>";
        let sections = extract_sections(html);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading, "");
        assert_eq!(sections[0].heading_level, 0);
        assert!(sections[0].content.contains("Preamble text"));
        assert_eq!(sections[1].heading, "First");
    }

    #[test]
    fn test_extract_sections_empty_string() {
        let sections = extract_sections("");
        assert!(sections.is_empty());
    }

    #[test]
    fn test_find_matching_section_found() {
        let sections = vec![
            Section {
                heading: "Alpha".to_string(),
                heading_level: 2,
                content: "alpha content".to_string(),
                start_offset: 0,
                end_offset: 13,
            },
            Section {
                heading: "Beta".to_string(),
                heading_level: 2,
                content: "beta content".to_string(),
                start_offset: 13,
                end_offset: 25,
            },
        ];
        let found = find_matching_section("Beta", &sections);
        assert!(found.is_some());
        assert_eq!(found.unwrap().heading, "Beta");
    }

    #[test]
    fn test_find_matching_section_not_found() {
        let sections = vec![Section {
            heading: "Alpha".to_string(),
            heading_level: 2,
            content: "alpha content".to_string(),
            start_offset: 0,
            end_offset: 13,
        }];
        let found = find_matching_section("Gamma", &sections);
        assert!(found.is_none());
    }

    #[test]
    fn test_classify_comment_keep_unchanged() {
        let html_old = r#"<h2>Title</h2><p>Some <ac:inline-comment-marker ac:ref="abc">marked</ac:inline-comment-marker> text</p>"#;
        let html_new = "<h2>Title</h2><p>Some marked text</p>";

        let old_sections = extract_sections(html_old);
        let new_sections = extract_sections(html_new);

        let marker = CommentMarker {
            full_match: r#"<ac:inline-comment-marker ac:ref="abc">marked</ac:inline-comment-marker>"#.to_string(),
            ac_ref: "abc".to_string(),
            anchor_text: "marked".to_string(),
            position: 19, // inside the old section
        };

        let result = classify_comment(&marker, &old_sections, &new_sections);
        assert_eq!(result, Some(CommentDecision::Keep));
    }

    #[test]
    fn test_classify_comment_drop_section_deleted() {
        let html_old = "<h2>Removed</h2><p>Old content</p>";
        let html_new = "<h2>Different</h2><p>New content</p>";

        let old_sections = extract_sections(html_old);
        let new_sections = extract_sections(html_new);

        let marker = CommentMarker {
            full_match: r#"<ac:inline-comment-marker ac:ref="x">text</ac:inline-comment-marker>"#.to_string(),
            ac_ref: "x".to_string(),
            anchor_text: "text".to_string(),
            position: 5, // inside the old section
        };

        let result = classify_comment(&marker, &old_sections, &new_sections);
        assert_eq!(result, Some(CommentDecision::Drop));
    }

    #[test]
    fn test_classify_comment_ambiguous_content_differs() {
        let html_old = "<h2>Title</h2><p>Old paragraph</p>";
        let html_new = "<h2>Title</h2><p>New paragraph</p>";

        let old_sections = extract_sections(html_old);
        let new_sections = extract_sections(html_new);

        let marker = CommentMarker {
            full_match: r#"<ac:inline-comment-marker ac:ref="y">Old</ac:inline-comment-marker>"#.to_string(),
            ac_ref: "y".to_string(),
            anchor_text: "Old".to_string(),
            position: 5,
        };

        let result = classify_comment(&marker, &old_sections, &new_sections);
        assert_eq!(result, None); // ambiguous, needs LLM
    }

    #[test]
    fn test_strip_markers_preserves_anchor_text() {
        let content = r#"<p>Some <ac:inline-comment-marker ac:ref="abc">highlighted</ac:inline-comment-marker> text</p>"#;
        let stripped = strip_markers(content);
        assert_eq!(stripped, "<p>Some highlighted text</p>");
    }

    #[test]
    fn test_strip_markers_removes_self_closing() {
        let content = r#"<p>Text <ac:inline-comment-marker ac:ref="x"/> more</p>"#;
        let stripped = strip_markers(content);
        assert_eq!(stripped, "<p>Text  more</p>");
    }
}
