# Phase 2: Markdown-to-Confluence Storage Format Converter - Research

**Researched:** 2026-04-10
**Domain:** Markdown parsing, Confluence storage format XML generation, diagram rendering
**Confidence:** HIGH

## Summary

Phase 2 converts Markdown files to Confluence storage format (a non-standard XHTML dialect using `ac:` and `ri:` namespace prefixes). The core work is a custom pulldown-cmark event visitor that emits Confluence-compatible XML instead of HTML. pulldown-cmark 0.13.3 provides native parsing for all required Markdown elements (headings, code blocks, tables, links, images) plus YAML frontmatter detection via `ENABLE_YAML_STYLE_METADATA_BLOCKS`, eliminating the need for a separate frontmatter stripping step.

The key architectural risk is XML namespace handling: Confluence storage format uses `ac:` and `ri:` prefixed elements without declaring the namespaces in the fragment. The recommended approach is string-based XML generation (not quick-xml Writer) since the output is well-structured fragments, not full XML documents. The Python converter already follows this pattern -- it generates storage format via string concatenation through the `md2conf` library. The Rust converter should do the same: iterate pulldown-cmark events, write to a `String` buffer using `write!` / `push_str`, and produce the storage fragment directly.

**Primary recommendation:** Build a custom pulldown-cmark event-to-string renderer that emits Confluence storage XML directly via string formatting. Use `tokio::process::Command` for PlantUML (jar pipe mode) and Mermaid (mmdc CLI) diagram rendering. Do NOT use quick-xml for output generation -- string building avoids all namespace prefix issues.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CONV-01 | Convert Markdown to Confluence storage XML format (Confluence XHTML with `ac:*` elements) | pulldown-cmark 0.13.3 event iterator covers all required elements; custom visitor pattern documented below |
| CONV-02 | Strip Obsidian YAML frontmatter before conversion | pulldown-cmark `ENABLE_YAML_STYLE_METADATA_BLOCKS` detects frontmatter as `MetadataBlock` events -- skip them during rendering |
| CONV-03 | Render PlantUML diagrams to SVG -- configurable as JAR path or HTTP server URL | `plantuml` CLI available locally; supports `--pipe` mode for stdin/stdout; `tokio::process::Command` for async subprocess |
| CONV-04 | Render Mermaid diagrams to SVG via mermaid-cli | `mmdc` 11.12.0 available locally; temp file input/output pattern from Python converter works |
| CONV-05 | Converter is trait-based for testability | `Converter` trait with async method returning `(String, Vec<Attachment>)` -- follows `ConfluenceApi` pattern from Phase 1 |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| pulldown-cmark | 0.13.3 | Markdown parsing to event stream | De facto Rust Markdown parser; CommonMark + GFM extensions; pull-based iterator model [VERIFIED: cargo search] |
| tokio (already in deps) | 1.51 | Async subprocess execution for diagram rendering | Already in workspace; `tokio::process::Command` for non-blocking PlantUML/Mermaid calls [VERIFIED: Cargo.toml] |
| regex (already in deps) | 1 | Fenced code block language detection, post-processing | Already in workspace [VERIFIED: Cargo.toml] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tempfile | 3 | Temp files for mermaid-cli input/output | Mermaid rendering requires file paths, not stdin pipe [ASSUMED] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| pulldown-cmark | comrak 0.52 | comrak is full GFM; pulldown-cmark is lighter, more composable iterator model, better for custom output |
| String buffer output | quick-xml Writer | quick-xml requires namespace declarations; Confluence fragments have undeclared `ac:` / `ri:` prefixes -- string building is simpler and correct |
| Custom visitor | md2conf (Python bridge) | Fallback option if spike fails; adds Python runtime dependency (contradicts REQUIREMENTS out-of-scope) |

**Installation:**
```bash
cargo add pulldown-cmark@0.13.3
cargo add tempfile@3 --dev  # Or as regular dep if needed at runtime
```

**Version verification:**
- pulldown-cmark: 0.13.3 [VERIFIED: cargo search 2026-04-10]
- quick-xml: 0.39.2 (NOT recommended for output; noted for reference) [VERIFIED: cargo search 2026-04-10]
- tempfile: latest 3.x [ASSUMED -- standard, stable API]

