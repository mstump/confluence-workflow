---
phase: 02-markdown-to-confluence-storage-format-converter
reviewed: 2026-04-10T00:00:00Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - src/converter/mod.rs
  - src/converter/renderer.rs
  - src/converter/tests.rs
  - src/lib.rs
  - src/error.rs
  - src/config.rs
  - src/converter/diagrams.rs
  - Cargo.toml
findings:
  critical: 0
  warning: 5
  info: 3
  total: 8
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-04-10T00:00:00Z
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

This phase implements the Markdown-to-Confluence-storage-format converter. The core
architecture is solid: a pulldown-cmark event-driven renderer (`ConfluenceRenderer`),
async subprocess-based diagram rendering for PlantUML and Mermaid, and a clean
`Converter` trait with a `MarkdownConverter` implementation. Error handling is
generally thorough and structured output types are appropriate.

Five warnings were found, the most significant being a state reuse bug in the
renderer that can corrupt heading suppression when images appear inside headings,
a silently discarded stdin write error in the PlantUML subprocess path that can
produce corrupt diagrams without any error signal, and a temp file leak in the
Mermaid renderer for early-exit error paths.

---

## Warnings

### WR-01: `skip_heading_content` flag dual-used for H1 suppression and image alt — can corrupt heading state

**File:** `src/converter/renderer.rs:276-279`

**Issue:** The boolean field `skip_heading_content` is repurposed to suppress text
events inside image tags (alt text collection) in addition to its primary role of
suppressing the first H1 heading. At line 276, `Start(Tag::Image)` sets
`renderer.skip_heading_content = true`, and `End(TagEnd::Image)` at line 279 resets
it to `false`.

If a document contains an image inside a heading — e.g., `## ![logo](logo.png) Title`
— the `End(TagEnd::Image)` event fires while still inside the heading, resetting
`skip_heading_content` to `false`. Subsequent text in that heading will then render
when it should have been suppressed (if this was the first H1) or the image cleanup
will fight with whatever the heading suppress state expected.

More concretely: if a `# ![img](x.png)` heading is the first H1, the sequence is:
1. `Start(Tag::Heading { level: 1 })` — sets `skip_heading_content = true`
2. `Start(Tag::Image)` — sets `skip_heading_content = true` (no-op here)
3. `End(TagEnd::Image)` — resets `skip_heading_content = false` (WRONG: still in H1)
4. `End(TagEnd::Heading(1))` — checks `skip_heading_content`, finds `false`, emits `</h1>` with no opening `<h1>`

This produces a dangling `</h1>` in the output for any first-H1 that contains an image.

**Fix:** Use a separate flag (or a counter) for the image alt-suppression state.

```rust
// Add to ConfluenceRenderer struct:
in_image: bool,

// In new():
in_image: false,

// In Start(Tag::Image):
renderer.in_image = true;

// In End(TagEnd::Image):
renderer.in_image = false;

// In Event::Text, Event::Code, Event::SoftBreak, Event::HardBreak:
// Change guard condition from `renderer.skip_heading_content` to
// `renderer.skip_heading_content || renderer.in_image`
```

---

### WR-02: Stdin write error silently swallowed in PlantUML renderer — can produce corrupt diagrams

**File:** `src/converter/diagrams.rs:38-41`

**Issue:** The stdin write to the PlantUML subprocess is performed in a detached
`tokio::spawn` task with the result discarded:

```rust
tokio::spawn(async move {
    let _ = stdin.write_all(&content_bytes).await;
    drop(stdin);
});
```

If `write_all` fails (e.g., the process exits early and the pipe closes, or a write
error occurs), the error is completely swallowed. The `child.wait_with_output()` call
may then see a partial or empty stdin, yet PlantUML might still exit with status 0
if it can produce output from partial input (e.g., for short diagrams). The caller
gets `Ok(svg_bytes)` for what is actually corrupt or incomplete output.

