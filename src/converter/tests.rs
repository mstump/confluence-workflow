use super::*;
use crate::error::ConversionError;

/// Test-scope DiagramConfig literal — replaces the deleted Default impls on
/// DiagramConfig and MarkdownConverter (removed in Phase 10-02, D-04).
/// Values match the former defaults: plantuml_path="plantuml",
/// mermaid_path="mmdc", no puppeteer config, 30-sec timeout.
/// Mirrors the pattern used by `config_with_defaults` in `src/converter/diagrams.rs`.
fn test_diagram_config() -> crate::config::DiagramConfig {
    crate::config::DiagramConfig {
        plantuml_path: "plantuml".to_string(),
        mermaid_path: "mmdc".to_string(),
        mermaid_puppeteer_config: None,
        timeout_secs: 30,
    }
}

/// Mock converter that returns a fixed result -- proves the trait is implementable.
struct MockConverter;

#[async_trait::async_trait]
impl Converter for MockConverter {
    async fn convert(&self, markdown: &str) -> Result<ConvertResult, ConversionError> {
        Ok(ConvertResult {
            storage_xml: format!("<p>{}</p>", markdown),
            attachments: vec![Attachment {
                filename: "test.svg".to_string(),
                content: b"<svg/>".to_vec(),
                content_type: "image/svg+xml".to_string(),
            }],
        })
    }
}

#[tokio::test]
async fn test_mock_converter_compiles_and_works() {
    let converter = MockConverter;
    let result = converter.convert("hello").await.unwrap();
    assert_eq!(result.storage_xml, "<p>hello</p>");
    assert_eq!(result.attachments.len(), 1);
    assert_eq!(result.attachments[0].filename, "test.svg");
    assert_eq!(result.attachments[0].content, b"<svg/>");
    assert_eq!(result.attachments[0].content_type, "image/svg+xml");
}

#[test]
fn test_convert_result_has_storage_xml_and_attachments() {
    let result = ConvertResult {
        storage_xml: "<p>test</p>".to_string(),
        attachments: vec![],
    };
    assert_eq!(result.storage_xml, "<p>test</p>");
    assert!(result.attachments.is_empty());
}

#[test]
fn test_attachment_struct_fields() {
    let att = Attachment {
        filename: "diagram.svg".to_string(),
        content: vec![1, 2, 3],
        content_type: "image/svg+xml".to_string(),
    };
    assert_eq!(att.filename, "diagram.svg");
    assert_eq!(att.content, vec![1, 2, 3]);
    assert_eq!(att.content_type, "image/svg+xml");
}

#[test]
fn test_conversion_error_variants() {
    let render_err = ConversionError::RenderError("bad markdown".to_string());
    assert!(format!("{}", render_err).contains("bad markdown"));

    let diagram_err = ConversionError::DiagramError {
        diagram_type: "plantuml".to_string(),
        message: "jar not found".to_string(),
    };
    assert!(format!("{}", diagram_err).contains("plantuml"));
    assert!(format!("{}", diagram_err).contains("jar not found"));

    let timeout_err = ConversionError::DiagramTimeout {
        diagram_type: "mermaid".to_string(),
        timeout_secs: 30,
    };
    assert!(format!("{}", timeout_err).contains("30s"));
    assert!(format!("{}", timeout_err).contains("mermaid"));
}

#[test]
fn test_conversion_error_into_app_error() {
    use crate::error::AppError;

    let conversion_err = ConversionError::RenderError("test".to_string());
    let app_err: AppError = conversion_err.into();
    assert!(format!("{}", app_err).contains("test"));
}

// ---------- MarkdownConverter trait tests ----------

#[tokio::test]
async fn test_converter_trait_empty_input() {
    let converter = MarkdownConverter::new(test_diagram_config());
    let result = converter.convert("").await.unwrap();
    assert!(
        result.storage_xml.is_empty() || result.storage_xml.trim().is_empty(),
        "Empty input should produce empty or whitespace-only output, got: {:?}",
        result.storage_xml
    );
    assert!(result.attachments.is_empty());
}