## Architecture Patterns

### Recommended Project Structure
```
src/
├── converter/
│   ├── mod.rs           # Converter trait + ConvertResult type
│   ├── renderer.rs      # pulldown-cmark event → Confluence storage XML
│   ├── frontmatter.rs   # Frontmatter detection and stripping (may be inline in renderer)
│   └── diagrams.rs      # PlantUML + Mermaid subprocess rendering
├── confluence/          # (existing from Phase 1)
├── config.rs            # (existing -- add PlantUML/Mermaid config fields)
├── error.rs             # (existing -- add ConversionError variant)
└── lib.rs               # (existing -- add converter module)
```

### Pattern 1: Pull-Parser Event Visitor
**What:** Iterate pulldown-cmark events, match on `Start(tag)` / `End(tag)` / `Text` / `Code` etc., and write corresponding Confluence XML to a `String` buffer.
**When to use:** This is the core conversion pattern for CONV-01.
**Example:**
```rust
// Source: pulldown-cmark docs + Confluence storage format reference
use pulldown_cmark::{Parser, Event, Tag, TagEnd, Options, CodeBlockKind};

pub struct ConfluenceRenderer {
    output: String,
    // Track state for multi-event constructs
    in_code_block: bool,
    code_language: Option<String>,
    code_content: String,
    table_alignments: Vec<pulldown_cmark::Alignment>,
    in_table_head: bool,
}

impl ConfluenceRenderer {
    pub fn render(markdown: &str) -> (String, Vec<DiagramBlock>) {
        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        opts.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        opts.insert(Options::ENABLE_TASKLISTS);

        let parser = Parser::new_ext(markdown, opts);
        let mut renderer = Self::new();
        let mut diagram_blocks = Vec::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    write!(renderer.output, "<h{}>", level as u8).unwrap();
                }
                Event::End(TagEnd::Heading(level)) => {
                    write!(renderer.output, "</h{}>", level as u8).unwrap();
                }
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                    let lang_str = lang.to_string();
                    if lang_str == "plantuml" || lang_str == "puml" || lang_str == "mermaid" {
                        // Collect content; render diagram later
                        renderer.in_code_block = true;
                        renderer.code_language = Some(lang_str);
                    } else {
                        renderer.in_code_block = true;
                        renderer.code_language = if lang_str.is_empty() {
                            None
                        } else {
                            Some(lang_str)
                        };
                    }
                }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some(ref lang) = renderer.code_language {
                        if lang == "plantuml" || lang == "puml" || lang == "mermaid" {
                            diagram_blocks.push(DiagramBlock {
                                kind: lang.clone(),
                                content: std::mem::take(&mut renderer.code_content),
                            });
                            // Placeholder replaced after rendering
                            write!(renderer.output,
                                "<!-- DIAGRAM_PLACEHOLDER_{} -->",
                                diagram_blocks.len() - 1
                            ).unwrap();
                        } else {
                            // Emit Confluence code macro
                            renderer.emit_code_block();
                        }
                    } else {
                        renderer.emit_code_block();
                    }
                    renderer.in_code_block = false;
                    renderer.code_language = None;
                }
                Event::Text(text) => {
                    if renderer.in_code_block {
                        renderer.code_content.push_str(&text);
                    } else {
                        // XML-escape text content
                        renderer.push_escaped(&text);
                    }
                }
                // ... other events
                _ => {}
            }
        }
        (renderer.output, diagram_blocks)
    }
}
```

### Pattern 2: Confluence Code Block Macro
**What:** Map fenced code blocks to `ac:structured-macro name="code"`.
**When to use:** Every fenced code block that is not a diagram.
**Example:**
```rust
// Source: Confluence Storage Format docs
// https://confluence.atlassian.com/doc/confluence-storage-format-790796544.html
fn emit_code_block(&mut self) {
    self.output.push_str(r#"<ac:structured-macro ac:name="code">"#);
    if let Some(ref lang) = self.code_language {
        write!(self.output,
            r#"<ac:parameter ac:name="language">{}</ac:parameter>"#,
            lang
        ).unwrap();
    }
    self.output.push_str("<ac:plain-text-body><![CDATA[");
    // CDATA content does NOT need XML escaping
    self.output.push_str(&self.code_content);
    self.output.push_str("]]></ac:plain-text-body>");
    self.output.push_str("</ac:structured-macro>");
    self.code_content.clear();
}
```