**Fix:** Either join the write task before waiting for output, or use the non-async
approach of writing to stdin directly before calling `wait_with_output`. The
simplest correct pattern:

```rust
// Instead of tokio::spawn, write synchronously then close stdin:
if let Some(mut stdin) = child.stdin.take() {
    stdin.write_all(content.as_bytes()).await.map_err(|e| ConversionError::DiagramError {
        diagram_type: "plantuml".to_string(),
        message: format!("Failed to write to PlantUML stdin: {e}"),
    })?;
    // stdin drop closes the pipe
}

let output = tokio::time::timeout(
    Duration::from_secs(config.timeout_secs),
    child.wait_with_output(),
)
// ... rest unchanged
```

Note: writing inline before `wait_with_output` is safe because PlantUML reads stdin
in `-pipe` mode and produces output only after EOF. For very large diagrams consider
a join approach, but this is fine for typical use.

---

### WR-03: Temp file leak in `render_mermaid` on early-exit error paths

**File:** `src/converter/diagrams.rs:96-150`

**Issue:** The mermaid output SVG is written to `input_file.path().with_extension("svg")`
(a sibling path of the `NamedTempFile`). The `NamedTempFile` for the input `.mmd`
file is automatically cleaned up on drop, but the output `.svg` path is a separate
file that must be manually removed.

The code attempts cleanup at line 143:
```rust
let _ = std::fs::remove_file(&output_path);
```

However, there are two early-return error paths that skip this cleanup:

1. `std::fs::read` failing at line 137 — returns `Err` without removing `output_path`
2. `svg_bytes.is_empty()` at line 145 — returns `Err` without removing `output_path`

In either case, a `.svg` temp file is left on disk. This is a resource leak that
accumulates with repeated diagram rendering failures.

**Fix:** Ensure cleanup before each early return, or restructure to use a scope
guard / defer pattern:

```rust
let svg_bytes = std::fs::read(&output_path).map_err(|e| {
    let _ = std::fs::remove_file(&output_path); // clean up before returning
    ConversionError::DiagramError {
        diagram_type: "mermaid".to_string(),
        message: format!("Failed to read SVG output: {e}"),
    }
})?;

let _ = std::fs::remove_file(&output_path);

if svg_bytes.is_empty() {
    return Err(ConversionError::DiagramError {
        diagram_type: "mermaid".to_string(),
        message: "mmdc produced empty SVG output".to_string(),
    });
}
```

---

### WR-04: `ac:width="100%"` is not a valid Confluence `ac:image` attribute

**File:** `src/converter/mod.rs:79` and `src/converter/renderer.rs:270`

**Issue:** Both the diagram placeholder replacement (mod.rs:79) and the image
rendering (renderer.rs:270) emit `ac:width="100%"` on `<ac:image>` elements:

```xml
<ac:image ac:alt="..." ac:width="100%">...</ac:image>
```

The Confluence storage format `ac:image` macro accepts `ac:width` as a numeric
pixel value (integer), not a percentage string. A value of `"100%"` is not a valid
integer and will cause Confluence to either silently ignore the attribute or reject
the page content, depending on the Confluence version.

**Fix:** Either remove `ac:width` entirely (Confluence will use the image's natural
size) or use a numeric pixel value. Alternatively, use the `ac:thumbnail` parameter
for a preview. The safest change is to drop the attribute:

```xml
<!-- diagram images -->
<ac:image ac:alt="{kind} diagram"><ri:attachment ri:filename="{filename}" /></ac:image>

<!-- inline images -->
<ac:image ac:alt="{alt}"><ri:attachment ri:filename="{filename}" /></ac:image>
```

---

### WR-05: Integration tests use relative `std::fs::read_to_string` paths — fragile

**File:** `src/converter/tests.rs:144, 167, 200`

**Issue:** Three integration tests read fixture files using relative paths:

