use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::fmt::Write;

/// A diagram code block extracted during rendering, to be processed separately.
#[derive(Debug, Clone)]
pub struct DiagramBlock {
    /// Diagram type: "plantuml", "puml", or "mermaid".
    pub kind: String,
    /// Raw diagram source content.
    pub content: String,
}

/// Renders Markdown to Confluence storage format XML using pulldown-cmark events.
pub struct ConfluenceRenderer {
    output: String,
    in_code_block: bool,
    code_language: Option<String>,
    code_content: String,
    table_alignments: Vec<Alignment>,
    in_table_head: bool,
    had_table_head: bool,
    first_h1_skipped: bool,
    skip_heading_content: bool,
    in_image: bool,
    list_stack: Vec<bool>, // true = ordered
    in_metadata_block: bool,
}

impl ConfluenceRenderer {
    fn new() -> Self {
        Self {
            output: String::new(),
            in_code_block: false,
            code_language: None,
            code_content: String::new(),
            table_alignments: Vec::new(),
            in_table_head: false,
            had_table_head: false,
            first_h1_skipped: false,
            skip_heading_content: false,
            in_image: false,
            list_stack: Vec::new(),
            in_metadata_block: false,
        }
    }

    /// Render Markdown to Confluence storage XML.
    ///
    /// Returns the XML string and a list of diagram blocks that need
    /// separate rendering (their positions are marked with
    /// `<!-- DIAGRAM_PLACEHOLDER_N -->` comments in the output).
    pub fn render(markdown: &str) -> (String, Vec<DiagramBlock>) {
        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        opts.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        opts.insert(Options::ENABLE_TASKLISTS);

        let parser = Parser::new_ext(markdown, opts);
        let mut renderer = Self::new();
        let mut diagram_blocks: Vec<DiagramBlock> = Vec::new();

        for event in parser {
            match event {
                // --- Metadata / frontmatter ---
                Event::Start(Tag::MetadataBlock(_)) => {
                    renderer.in_metadata_block = true;
                }
                Event::End(TagEnd::MetadataBlock(_)) => {
                    renderer.in_metadata_block = false;
                }

                // --- Headings ---
                Event::Start(Tag::Heading { level, .. }) => {
                    let lvl = level as u8;
                    if lvl == 1 && !renderer.first_h1_skipped {
                        renderer.first_h1_skipped = true;
                        renderer.skip_heading_content = true;
                    } else {
                        write!(renderer.output, "<h{}>", lvl).unwrap();
                    }
                }
                Event::End(TagEnd::Heading(level)) => {
                    if renderer.skip_heading_content {
                        renderer.skip_heading_content = false;
                    } else {
                        let lvl = level as u8;
                        write!(renderer.output, "</h{}>", lvl).unwrap();
                    }
                }

                // --- Paragraphs ---
                Event::Start(Tag::Paragraph) => {
                    renderer.output.push_str("<p>");
                }
                Event::End(TagEnd::Paragraph) => {
                    renderer.output.push_str("</p>");
                }

                // --- Code blocks ---
                Event::Start(Tag::CodeBlock(kind)) => {
                    renderer.in_code_block = true;
                    renderer.code_content.clear();
                    match kind {
                        CodeBlockKind::Fenced(lang) => {
                            let lang_str = lang.to_string();
                            if lang_str.is_empty() {
                                renderer.code_language = None;
                            } else {
                                renderer.code_language = Some(lang_str);
                            }
                        }
                        CodeBlockKind::Indented => {
                            renderer.code_language = None;
                        }
                    }
                }
                Event::End(TagEnd::CodeBlock) => {
                    let is_diagram = renderer
                        .code_language
                        .as_deref()
                        .map(|l| l == "plantuml" || l == "puml" || l == "mermaid")
                        .unwrap_or(false);

                    if is_diagram {
                        let kind = renderer.code_language.take().unwrap();
                        let content = std::mem::take(&mut renderer.code_content);
                        diagram_blocks.push(DiagramBlock { kind, content });
                        write!(
                            renderer.output,
                            "<!-- DIAGRAM_PLACEHOLDER_{:04} -->",
                            diagram_blocks.len() - 1
                        )
                        .unwrap();
                    } else {
                        renderer.emit_code_block();
                    }
                    renderer.in_code_block = false;
                    renderer.code_language = None;
                }

                // --- Blockquotes ---
                Event::Start(Tag::BlockQuote(_)) => {
                    renderer.output.push_str("<blockquote>");
                }
                Event::End(TagEnd::BlockQuote(_)) => {
                    renderer.output.push_str("</blockquote>");
                }

                // --- Lists ---
                Event::Start(Tag::List(start)) => {
                    if start.is_some() {
                        renderer.list_stack.push(true);
                        renderer.output.push_str("<ol>");
                    } else {
                        renderer.list_stack.push(false);
                        renderer.output.push_str("<ul>");
                    }
                }
                Event::End(TagEnd::List(_)) => {
                    let ordered = renderer.list_stack.pop().unwrap_or(false);
                    if ordered {
                        renderer.output.push_str("</ol>");
                    } else {
                        renderer.output.push_str("</ul>");
                    }
                }
                Event::Start(Tag::Item) => {
                    renderer.output.push_str("<li>");
                }
                Event::End(TagEnd::Item) => {
                    renderer.output.push_str("</li>");
                }

                // --- Tables ---
                Event::Start(Tag::Table(alignments)) => {
                    renderer.table_alignments = alignments;
                    renderer.had_table_head = false;
                    renderer.output.push_str("<table>");
                }
                Event::End(TagEnd::Table) => {
                    if renderer.had_table_head {
                        renderer.output.push_str("</tbody>");
                    }
                    renderer.output.push_str("</table>");
                }
                Event::Start(Tag::TableHead) => {
                    renderer.in_table_head = true;
                    renderer.had_table_head = true;
                    renderer.output.push_str("<thead><tr>");
                }
                Event::End(TagEnd::TableHead) => {
                    renderer.in_table_head = false;
                    renderer.output.push_str("</tr></thead><tbody>");
                }
                Event::Start(Tag::TableRow) => {
                    renderer.output.push_str("<tr>");
                }
                Event::End(TagEnd::TableRow) => {
                    renderer.output.push_str("</tr>");
                }
                Event::Start(Tag::TableCell) => {
                    if renderer.in_table_head {
                        renderer.output.push_str("<th>");
                    } else {
                        renderer.output.push_str("<td>");
                    }
                }
                Event::End(TagEnd::TableCell) => {
                    if renderer.in_table_head {
                        renderer.output.push_str("</th>");
                    } else {
                        renderer.output.push_str("</td>");
                    }
                }

                // --- Inline formatting ---
                Event::Start(Tag::Emphasis) => {
                    renderer.output.push_str("<em>");
                }
                Event::End(TagEnd::Emphasis) => {
                    renderer.output.push_str("</em>");
                }
                Event::Start(Tag::Strong) => {
                    renderer.output.push_str("<strong>");
                }
                Event::End(TagEnd::Strong) => {
                    renderer.output.push_str("</strong>");
                }
                Event::Start(Tag::Strikethrough) => {
                    renderer
                        .output
                        .push_str(r#"<span style="text-decoration: line-through;">"#);
                }
                Event::End(TagEnd::Strikethrough) => {
                    renderer.output.push_str("</span>");
                }

                // --- Links ---
                Event::Start(Tag::Link { dest_url, .. }) => {
                    write!(
                        renderer.output,
                        r#"<a href="{}">"#,
                        escape_attr(&dest_url)
                    )
                    .unwrap();
                }
                Event::End(TagEnd::Link) => {
                    renderer.output.push_str("</a>");
                }

                // --- Images ---
                Event::Start(Tag::Image {
                    dest_url, title, ..
                }) => {
                    // Extract filename from the URL path
                    let filename = dest_url
                        .rsplit('/')
                        .next()
                        .unwrap_or(&dest_url);
                    // Collect alt text from child Text events -- for now use title or empty
                    // The alt text comes as Text events between Start(Image) and End(Image),
                    // so we handle it by emitting the opening tag and collecting alt in a
                    // simplified way. We use the title if available.
                    let alt = if title.is_empty() {
                        String::new()
                    } else {
                        title.to_string()
                    };
                    write!(
                        renderer.output,
                        r#"<ac:image ac:alt="{alt}"><ri:attachment ri:filename="{filename}" /></ac:image>"#,
                        alt = escape_attr(&alt),
                        filename = escape_attr(filename),
                    )
                    .unwrap();
                    // Skip text events inside image (they are alt text which we already handled)
                    renderer.in_image = true;
                }
                Event::End(TagEnd::Image) => {
                    renderer.in_image = false;
                }

                // --- Text content ---
                Event::Text(text) => {
                    if renderer.in_metadata_block {
                        // Skip frontmatter content
                    } else if renderer.in_code_block {
                        renderer.code_content.push_str(&text);
                    } else if renderer.skip_heading_content || renderer.in_image {
                        // Skip (first h1 or image alt text)
                    } else {
                        renderer.push_escaped(&text);
                    }
                }

                // --- Inline code ---
                Event::Code(text) => {
                    if !renderer.skip_heading_content && !renderer.in_image {
                        renderer.output.push_str("<code>");
                        renderer.push_escaped(&text);
                        renderer.output.push_str("</code>");
                    }
                }

                // --- Breaks ---
                Event::SoftBreak => {
                    if !renderer.skip_heading_content && !renderer.in_image {
                        renderer.output.push('\n');
                    }
                }
                Event::HardBreak => {
                    if !renderer.skip_heading_content && !renderer.in_image {
                        renderer.output.push_str("<br />");
                    }
                }

                // --- Horizontal rule ---
                Event::Rule => {
                    renderer.output.push_str("<hr />");
                }

                // --- Task list markers ---
                Event::TaskListMarker(checked) => {
                    if checked {
                        renderer.output.push_str("[x] ");
                    } else {
                        renderer.output.push_str("[ ] ");
                    }
                }

                // --- Everything else (HTML blocks, footnotes, etc.) ---
                _ => {}
            }
        }

        (renderer.output, diagram_blocks)
    }

    /// Emit a Confluence code block macro wrapped in an expand macro.
    fn emit_code_block(&mut self) {
        self.output
            .push_str(r#"<ac:structured-macro ac:name="expand">"#);
        self.output
            .push_str(r#"<ac:parameter ac:name="title">Source</ac:parameter>"#);
        self.output.push_str("<ac:rich-text-body>");
        self.output
            .push_str(r#"<ac:structured-macro ac:name="code">"#);
        if let Some(ref lang) = self.code_language {
            if !lang.is_empty() {
                write!(
                    self.output,
                    r#"<ac:parameter ac:name="language">{}</ac:parameter>"#,
                    lang
                )
                .unwrap();
            }
        }
        self.output
            .push_str("<ac:plain-text-body><![CDATA[");
        // CDATA split for ]]> (Pitfall 2)
        let safe_content = self.code_content.replace("]]>", "]]]]><![CDATA[>");
        self.output.push_str(&safe_content);
        self.output.push_str("]]></ac:plain-text-body>");
        self.output.push_str("</ac:structured-macro>");
        self.output.push_str("</ac:rich-text-body>");
        self.output.push_str("</ac:structured-macro>");
        self.code_content.clear();
    }

    /// Append XML-escaped text to the output buffer.
    fn push_escaped(&mut self, text: &str) {
        self.output.push_str(&escape_xml(text));
    }
}

/// Escape text for safe inclusion in XML text nodes.
pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Escape text for safe inclusion in XML attribute values.
pub fn escape_attr(s: &str) -> String {
    escape_xml(s)
}