### Pattern 3: Image Attachment Reference
**What:** Map Markdown images to `ac:image` + `ri:attachment`.
**When to use:** Every `![alt](src)` image reference, and diagram SVG outputs.
**Example:**
```rust
// Source: Confluence Storage Format docs
// https://confluence.atlassian.com/doc/confluence-storage-format-790796544.html
fn emit_image(&mut self, filename: &str, alt: &str) {
    write!(self.output,
        r#"<ac:image ac:alt="{alt}" ac:width="100%"><ri:attachment ri:filename="{filename}" /></ac:image>"#,
        alt = Self::escape_attr(alt),
        filename = Self::escape_attr(filename),
    ).unwrap();
}
```

### Pattern 4: Table Rendering
**What:** Map GFM tables to standard HTML table elements (Confluence accepts standard `<table>` markup).
**Example:**
```rust
// Confluence accepts standard XHTML tables
// Event::Start(Tag::Table(alignments)) -> <table>
// Event::Start(Tag::TableHead) -> <thead><tr>
// Event::Start(Tag::TableCell) when in_table_head -> <th>
// Event::Start(Tag::TableCell) when !in_table_head -> <td>
// Event::Start(Tag::TableRow) -> <tr>
```

### Pattern 5: Diagram Rendering via Async Subprocess
**What:** Render PlantUML/Mermaid fenced blocks to SVG via subprocess, return as attachments.
**When to use:** CONV-03 and CONV-04.
**Example:**
```rust
// PlantUML: pipe mode (stdin -> stdout)
use tokio::process::Command;

async fn render_plantuml(content: &str, jar_path: &str) -> Result<Vec<u8>, ConversionError> {
    let output = Command::new("java")
        .args(["-jar", jar_path, "-tsvg", "-pipe"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    // write content to stdin, read SVG from stdout
    // ...
}

// Mermaid: file-based (temp input -> temp output)
async fn render_mermaid(content: &str, mmdc_path: &str) -> Result<Vec<u8>, ConversionError> {
    let input = tempfile::Builder::new().suffix(".mmd").tempfile()?;
    std::fs::write(input.path(), content)?;
    let output_path = input.path().with_extension("svg");
    Command::new(mmdc_path)
        .args(["-i", &input.path().to_string_lossy(), "-o", &output_path.to_string_lossy()])
        .status()
        .await?;
    std::fs::read(&output_path).map_err(Into::into)
}
```

### Pattern 6: Converter Trait (CONV-05)
**What:** Trait boundary for testability, following Phase 1's `ConfluenceApi` pattern.
**Example:**
```rust
use async_trait::async_trait;

pub struct Attachment {
    pub filename: String,
    pub content: Vec<u8>,
    pub content_type: String,
}

pub struct ConvertResult {
    pub storage_xml: String,
    pub attachments: Vec<Attachment>,
}

#[async_trait]
pub trait Converter: Send + Sync {
    async fn convert(&self, markdown: &str) -> Result<ConvertResult, ConversionError>;
}
```