```rust
let md = std::fs::read_to_string("tests/fixtures/plantuml_diagram.md").unwrap();
let md = std::fs::read_to_string("tests/fixtures/mermaid_diagram.md").unwrap();
let md = std::fs::read_to_string("tests/fixtures/plantuml_diagram.md").unwrap();
```

Relative paths in `std::fs::read_to_string` resolve against the process working
directory at runtime, not the file's location. This works when `cargo test` is
invoked from the crate root but will panic with `unwrap()` if the working directory
is different (e.g., a CI system with a non-standard CWD, or workspace-level test
invocation). Other tests in the same file use `include_str!` which is resolved at
compile time and is immune to this problem.

**Fix:** Use `include_str!` for consistency with the rest of the test suite:

```rust
let md = include_str!("../../tests/fixtures/plantuml_diagram.md");
```

Note that the integration tests that skip when the binary is not installed cannot
use `include_str!` for the binary-check branch, but they can still use it for the
fixture content itself.

---

## Info

### IN-01: Dependency versions not pinned to exact patch versions (CLAUDE.md convention)

**File:** `Cargo.toml:12-31`

**Issue:** The project's CLAUDE.md instructs to "always pin dependency versions" with
exact versions (e.g., `package==1.2.3`). Cargo.toml uses minor-version ranges for all
dependencies:

```toml
tokio = { version = "1.51", ... }
reqwest = { version = "0.13", ... }
pulldown-cmark = { version = "0.13", ... }
```

These allow any patch release to be selected by Cargo, which violates the pinning
convention established for this project.

**Fix:** Lock to exact versions using `=` in Cargo.toml:

```toml
tokio = { version = "=1.51.0", ... }
reqwest = { version = "=0.13.x", ... }
```

Alternatively, rely on `Cargo.lock` being committed (which Cargo does for binaries
by default) — this provides reproducibility without cluttering `Cargo.toml`. Confirm
that `Cargo.lock` is committed to the repository.

---

### IN-02: `unwrap()` on `list_stack.pop()` replaced with silent fallback — masks malformed events

**File:** `src/converter/renderer.rs:159`

**Issue:** When a `TagEnd::List` event fires, the code does:

```rust
let ordered = renderer.list_stack.pop().unwrap_or(false);
```

If the event stream is malformed (more `End(List)` events than `Start(List)`) the
stack will be empty and `unwrap_or(false)` silently emits `</ul>` regardless of
the actual list type. This produces mismatched HTML tags in the output without any
diagnostic. While pulldown-cmark guarantees balanced events for valid Markdown,
using `unwrap_or` hides potential bugs in the renderer logic itself during development.

**Fix:** Consider `debug_assert!` to catch this in test builds while keeping the
`unwrap_or` fallback in release for robustness:

```rust
debug_assert!(!renderer.list_stack.is_empty(), "list_stack underflow on TagEnd::List");
let ordered = renderer.list_stack.pop().unwrap_or(false);
```

---

### IN-03: Image filename extraction does not strip query strings or fragments

**File:** `src/converter/renderer.rs:254-258`

**Issue:** Image URLs like `image.png?v=2` or `image.png#anchor` are split by `/`
to extract the filename, but the query string and fragment are not stripped:

```rust
let filename = dest_url.rsplit('/').next().unwrap_or(&dest_url);
```

For `https://example.com/images/logo.png?v=2`, `rsplit('/').next()` yields
`"logo.png?v=2"`, which is then used as `ri:filename`. Confluence will not match
this to an attachment named `logo.png` and the image will break.

This only affects external URLs with query strings; bare filenames and paths without
query strings are unaffected. URLs from the same Confluence instance are unlikely to
have this form, but it is a latent correctness issue.

**Fix:** Strip query and fragment after extracting the filename:

```rust
let raw = dest_url.rsplit('/').next().unwrap_or(&dest_url);
let filename = raw.split('?').next().unwrap_or(raw)
                  .split('#').next().unwrap_or(raw);
```

---

_Reviewed: 2026-04-10T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
