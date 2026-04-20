---
phase: 02-markdown-to-confluence-storage-format-converter
fixed_at: 2026-04-11T00:59:41Z
review_path: .planning/phases/02-markdown-to-confluence-storage-format-converter/02-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 02: Code Review Fix Report

**Fixed at:** 2026-04-11T00:59:41Z
**Source review:** .planning/phases/02-markdown-to-confluence-storage-format-converter/02-REVIEW.md
**Iteration:** 1

**Summary:**

- Findings in scope: 5
- Fixed: 5
- Skipped: 0

## Fixed Issues

### WR-01: `skip_heading_content` flag dual-used for H1 suppression and image alt — can corrupt heading state

**Files modified:** `src/converter/renderer.rs`
**Commit:** 99b3e8f
**Applied fix:** Added a new `in_image: bool` field to `ConfluenceRenderer` (struct definition and `new()` initializer). Changed `Start(Tag::Image)` to set `renderer.in_image = true` and `End(TagEnd::Image)` to set `renderer.in_image = false`, leaving `skip_heading_content` exclusively for H1 suppression. Updated `Event::Text`, `Event::Code`, `Event::SoftBreak`, and `Event::HardBreak` guards to check `renderer.skip_heading_content || renderer.in_image` (or `&& !renderer.in_image` for the negated forms). Also removed the invalid `ac:width="100%"` attribute from the inline image output as part of this edit (WR-04 renderer.rs half).

---

### WR-02: Stdin write error silently swallowed in PlantUML renderer — can produce corrupt diagrams

**Files modified:** `src/converter/diagrams.rs`
**Commit:** e42771a
**Applied fix:** Replaced the `tokio::spawn` fire-and-forget pattern with an inline `if let Some(mut stdin) = child.stdin.take()` block that calls `stdin.write_all(content.as_bytes()).await` and propagates errors via `map_err(...)?`. Dropping `stdin` at end of the block closes the pipe and signals EOF to the PlantUML process. The `tokio::io::AsyncWriteExt` import was already present and continues to be used.

---

### WR-03: Temp file leak in `render_mermaid` on early-exit error paths

**Files modified:** `src/converter/diagrams.rs`
**Commit:** fedc090
**Applied fix:** Restructured the `std::fs::read(&output_path)` call to include cleanup in its `map_err` closure: `let _ = std::fs::remove_file(&output_path)` is called before constructing the error, ensuring the SVG temp file is deleted even when the read fails. The existing cleanup after a successful read and the `svg_bytes.is_empty()` check remain unchanged (the empty-bytes path returns an error after the successful read+cleanup, so no additional cleanup is needed there).

---

### WR-04: `ac:width="100%"` is not a valid Confluence `ac:image` attribute

**Files modified:** `src/converter/mod.rs`, `src/converter/renderer.rs`
**Commit:** a975720 (mod.rs); 99b3e8f (renderer.rs, applied as part of WR-01 commit)
**Applied fix:** Removed `ac:width="100%"` from both locations where `<ac:image>` elements are emitted. In `renderer.rs` line 272, the inline image format string was updated to omit the attribute. In `mod.rs` line 79, the diagram placeholder replacement format string was likewise updated. Confluence will use the image's natural size, which is the recommended safe approach.

---

### WR-05: Integration tests use relative `std::fs::read_to_string` paths — fragile

**Files modified:** `src/converter/tests.rs`
**Commit:** d562539
**Applied fix:** Replaced all three `std::fs::read_to_string("tests/fixtures/...")` calls with `include_str!("../../tests/fixtures/...")`. The fixture content is now embedded at compile time using the same pattern as other tests in the file. Updated the subsequent `converter.convert(&md)` calls to `converter.convert(md)` since `include_str!` returns `&str` directly rather than `String`. All three fixture files were confirmed to exist at the expected paths.

---

_Fixed: 2026-04-11T00:59:41Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