### Anti-Patterns to Avoid
- **Using quick-xml Writer for output:** The `ac:` and `ri:` namespace prefixes are not declared in Confluence storage fragments. quick-xml will either reject them or require wrapping in a synthetic root with `xmlns:ac` / `xmlns:ri` declarations. String building is simpler and produces correct output.
- **Parsing the output XML for validation:** The output is a fragment, not a well-formed XML document. Parsing it back with an XML parser will fail on namespace prefixes. Validate by uploading to Confluence or using substring assertions in tests.
- **Regex-based Markdown parsing:** The Python converter uses regex for some preprocessing (code blocks, diagrams). The Rust version should rely on pulldown-cmark events for all structural parsing -- regex is fragile for nested/edge-case Markdown.
- **Blocking subprocess calls:** PlantUML and Mermaid rendering must use `tokio::process::Command`, not `std::process::Command`, to avoid blocking the async runtime.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Markdown parsing | Custom parser | pulldown-cmark 0.13.3 | CommonMark spec compliance, GFM tables, edge cases (nested lists, setext headings, link references) |
| YAML frontmatter detection | Regex `^---\n.*?\n---` | pulldown-cmark `ENABLE_YAML_STYLE_METADATA_BLOCKS` | Parser emits `MetadataBlock` events; handles edge cases like `---` inside code blocks |
| PlantUML rendering | Java FFI / JNI bridge | Subprocess call to `plantuml` CLI or `java -jar` | Mature tooling; pipe mode is fast enough; no need for JNI complexity |
| Mermaid rendering | Headless browser integration | `mmdc` CLI (mermaid-cli) | Handles Puppeteer/Chromium internally; configurable via puppeteer config |
| XML escaping | Manual string replacement | `fn escape_xml(s: &str)` utility with `&amp;`, `&lt;`, `&gt;`, `&quot;` | Simple but must be correct; cover all 5 XML special characters |
| Temporary file management | Manual `create`/`unlink` | `tempfile` crate | Auto-cleanup on drop; secure temp file creation |

**Key insight:** The converter's complexity is in the pulldown-cmark-to-Confluence-XML mapping, not in parsing or XML generation. The mapping is finite (23 Tag variants) and each has a known Confluence equivalent. The spike (02-01) should prove this mapping works for the 5 most complex elements: tables, code blocks, images, nested lists, and links.

## Common Pitfalls

### Pitfall 1: Undeclared XML Namespace Prefixes
**What goes wrong:** Using an XML library (quick-xml, xml-rs) to generate output causes errors because `ac:` and `ri:` prefixes have no `xmlns` declarations in the fragment.
**Why it happens:** Confluence storage format is technically malformed XML -- it uses namespace prefixes without declaring them, relying on the server to resolve them.
**How to avoid:** Use string-based output generation (`write!` / `push_str` to a `String`). Never parse the output fragment with a strict XML parser.
**Warning signs:** Errors mentioning "undeclared namespace prefix" or "unbound prefix" during output generation or validation.

### Pitfall 2: CDATA End Sequence in Code Blocks
**What goes wrong:** Code content containing the literal string `]]>` breaks CDATA sections.
**Why it happens:** `]]>` is the CDATA closing delimiter. If user code contains it, the XML becomes malformed.
**How to avoid:** Split CDATA sections at `]]>` boundaries: `<![CDATA[content with ]]]]><![CDATA[> more content]]>`. Alternatively, escape `]]>` as `]]]]><![CDATA[>` within the CDATA.
**Warning signs:** Code blocks containing `]]>` (rare but possible in XML/XSLT documentation).

### Pitfall 3: HTML Entities vs XML Escaping
**What goes wrong:** Generating `&nbsp;` or other HTML entities in Confluence storage format. Confluence expects XML, where only `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;` are predefined.
**Why it happens:** Copying HTML output patterns into the Confluence renderer.
**How to avoid:** Use only the 5 XML predefined entities. For non-breaking spaces, use the Unicode character `\u{00A0}` directly.
**Warning signs:** Confluence editor showing raw `&nbsp;` text or parse errors.

### Pitfall 4: First H1 Removal
**What goes wrong:** The first `<h1>` in the output duplicates the Confluence page title (which is stored separately).
**Why it happens:** Markdown files typically start with `# Title` which becomes `<h1>Title</h1>`, but Confluence pages already have a title field.
**How to avoid:** Skip the first `Heading(H1)` event during rendering (the Python converter does `re.sub(r"<h1>.*?</h1>\s*", "", storage_format, count=1)`).
**Warning signs:** Page title appearing twice when viewing the page.

