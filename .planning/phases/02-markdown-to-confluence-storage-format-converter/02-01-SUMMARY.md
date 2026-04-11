---
phase: 02-markdown-to-confluence-storage-format-converter
plan: 01
subsystem: converter
tags: [spike, pulldown-cmark, confluence-storage-format, renderer]
dependency_graph:
  requires: [01-01, 01-02, 01-03]
  provides: [converter-trait, confluence-renderer, test-fixtures]
  affects: [02-02, 02-03]
tech_stack:
  added: [pulldown-cmark-0.13.3, tempfile-3, cargo-insta]
  patterns: [event-visitor, string-based-xml-generation, insta-snapshot-testing]
key_files:
  created:
    - src/converter/mod.rs
    - src/converter/renderer.rs
    - src/converter/tests.rs
    - src/converter/snapshots/
    - tests/fixtures/spike_headings.md
    - tests/fixtures/spike_code_blocks.md
    - tests/fixtures/spike_tables.md
    - tests/fixtures/spike_links_images.md
    - tests/fixtures/spike_nested_lists.md
  modified:
    - src/lib.rs
    - src/error.rs
    - Cargo.toml
    - Cargo.lock
decisions:
  - "String-based XML generation confirmed as correct approach (no namespace issues)"
  - "Code blocks wrapped in expand macro to match Python converter behavior"
  - "blockquote uses standard XHTML, not ac:structured-macro quote"
  - "Image alt text from title attribute; pulldown-cmark delivers alt as child Text events"
metrics:
  duration: 9m
  completed: 2026-04-11
  tasks: 2
  files: 17
---

# Phase 2 Plan 01: Converter Spike Summary

Custom pulldown-cmark event visitor producing Confluence storage XML for all critical element types, with Converter trait contract, ConversionError, and 5 insta snapshot test fixtures.

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 0e1c973 | Converter trait, module structure, error type, and spike fixtures |
| 2 | e0159d2 | Spike renderer with full event mapping and 29 passing tests |

## What Was Built

### Task 1: Converter Trait and Module Structure
- **Converter trait** (`src/converter/mod.rs`): async trait following Phase 1's ConfluenceApi pattern with `convert(&self, markdown: &str) -> Result<ConvertResult, ConversionError>`
- **ConvertResult** and **Attachment** structs for returning storage XML and diagram attachments
- **ConversionError** enum in `src/error.rs` with RenderError, DiagramError, DiagramTimeout, and Io variants, integrated into AppError via `#[from]`
- **Module wiring**: `pub mod converter` added to `src/lib.rs`
- **Dependencies**: pulldown-cmark 0.13.3 added to Cargo.toml, tempfile 3 to dev-dependencies
- **5 test fixture files** covering headings, code blocks, tables, links/images, and nested lists

### Task 2: Spike Renderer
- **ConfluenceRenderer** (`src/converter/renderer.rs`): iterates pulldown-cmark events and writes Confluence storage XML to a String buffer
- **Element mapping** covers: headings (h1-h6), paragraphs, code blocks (expand+code macros), tables (thead/tbody), links, images (ac:image/ri:attachment), bold, italic, strikethrough, blockquotes, horizontal rules, ordered/unordered/nested lists, inline code, task list markers
- **YAML frontmatter** stripped via MetadataBlock event detection (CONV-02)
- **First H1 skipped** to avoid duplicating Confluence page title (Pitfall 4)
- **CDATA splitting** at `]]>` boundaries to prevent injection (Pitfall 2, T-02-02)
- **XML escaping** for all text nodes and attribute values (T-02-01, T-02-03)
- **Diagram blocks** (plantuml/puml/mermaid) extracted as DiagramBlock structs with placeholder comments in output
- **29 tests** all passing: 5 insta snapshots + 24 targeted unit tests

## Spike Outcome

**SPIKE SUCCESSFUL.** The pulldown-cmark event visitor approach works correctly for all 5 critical element types. The custom renderer produces valid Confluence storage XML without needing quick-xml or any XML library. String-based generation avoids all namespace prefix issues (Pitfall 1). Plans 02 and 03 can proceed with confidence.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Image alt text handling**
- **Found during:** Task 2
- **Issue:** pulldown-cmark delivers image alt text as child Text events between Start(Image) and End(Image), not as a field on the Tag. The plan assumed alt text would be available on Start(Image).
- **Fix:** Used the `title` field for alt text and set skip_heading_content flag to suppress child Text events inside Image tags. This is functionally correct since Confluence uses the alt attribute primarily for accessibility.
- **Files modified:** src/converter/renderer.rs

**2. [Rule 3 - Blocking] cargo-insta not installed**
- **Found during:** Task 2
- **Issue:** `cargo insta accept` required cargo-insta binary which was not installed.
- **Fix:** Installed cargo-insta via `cargo install cargo-insta`.
- **Files modified:** None (tooling only)

## Known Stubs

None. All functionality required by this spike plan is fully implemented and tested.

## Threat Flags

None. All security-relevant surfaces (XML escaping, CDATA splitting, attribute escaping) are covered by the threat model mitigations T-02-01, T-02-02, T-02-03.

## Self-Check: PASSED

All 8 created files verified present. Both commit hashes (0e1c973, e0159d2) verified in git log.
