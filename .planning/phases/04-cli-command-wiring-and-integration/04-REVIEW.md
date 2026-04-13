---
phase: 04
status: issues-found
critical: 0
high: 2
medium: 2
low: 1
---

# Phase 04: Code Review Report

**Reviewed:** 2026-04-13T00:00:00Z
**Depth:** standard
**Files Reviewed:** 3
**Status:** issues_found

## Summary

Three files were reviewed: `src/cli.rs`, `src/lib.rs`, and `src/main.rs`. The implementation correctly wires all three CLI commands through the full pipeline. The `CommandResult` enum and JSON output helpers are well-structured. Error handling uses `?` propagation consistently and no panicking `unwrap()` calls appear in the production paths.

Two high-severity correctness bugs were found: a path traversal vulnerability in the `convert` command where attachment filenames derived from converter output are joined directly onto the user-supplied output directory without sanitisation; and a `RUST_LOG` environment variable being silently ignored due to `EnvFilter::new()` overwriting it unconditionally. Two medium issues round out the findings: an unused import of the `ConfluenceApi` trait in `lib.rs` (likely a lint suppression candidate), and a spec inconsistency where verbose comment detail in Human mode is emitted with `eprintln!` instead of being routed through the tracing subscriber.

---

## High Issues

### HR-01: Path traversal in Convert command via attachment filename

**File:** `src/lib.rs:221`

**Issue:** The `convert` command writes each attachment to disk by joining the user-supplied `output_dir` with `att.filename` directly:

```rust
let att_path = output_dir.join(&att.filename);
std::fs::write(&att_path, &att.content).map_err(AppError::Io)?;
```

`att.filename` is currently always `diagram_N.svg` (a simple basename generated in `converter/mod.rs:60`), but `Attachment::filename` is a public `String` field with no invariant preventing a caller or future converter implementation from supplying a name such as `../../etc/cron.daily/evil` or `../../../home/user/.ssh/authorized_keys`. Because `PathBuf::join` resolves `..` components normally, any filename containing path separators or `..` segments would escape the intended output directory.

The threat model entry T-04-01 acknowledges this boundary but dismisses the risk on the basis that the converter only produces simple names today. That reasoning does not hold as a long-term defence — the converter module is independently extensible, and the `Attachment` struct carries no validation contract.

**Fix:** Validate that each `att.filename` is a plain filename (no path separators, no `..`) before writing, and return an `AppError::Io` or a new `AppError::Conversion` if the check fails:

```rust
use std::path::Path;

for att in &convert_result.attachments {
    // Reject filenames that contain path components
    let bare = Path::new(&att.filename);
    if bare.components().count() != 1
        || bare.file_name().map_or(true, |n| n != bare.as_os_str())
    {
        return Err(AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("attachment filename contains path separators: {}", att.filename),
        )));
    }
    let att_path = output_dir.join(&att.filename);
    std::fs::write(&att_path, &att.content).map_err(AppError::Io)?;
    files.push(att_path.to_string_lossy().to_string());
}
```

This is also applicable to the `update` and `upload` commands where `client.upload_attachment()` receives `att.filename` as the remote filename sent to the Confluence API. While the server-side impact differs, sending a filename with `../` components could cause unexpected behaviour depending on the Confluence server's multipart handling.

---

### HR-02: `RUST_LOG` environment variable is silently ignored

**File:** `src/main.rs:14`

**Issue:** `init_tracing` constructs the filter using `EnvFilter::new(level)` where `level` is either `"debug"` or `"warn"`:

```rust
let level = if verbose { "debug" } else { "warn" };
fmt()
    .with_env_filter(EnvFilter::new(level))
    .with_writer(std::io::stderr)
    .init();
```

`EnvFilter::new(directive)` parses its argument as an override directive and does **not** consult `RUST_LOG`. This means `RUST_LOG=confluence_agent=trace cargo run -- convert …` produces no additional output, contrary to the project's planning documentation (`.planning/research/ARCHITECTURE.md:774`, `INTEGRATIONS.md:81`, `03-VALIDATION.md:72`) which explicitly states `RUST_LOG` is supported and instructs developers to use it for debugging.