#[tokio::test]
async fn test_converter_trait_whitespace_only() {
    let converter = MarkdownConverter::new(test_diagram_config());
    let result = converter.convert("   \n\n  ").await.unwrap();
    // Should not crash; output may be empty or whitespace
    assert!(result.attachments.is_empty());
}

#[tokio::test]
async fn test_converter_trait_frontmatter_stripped() {
    let converter = MarkdownConverter::new(test_diagram_config());
    let md = include_str!("../../tests/fixtures/frontmatter_document.md");
    let result = converter.convert(md).await.unwrap();
    assert!(
        !result.storage_xml.contains("title: Document With Frontmatter"),
        "Frontmatter YAML should not appear in output"
    );
    assert!(
        result.storage_xml.contains("Content After Frontmatter"),
        "Body content should appear in output"
    );
}

// ---------- Diagram integration tests ----------

#[tokio::test]
async fn test_no_diagrams_no_attachments() {
    let config = test_diagram_config();
    let converter = MarkdownConverter::new(config);
    let result = converter.convert("## Hello\n\nA paragraph.").await.unwrap();
    assert!(result.attachments.is_empty());
    assert!(!result.storage_xml.contains("DIAGRAM_PLACEHOLDER"));
}

#[tokio::test]
async fn test_plantuml_rendering_integration() {
    // Skip if plantuml not available
    if std::process::Command::new("plantuml")
        .arg("-version")
        .output()
        .is_err()
    {
        eprintln!("Skipping: plantuml not installed");
        return;
    }
    let config = test_diagram_config();
    let converter = MarkdownConverter::new(config);
    let md = include_str!("../../tests/fixtures/plantuml_diagram.md");
    let result = converter.convert(md).await.unwrap();
    assert_eq!(result.attachments.len(), 1);
    assert_eq!(result.attachments[0].filename, "diagram_0.svg");
    assert_eq!(result.attachments[0].content_type, "image/svg+xml");
    assert!(!result.attachments[0].content.is_empty());
    assert!(result.storage_xml.contains(r#"ri:filename="diagram_0.svg""#));
    assert!(!result.storage_xml.contains("DIAGRAM_PLACEHOLDER"));
}

#[tokio::test]
async fn test_mermaid_rendering_integration() {
    // Skip if mmdc not available
    if std::process::Command::new("mmdc")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("Skipping: mmdc not installed");
        return;
    }
    let config = test_diagram_config();
    let converter = MarkdownConverter::new(config);
    let md = include_str!("../../tests/fixtures/mermaid_diagram.md");
    let result = converter.convert(md).await;
    match result {
        Ok(result) => {
            assert_eq!(result.attachments.len(), 1);
            assert_eq!(result.attachments[0].filename, "diagram_0.svg");
            assert_eq!(result.attachments[0].content_type, "image/svg+xml");
            assert!(!result.attachments[0].content.is_empty());
            assert!(result.storage_xml.contains(r#"ri:filename="diagram_0.svg""#));
            assert!(!result.storage_xml.contains("DIAGRAM_PLACEHOLDER"));
        }
        Err(ConversionError::DiagramError { message, .. })
            if message.contains("Chrome") || message.contains("puppeteer") =>
        {
            eprintln!("Skipping: mmdc requires Chrome/puppeteer setup");
        }
        Err(e) => panic!("Unexpected error: {e}"),
    }
}

#[tokio::test]
async fn test_placeholder_replaced_with_ac_image() {
    // Skip if plantuml not available
    if std::process::Command::new("plantuml")
        .arg("-version")
        .output()
        .is_err()
    {
        eprintln!("Skipping: plantuml not installed");
        return;
    }
    let config = test_diagram_config();
    let converter = MarkdownConverter::new(config);
    let md = include_str!("../../tests/fixtures/plantuml_diagram.md");
    let result = converter.convert(md).await.unwrap();
    assert!(result.storage_xml.contains("ac:image"));
    assert!(result.storage_xml.contains("ri:attachment"));
    assert!(result.storage_xml.contains("diagram_0.svg"));
}

// ---------- Renderer tests ----------

use renderer::ConfluenceRenderer;

#[test]
fn test_headings_fixture() {
    let md = include_str!("../../tests/fixtures/spike_headings.md");
    let (output, diagrams) = ConfluenceRenderer::render(md);
    assert!(diagrams.is_empty());
    insta::assert_snapshot!("headings", output);
}

#[test]
fn test_code_blocks_fixture() {
    let md = include_str!("../../tests/fixtures/spike_code_blocks.md");
    let (output, diagrams) = ConfluenceRenderer::render(md);
    assert!(diagrams.is_empty());
    insta::assert_snapshot!("code_blocks", output);
}

#[test]
fn test_tables_fixture() {
    let md = include_str!("../../tests/fixtures/spike_tables.md");
    let (output, diagrams) = ConfluenceRenderer::render(md);
    assert!(diagrams.is_empty());
    insta::assert_snapshot!("tables", output);
}

#[test]
fn test_links_images_fixture() {
    let md = include_str!("../../tests/fixtures/spike_links_images.md");
    let (output, diagrams) = ConfluenceRenderer::render(md);
    assert!(diagrams.is_empty());
    insta::assert_snapshot!("links_images", output);
}

#[test]
fn test_nested_lists_fixture() {
    let md = include_str!("../../tests/fixtures/spike_nested_lists.md");
    let (output, diagrams) = ConfluenceRenderer::render(md);
    assert!(diagrams.is_empty());
    insta::assert_snapshot!("nested_lists", output);
}

#[test]
fn test_cdata_split() {
    let md = "```xml\nContent with ]]> inside\n```\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains("]]]]><![CDATA[>"), "CDATA should be split at ]]>");
    assert!(!output.contains("]]>]]>"), "Raw ]]> should not appear in CDATA");
}