### Pitfall 5: PlantUML Jar Path vs CLI
**What goes wrong:** Assuming PlantUML is always a JAR file when it might be installed as a CLI wrapper (e.g., `plantuml` from Homebrew).
**Why it happens:** The Python converter hardcodes `java -jar <path>` invocation.
**How to avoid:** Support two modes: (1) `plantuml` CLI command (Homebrew/package manager), (2) `java -jar <path>` (manual installation). Check config for which to use. On this machine, `/opt/homebrew/bin/plantuml` exists as a CLI wrapper.
**Warning signs:** "Java not found" errors when PlantUML is installed via package manager.

### Pitfall 6: Mermaid CLI Puppeteer Config
**What goes wrong:** `mmdc` fails with Chromium sandbox errors or timeouts on headless environments.
**Why it happens:** mermaid-cli requires Puppeteer/Chromium and may need `--puppeteerConfigFile` for sandbox configuration.
**How to avoid:** Make the puppeteer config path configurable (the Python converter has `mermaid_puppeteer_config` setting). Pass `--no-sandbox` via puppeteer config if needed.
**Warning signs:** `mmdc` exits with non-zero status and stderr mentioning "sandbox" or "chromium".

## Code Examples

Verified patterns from official sources:

### Confluence Storage Format: Complete Element Mapping

```xml
<!-- Source: https://confluence.atlassian.com/doc/confluence-storage-format-790796544.html -->

<!-- Headings: standard XHTML -->
<h1>Heading 1</h1>
<h2>Heading 2</h2>

<!-- Paragraphs: standard XHTML -->
<p>Paragraph text with <strong>bold</strong> and <em>italic</em>.</p>

<!-- Links: standard XHTML for external -->
<a href="https://example.com">Link text</a>

<!-- Images as attachments -->
<ac:image ac:alt="description" ac:width="100%">
  <ri:attachment ri:filename="image.png" />
</ac:image>

<!-- Code block macro -->
<ac:structured-macro ac:name="code">
  <ac:parameter ac:name="language">python</ac:parameter>
  <ac:plain-text-body><![CDATA[def hello():
    print("Hello")]]></ac:plain-text-body>
</ac:structured-macro>

<!-- Tables: standard XHTML -->
<table>
  <thead>
    <tr><th>Header 1</th><th>Header 2</th></tr>
  </thead>
  <tbody>
    <tr><td>Cell 1</td><td>Cell 2</td></tr>
  </tbody>
</table>

<!-- Unordered list -->
<ul>
  <li>Item 1</li>
  <li>Item 2</li>
</ul>

<!-- Ordered list -->
<ol>
  <li>Item 1</li>
  <li>Item 2</li>
</ol>

<!-- Blockquote: standard XHTML -->
<blockquote><p>Quoted text</p></blockquote>

<!-- Horizontal rule -->
<hr />

<!-- Inline code: standard XHTML -->
<code>inline code</code>

<!-- Strikethrough -->
<span style="text-decoration: line-through;">struck text</span>
```

### pulldown-cmark Parser Options for This Project

```rust
// Source: https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html
use pulldown_cmark::Options;

fn parser_options() -> Options {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);           // GFM tables (CONV-01)
    opts.insert(Options::ENABLE_STRIKETHROUGH);     // ~~text~~ support
    opts.insert(Options::ENABLE_TASKLISTS);         // [ ] / [x] checkboxes
    opts.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS); // CONV-02: frontmatter
    opts
}
```

### Expanding Code Blocks (Python Converter Behavior)

The Python converter wraps code blocks in an expand macro so they are collapsed by default:

```xml
<!-- Source: converter.py lines 219-227 -->
<ac:structured-macro ac:name="expand">
  <ac:parameter ac:name="title">Source</ac:parameter>
  <ac:rich-text-body>
    <ac:structured-macro ac:name="code">
      <ac:parameter ac:name="language">python</ac:parameter>
      <ac:plain-text-body><![CDATA[code here]]></ac:plain-text-body>
    </ac:structured-macro>
  </ac:rich-text-body>
</ac:structured-macro>
```