A developer following the documented procedure `RUST_LOG=trace cargo run -- update doc.md <url>` to verify that the API key is absent from log output (03-VALIDATION.md:72) will see `warn`-level output and incorrectly conclude the check passed, because no log lines are produced at all.

**Fix:** Give `RUST_LOG` precedence and fall back to the verbose-controlled default:

```rust
fn init_tracing(verbose: bool) {
    let default_level = if verbose { "debug" } else { "warn" };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_level));
    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
```

`try_from_default_env()` reads `RUST_LOG`; if unset or invalid it falls back to the hardcoded default. This preserves the D-08 default-level behaviour while restoring the documented `RUST_LOG` override capability.

---

## Medium Issues

### MR-01: Unused import `ConfluenceApi` in lib.rs

**File:** `src/lib.rs:14`

**Issue:** The import line is:

```rust
use confluence::{extract_page_id, ConfluenceApi, ConfluenceClient};
```

`ConfluenceApi` is not referenced anywhere in `lib.rs` after this import. `ConfluenceClient` (concrete type) is used directly when constructing the client, and `update_page_with_retry` accepts `&dyn ConfluenceApi` — but the trait bound is resolved by the callee, not by the caller. Rust does not require the trait to be in scope to call a function that takes `&dyn TraitName`; coercion from `&ConfluenceClient` to `&dyn ConfluenceApi` is performed at the call site without needing the trait imported.

`cargo check` currently reports zero warnings, which suggests either the compiler is not emitting a warning here (because the trait is technically used as part of the coercion), or `#[allow(unused_imports)]` is active somewhere. Either way the import is at minimum non-obvious — it should be accompanied by a comment explaining why it is needed, or removed if the compiler confirms it is genuinely unused.

**Fix:** Run `cargo check 2>&1 | grep unused` after removing `ConfluenceApi` from the import. If the compiler accepts the removal, delete it. If coercion requires it in scope, add a comment:

```rust
// ConfluenceApi must be in scope for ConfluenceClient -> &dyn ConfluenceApi coercion
use confluence::{extract_page_id, ConfluenceApi, ConfluenceClient};
```

---

### MR-02: Verbose comment/file details emitted via `eprintln!` bypass the tracing subscriber

**File:** `src/main.rs:56-58`, `src/main.rs:67-69`

**Issue:** In Human output mode with `--verbose`, supplemental details (comment counts, converted file list) are printed using raw `eprintln!`:

```rust
if verbose {
    eprintln!("  Comments kept: {comments_kept}, dropped: {comments_dropped}");
}
// and
if verbose {
    for f in &files {
        eprintln!("  {f}");
    }
}
```

This is a correctness issue relative to D-06/D-07: these lines are outside the tracing subscriber and will always appear on stderr regardless of the `RUST_LOG` or filter level, and they carry no structured fields, span context, or timestamps. They also cannot be suppressed without removing `--verbose`. If a caller uses `--verbose` to enable tracing but wants to silence this raw output, there is no mechanism to do so.

The plan spec (D-04) says "on success: one line" to stdout in human mode, with no provision for raw multi-line stderr emission. This creates a silent API contract violation for scripted callers that parse stderr.

**Fix:** Route these through the tracing subscriber using `tracing::info!` or `tracing::debug!` so they are subject to the configured filter level and carry structured metadata. Alternatively, document explicitly that `--verbose` produces additional unstructured stderr lines beyond tracing output, and update D-04 accordingly.

```rust
// Replace raw eprintln! with tracing events
tracing::info!(kept = comments_kept, dropped = comments_dropped, "comment merge summary");
// and
for f in &files {
    tracing::info!(file = f, "converted file");
}
```

---

## Low Issues

### LR-01: `ConfluenceApi` import is not flagged by clippy — add dead_code lint note

**File:** `src/lib.rs:14`

**Issue:** This overlaps with MR-01 but from a maintainability angle: because `cargo check` produces no warning for this import, future maintainers have no compiler-enforced signal when it becomes genuinely unused. Consider adding `#![warn(unused_imports)]` at the crate root if it is not already present, to ensure this class of issue is surfaced automatically.

**Fix:** Verify `src/lib.rs` or `src/main.rs` does not carry `#![allow(unused_imports)]`. Add `#![warn(unused_imports)]` to `src/lib.rs` if it is absent from the crate root.

---

_Reviewed: 2026-04-13T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