#[test]
fn test_xml_escape() {
    let md = "Text with & < > \" and ' characters.\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains("&amp;"), "& should be escaped");
    assert!(output.contains("&lt;"), "< should be escaped");
    assert!(output.contains("&gt;"), "> should be escaped");
    assert!(output.contains("&quot;"), "\" should be escaped");
    assert!(output.contains("&apos;"), "' should be escaped");
}

#[test]
fn test_frontmatter_stripped() {
    let md = "---\ntitle: test\nauthor: someone\n---\n# Hello\n\nContent here.\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(!output.contains("title: test"), "Frontmatter should be stripped");
    assert!(!output.contains("author: someone"), "Frontmatter should be stripped");
    assert!(output.contains("Content here."), "Body content should remain");
}

#[test]
fn test_first_h1_skipped() {
    let md = "# Title\n## Sub\n\nParagraph.\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(!output.contains("<h1>"), "First h1 should be skipped");
    assert!(output.contains("<h2>Sub</h2>"), "h2 should be present");
    assert!(output.contains("Paragraph."), "Paragraph should be present");
}

#[test]
fn test_bold_italic() {
    let md = "**bold** and *italic* text.\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains("<strong>bold</strong>"));
    assert!(output.contains("<em>italic</em>"));
}

#[test]
fn test_strikethrough() {
    let md = "~~struck~~\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains(r#"<span style="text-decoration: line-through;">struck</span>"#));
}

#[test]
fn test_inline_code() {
    let md = "Use `code_here` inline.\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains("<code>code_here</code>"));
}

#[test]
fn test_blockquote() {
    let md = "> Quoted text\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains("<blockquote>"));
    assert!(output.contains("</blockquote>"));
    assert!(output.contains("Quoted text"));
}