This behavior should be replicated in the Rust converter for parity.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| pulldown-cmark 0.9.x Tag enum | 0.13.x split Tag/TagEnd enums | v0.10.0 (2023) | `End(Tag)` became `End(TagEnd)` -- pattern matching syntax changed [VERIFIED: docs.rs] |
| md2conf Python library | Custom pulldown-cmark visitor | This phase | Eliminates Python runtime dependency |
| `java -jar plantuml.jar` only | `plantuml` CLI wrapper (Homebrew) | Ongoing | Must support both invocation styles |

**Deprecated/outdated:**
- pulldown-cmark < 0.10: `End(Tag)` variant was changed to `End(TagEnd)` -- examples from pre-2023 are syntactically wrong [VERIFIED: docs.rs]
- `md2conf` Python library: Still maintained but being replaced in this project by native Rust [ASSUMED]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | tempfile crate version 3.x is current and stable | Standard Stack | Low -- mature crate, unlikely to have breaking changes |
| A2 | Mermaid CLI (`mmdc`) requires file-based I/O, not stdin pipe | Architecture Patterns | Low -- can adapt to pipe mode if supported, but file-based is the documented approach |
| A3 | Confluence accepts `<blockquote>` as standard XHTML in storage format | Code Examples | Medium -- may need `ac:structured-macro name="quote"` instead; spike should verify |
| A4 | The Python converter's expand-macro wrapping of code blocks is desired behavior | Code Examples | Medium -- user may prefer non-collapsed code blocks; ask during implementation |
| A5 | pulldown-cmark `MetadataBlock` event correctly identifies YAML frontmatter without consuming document content | Architecture Patterns | Low -- documented feature, but spike should verify edge cases |

## Open Questions (RESOLVED)

1. **Blockquote rendering format**
   - What we know: Standard XHTML `<blockquote>` exists, but Confluence also has a quote macro `ac:structured-macro name="quote"`
   - What's unclear: Which format Confluence Cloud prefers / renders better
   - Recommendation: Spike (02-01) should test both; `<blockquote>` is simpler
   - RESOLVED: Plan 02-01 Task 2 implements `<blockquote>` (standard XHTML). Simpler approach chosen; spike validates rendering. See renderer.rs event mapping for `Start(BlockQuote)`.

2. **Code block collapse behavior**
   - What we know: Python converter wraps all code blocks in expand macros (collapsed by default)
   - What's unclear: Whether users want this behavior in the Rust version
   - Recommendation: Make it configurable (default: match Python behavior)
   - RESOLVED: Plan 02-01 Task 2 wraps all code blocks in `ac:structured-macro name="expand"` to match Python converter behavior. See `emit_code_block()` in renderer.rs.

3. **PlantUML HTTP server mode**
   - What we know: CONV-03 specifies "JAR path or HTTP server URL" as configurable
   - What's unclear: HTTP server API endpoint format for PlantUML server
   - Recommendation: Implement JAR/CLI mode first; HTTP mode can be added in 02-03 or deferred
   - RESOLVED: Plan 02-03 Task 1 implements CLI mode and JAR mode (detecting `.jar` suffix in `plantuml_path`). HTTP server mode deferred beyond Phase 2 scope — CLI/JAR covers CONV-03 requirements.

4. **Image width default**
   - What we know: Python converter sets `ac:width="100%"` on all images
   - What's unclear: Whether this is always desirable
   - Recommendation: Default to 100% for parity; make configurable later
   - RESOLVED: Plan 02-01 Task 2 and Plan 02-03 both use `ac:width="100%"` for parity with Python converter. See `emit_image()` in renderer.rs and placeholder replacement in mod.rs.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust compiler | Build | Yes | 1.89.0-nightly | -- |
| Java runtime | PlantUML JAR mode | Yes | OpenJDK 17.0.8 | Use `plantuml` CLI wrapper |
| PlantUML CLI | CONV-03 | Yes | 1.2026.2 | Fall back to `java -jar` |
| mermaid-cli (mmdc) | CONV-04 | Yes | 11.12.0 | -- |
| Puppeteer/Chromium | Mermaid rendering | Yes (via mmdc) | Bundled with mmdc | Puppeteer config for sandbox issues |