#[test]
fn test_horizontal_rule() {
    let md = "Above\n\n---\n\nBelow\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains("<hr />"));
}

#[test]
fn test_paragraphs() {
    let md = "First paragraph.\n\nSecond paragraph.\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains("<p>First paragraph.</p>"));
    assert!(output.contains("<p>Second paragraph.</p>"));
}

#[test]
fn test_external_link() {
    let md = "[Example](https://example.com)\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains(r#"<a href="https://example.com">Example</a>"#));
}

#[test]
fn test_image_produces_ac_image() {
    let md = "![Alt text](image.png)\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains("ac:image"));
    assert!(output.contains("ri:attachment"));
    assert!(output.contains(r#"ri:filename="image.png""#));
}

#[test]
fn test_ordered_list() {
    let md = "1. First\n2. Second\n3. Third\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains("<ol>"));
    assert!(output.contains("</ol>"));
    assert!(output.contains("<li>First</li>"));
}

#[test]
fn test_unordered_list() {
    let md = "- A\n- B\n- C\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains("<ul>"));
    assert!(output.contains("</ul>"));
    assert!(output.contains("<li>A</li>"));
}

#[test]
fn test_code_block_with_language() {
    let md = "```python\nprint('hi')\n```\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains(r#"ac:name="expand""#));
    assert!(output.contains(r#"ac:name="code""#));
    assert!(output.contains(r#"ac:name="language">python</ac:parameter>"#));
    assert!(output.contains("CDATA["));
}

#[test]
fn test_code_block_without_language() {
    let md = "```\nplain code\n```\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains(r#"ac:name="code""#));
    assert!(!output.contains(r#"ac:name="language""#));
}

#[test]
fn test_diagram_block_collected() {
    let md = "```plantuml\n@startuml\nAlice -> Bob\n@enduml\n```\n";
    let (output, diagrams) = ConfluenceRenderer::render(md);
    assert_eq!(diagrams.len(), 1);
    assert_eq!(diagrams[0].kind, "plantuml");
    assert!(diagrams[0].content.contains("Alice -> Bob"));
    assert!(output.contains("DIAGRAM_PLACEHOLDER_0"));
}

#[test]
fn test_mermaid_diagram_block() {
    let md = "```mermaid\ngraph LR\n  A --> B\n```\n";
    let (output, diagrams) = ConfluenceRenderer::render(md);
    assert_eq!(diagrams.len(), 1);
    assert_eq!(diagrams[0].kind, "mermaid");
    assert!(output.contains("DIAGRAM_PLACEHOLDER_0"));
}

#[test]
fn test_table_structure() {
    let md = "| H1 | H2 |\n|---|---|\n| C1 | C2 |\n";
    let (output, _) = ConfluenceRenderer::render(md);
    assert!(output.contains("<table>"));
    assert!(output.contains("<thead>"));
    assert!(output.contains("<th>"));
    assert!(output.contains("</thead>"));
    assert!(output.contains("<tbody>"));
    assert!(output.contains("<td>"));
    assert!(output.contains("</tbody>"));
    assert!(output.contains("</table>"));
}

// ---------- Integration snapshot tests ----------

#[test]
fn test_full_document_snapshot() {
    let md = include_str!("../../tests/fixtures/full_document.md");
    let (output, diagrams) = ConfluenceRenderer::render(md);
    assert!(diagrams.is_empty(), "full_document.md has no diagram blocks");

    // Verify key structural elements
    assert!(!output.contains("<h1>"), "First h1 should be skipped");
    assert!(output.contains("<h2>Introduction</h2>"));
    assert!(output.contains("<h2>Code Examples</h2>"));
    assert!(output.contains("<h2>Data Table</h2>"));
    assert!(output.contains("<h2>Links and Images</h2>"));
    assert!(output.contains("<h2>Lists</h2>"));
    assert!(output.contains("<h2>Blockquote</h2>"));
    assert!(output.contains(r#"ac:name="expand""#));
    assert!(output.contains("<table>"));
    assert!(output.contains(r#"<a href="https://confluence.atlassian.com">"#));
    assert!(output.contains(r#"ri:filename="architecture.png""#));
    assert!(output.contains("<ul>"));
    assert!(output.contains("<ol>"));
    assert!(output.contains("<blockquote>"));
    assert!(output.contains("<hr />"));

    // No frontmatter text in output
    assert!(!output.contains("title: Full Test Document"));
    assert!(!output.contains("author: Test Suite"));

    insta::assert_snapshot!("full_document", output);
}

#[test]
fn test_frontmatter_stripped_end_to_end() {
    let md = include_str!("../../tests/fixtures/frontmatter_document.md");
    let (output, _) = ConfluenceRenderer::render(md);

    // Frontmatter content must NOT appear
    assert!(
        !output.contains("title: Document With Frontmatter"),
        "YAML title should not appear"
    );
    assert!(!output.contains("tags:"), "YAML tags key should not appear");
    assert!(
        !output.contains("date: 2026"),
        "YAML date should not appear"
    );

    // Body content MUST appear
    assert!(
        output.contains("Content After Frontmatter"),
        "Heading text should appear"
    );
    assert!(
        output.contains("This paragraph should appear in output"),
        "Paragraph text should appear"
    );
}

#[test]
fn test_edge_cases_snapshot() {
    let md = include_str!("../../tests/fixtures/edge_cases.md");
    let (output, diagrams) = ConfluenceRenderer::render(md);
    assert!(diagrams.is_empty());

    // Verify special characters are escaped
    assert!(output.contains("&amp;"), "Ampersands should be escaped");

    // Verify link with ampersand in URL is handled
    assert!(
        output.contains("https://example.com?foo=1&amp;bar=2"),
        "Ampersand in URL should be escaped in href attribute"
    );

    // Verify heading with inline code
    assert!(
        output.contains("<code>code</code>"),
        "Inline code in heading should render"
    );

    insta::assert_snapshot!("edge_cases", output);
}

#[tokio::test]
async fn test_mock_converter_returns_fixed_result() {
    // Verify MockConverter is usable for downstream testing
    let mock = MockConverter;
    let result = mock.convert("test input").await.unwrap();
    assert_eq!(result.storage_xml, "<p>test input</p>");
    assert_eq!(result.attachments.len(), 1);
    assert_eq!(result.attachments[0].filename, "test.svg");
    assert_eq!(result.attachments[0].content_type, "image/svg+xml");
}

/// Trait-boundary lock-in: invoke `MarkdownConverter::convert` via `&dyn Converter`
/// (not as an inherent method) so the trait impl cannot be silently removed.
///
/// Roadmap Phase 10 success criterion: "Converter trait is exercised in the
/// integration test path." The integration tests in `tests/cli_integration.rs`
/// exercise the trait transparently through `src/lib.rs::run` (see the
/// 10-02-SUMMARY audit), but that coverage is indirect — if the `impl Converter
/// for MarkdownConverter` block were ever deleted, the integration tests would
/// keep compiling (they call `.convert(...)` via an inherent method on the
/// concrete type). This test fails to compile without the trait impl.
#[tokio::test]
async fn test_converter_trait_object_invocation() {
    let concrete = MarkdownConverter::new(test_diagram_config());
    let trait_obj: &dyn Converter = &concrete;
    let result = trait_obj
        .convert("# Heading\n\nBody paragraph.")
        .await
        .expect("trait-object convert should succeed on plain markdown");
    assert!(
        !result.storage_xml.trim().is_empty(),
        "storage_xml should be non-empty; got: {:?}",
        result.storage_xml
    );
    assert!(
        result.attachments.is_empty(),
        "plain markdown has no diagrams, so attachments must be empty; got len={}",
        result.attachments.len()
    );
}