**Missing dependencies with no fallback:** None

**Missing dependencies with fallback:** None -- all dependencies available

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) + insta 1.x for snapshot testing |
| Config file | Cargo.toml `[dev-dependencies]` |
| Quick run command | `cargo test --lib converter` |
| Full suite command | `cargo test` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CONV-01 | Markdown headings/code/tables/links/images -> storage XML | unit + snapshot | `cargo test converter::renderer::tests -q` | Wave 0 |
| CONV-02 | YAML frontmatter stripped from output | unit | `cargo test converter::renderer::tests::test_frontmatter -q` | Wave 0 |
| CONV-03 | PlantUML fenced blocks -> SVG attachment | integration | `cargo test converter::diagrams::tests::test_plantuml -q` | Wave 0 |
| CONV-04 | Mermaid fenced blocks -> SVG attachment | integration | `cargo test converter::diagrams::tests::test_mermaid -q` | Wave 0 |
| CONV-05 | Mock Converter trait compiles and works | unit | `cargo test converter::tests::test_mock_converter -q` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib converter -q`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `src/converter/mod.rs` -- Converter trait definition + ConvertResult type
- [ ] `src/converter/renderer.rs` -- Renderer struct with unit tests + insta snapshots
- [ ] `src/converter/diagrams.rs` -- Diagram rendering with integration tests
- [ ] Test fixtures: sample Markdown files with headings, code, tables, links, images, frontmatter, PlantUML, Mermaid

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A (converter has no auth) |
| V3 Session Management | No | N/A |
| V4 Access Control | No | N/A |
| V5 Input Validation | Yes | XML-escape all text content; CDATA split for `]]>` in code blocks; validate diagram subprocess exit codes |
| V6 Cryptography | No | N/A |

### Known Threat Patterns for Markdown-to-XML Conversion

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| XML injection via Markdown content | Tampering | XML-escape all text nodes; CDATA wrapping for code blocks |
| Command injection via diagram content | Elevation of Privilege | PlantUML/Mermaid run in subprocess with no shell; content passed via stdin/file only |
| Path traversal via image references | Information Disclosure | Strip directory components from image filenames; only reference attachment names |
| Denial of service via large diagrams | Denial of Service | Timeout on subprocess calls (configurable, default 30s) |

## Sources

### Primary (HIGH confidence)
- [pulldown-cmark docs.rs](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/) -- Event enum, Tag enum, Options flags [VERIFIED]
- [Confluence Storage Format](https://confluence.atlassian.com/doc/confluence-storage-format-790796544.html) -- Official element reference [CITED]
- [Confluence Code Block Macro](https://confluence.atlassian.com/display/CONF57/Code+Block+Macro) -- ac:structured-macro code format [CITED]
- Existing Python converter (`src/confluence_agent/converter.py`) -- Reference implementation [VERIFIED: codebase]
- Existing Rust codebase (`src/`, `Cargo.toml`) -- Phase 1 patterns and dependencies [VERIFIED: codebase]

### Secondary (MEDIUM confidence)
- [pulldown-cmark GitHub](https://github.com/pulldown-cmark/pulldown-cmark) -- Usage patterns, custom renderer approach [CITED]
- [Confluence Storage Format Preview](https://thomasrohde.github.io/publish-confluence/preview/~thro/7._Confluence_storage_format.html) -- Additional element examples [CITED]
- cargo search results for crate versions [VERIFIED: 2026-04-10]

### Tertiary (LOW confidence)
- WebSearch results for quick-xml namespace handling -- general guidance, not verified against specific version [LOW]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- pulldown-cmark is the clear choice; versions verified via cargo search
- Architecture: HIGH -- event visitor pattern is well-documented; string-based output avoids XML namespace issues
- Pitfalls: HIGH -- namespace prefix issue is well-known; CDATA splitting is standard XML practice
- Diagram rendering: HIGH -- both PlantUML and mermaid-cli verified available on this machine

**Research date:** 2026-04-10
**Valid until:** 2026-05-10 (stable domain; pulldown-cmark API unlikely to change)
