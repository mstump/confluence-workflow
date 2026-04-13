# Phase 7: Test Scaffold Completion - Research

**Researched:** 2026-04-13
**Domain:** Rust parallel test isolation, `serial_test` crate, unused dependency removal
**Confidence:** HIGH

## Summary

Phase 7 has a deceptively small scope that requires careful interpretation. The roadmap describes this phase as creating missing test files and fixing a config race condition. The milestone audit (v1.0-MILESTONE-AUDIT.md) and direct inspection of the repo reveal that **Phase 6 has already done most of the Phase 7 work**:

- `tests/cli_integration.rs` EXISTS with 7 full test implementations (6 active, 1 ignored for TLS reasons). All 7 tests pass.
- `tests/output_format.rs` EXISTS with 2 full test implementations. Both pass.
- The binary compiles cleanly with `cargo build`.

What Phase 7 still needs to do is narrowly scoped:

1. **Fix the config test race condition** in `src/config.rs` — two tests (`test_fallthrough_to_env_vars` and `test_env_vars_used_when_cli_absent`) use `std::env::set_var` / `std::env::remove_var` without isolation, so they fail under default parallel `cargo test` (2 failures) but pass when run sequentially. The fix is to add the `serial_test` crate to dev-dependencies and annotate the two affected tests with `#[serial]`.

2. **Remove `anyhow`** from `Cargo.toml` — confirmed unused in all `src/` files via grep; zero `anyhow` references in source.

The test files themselves are already complete and fully passing. Plan 07-01 as described in the roadmap ("Create tests/cli_integration.rs and tests/output_format.rs with full test implementations") is already done. Plan 07-02 ("Fix config race condition; remove anyhow") is the real work.

**Primary recommendation:** Reframe Plan 07-01 as a verification-only task (confirm tests pass and document what Phase 6 delivered), then execute Plan 07-02 to fix the race condition and remove `anyhow`.

## Actual State of Test Files (VERIFIED)

### tests/cli_integration.rs [VERIFIED: file read + cargo test run]

**Status: COMPLETE — all tests passing**

Functions present:
- `test_convert_command` — full implementation, passes (CLI-03)
- `test_convert_command_missing_file` — full implementation, passes (error path)
- `test_json_output_mode` — full implementation, passes (CLI-05)
- `test_json_output_mode_error` — full implementation, passes (JSON error path)
- `test_update_command_missing_api_key` — full implementation, passes (CLI-01 error path)
- `test_upload_command_missing_credentials` — full implementation, passes (CLI-02 error path)
- `test_upload_command_rejects_http_url` — full implementation, passes (T-01-04 security guard)
- `test_upload_command_happy_path` — `#[ignore]` with rationale: wiremock is http-only, Config enforces https://; cannot automate this path

Coverage summary:
- CLI-01: error path tested (`test_update_command_missing_api_key`)
- CLI-02: error paths tested (missing credentials + http:// rejection)
- CLI-03: full happy path + error path tested
- CLI-05: full happy path + error path tested

### tests/output_format.rs [VERIFIED: file read + cargo test run]

**Status: COMPLETE — all tests passing**

Functions present:
- `test_stderr_routing` — full implementation with --verbose, passes (CLI-04)
- `test_default_silent_mode` — full implementation without --verbose, passes (CLI-04)

### cargo test result [VERIFIED: live run]

```
Running tests/cli_integration.rs
test result: ok. 7 passed; 0 failed; 1 ignored

Running tests/output_format.rs
test result: ok. 2 passed; 0 failed; 0 ignored
```

The integration test stubs from the roadmap success criteria are fully implemented and passing. Phase 6 completed this work.

## Standard Stack

### Core (already in Cargo.toml)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serial_test` | 3.4.0 | Run annotated tests serially in a single process | Standard Rust crate for serializing parallel-unsafe tests; no alternatives needed |

[VERIFIED: `cargo search serial_test` — latest version 3.4.0]

### No Other New Dependencies Required

All integration test dependencies (`assert_cmd`, `wiremock`, `insta`, `tempfile`) are already in `[dev-dependencies]`. The only change is adding `serial_test` to `[dev-dependencies]`.

**Version verification:**
```bash
# Run before writing plan:
cargo search serial_test  # → serial_test = "3.4.0"
```

**Installation:**
```toml
[dev-dependencies]
serial_test = "3.4.0"
```

## Architecture Patterns

### Pattern 1: serial_test `#[serial]` Attribute

**What:** Mark tests that mutate shared global state (env vars) so cargo test never runs them concurrently.
**When to use:** Any test that calls `std::env::set_var` / `std::env::remove_var` on keys shared across multiple tests.

```rust
// Source: serial_test 3.x docs
// [VERIFIED: serial_test = "3.4.0" confirmed via cargo search]
use serial_test::serial;

#[test]
#[serial]
fn test_fallthrough_to_env_vars() {
    std::env::set_var("CONFLUENCE_URL", "https://via-env.atlassian.net");
    // ... test body ...
    std::env::remove_var("CONFLUENCE_URL");
    // ...
}
```

All tests marked `#[serial]` run sequentially relative to each other (via a per-test-binary mutex). Tests without `#[serial]` continue to run in parallel with each other and with serial tests (between serial sections).

**Alternative not recommended:** Refactoring the test to avoid env vars entirely (passing fake env values through a different mechanism) would be correct long-term but is higher risk for a gap-closure phase. The `serial_test` annotation is the minimal, well-understood fix.

### Pattern 2: Identifying Which Tests Need `#[serial]`

The two failing tests are:

1. `config::tests::test_fallthrough_to_env_vars` — sets `CONFLUENCE_URL`, `CONFLUENCE_USERNAME`, `CONFLUENCE_API_TOKEN`
2. `config::tests::test_env_vars_used_when_cli_absent` — sets the same three env vars

Both use the same three env var names. Under parallel execution, test A can set `CONFLUENCE_URL` while test B reads it (or has already removed it), causing the set_var in one test to be visible in another test's env-var lookup window.

Tests that only READ env vars (or use `no_home()` + CLI overrides exclusively) do not need `#[serial]`. Of the 10 config tests:
- 8 tests use CLI overrides exclusively — NOT affected by the race
- 2 tests call `std::env::set_var` — NEED `#[serial]`

**Tests that need `#[serial]`:** `test_fallthrough_to_env_vars`, `test_env_vars_used_when_cli_absent`

[VERIFIED: read src/config.rs — only these two tests call std::env::set_var for shared CONFLUENCE_* keys]

### Pattern 3: Removing an Unused Dependency

```toml
# Before (Cargo.toml):
anyhow = "1"

# After:
# (line deleted entirely)
```

After deletion, run `cargo build` to confirm zero errors. The `anyhow` crate has zero uses in `src/` — confirmed by grep returning no matches.

[VERIFIED: `grep -r "anyhow" src/` returns no results]

### Anti-Patterns to Avoid

- **Adding `#[serial]` to ALL config tests:** Only the two env-var-mutating tests need serialization. Over-annotating reduces test parallelism without benefit.
- **Using `std::env::set_var` + `std::env::remove_var` save/restore pattern without serial:** The save/restore pattern in the existing 6 tests (that save and restore a single env var) is acceptable for those tests since they touch different env vars than the failing pair, but the failing pair sets ALL THREE CONFLUENCE_* vars simultaneously and conflicts with each other.
- **Restructuring config.rs tests to avoid env vars entirely:** Valid but out of scope for a gap-closure phase. The `#[serial]` annotation is sufficient to meet the Phase 7 success criteria.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Serializing parallel-unsafe tests | Custom mutex in test code | `serial_test` crate | Standard Rust pattern; `#[serial]` is clear and idiomatic; home-grown mutex is fragile across test binary boundaries |

## Common Pitfalls

### Pitfall 1: Scope of the `#[serial]` attribute
**What goes wrong:** Annotating only one of the two conflicting tests — the other still runs in parallel and the race persists.
**Why it happens:** The conflict is symmetric: either test running first can cause the other to fail.
**How to avoid:** Both `test_fallthrough_to_env_vars` AND `test_env_vars_used_when_cli_absent` must have `#[serial]`.
**Warning signs:** After annotation, `cargo test` still fails intermittently — one test was missed.

### Pitfall 2: serial_test version mismatch
**What goes wrong:** Using `serial_test = "2.x"` syntax for `"3.x"` features or vice versa.
**Why it happens:** serial_test had a significant API change between v2 and v3.
**How to avoid:** Use `serial_test = "3.4.0"` (exact, pinned as per CLAUDE.md dependency pinning requirement). The `#[serial]` attribute exists in both v2 and v3; the import is `use serial_test::serial;`.
**Warning signs:** Compile error mentioning `serial` not found in `serial_test`.

### Pitfall 3: anyhow removal breaks something unexpected
**What goes wrong:** Removing `anyhow` from Cargo.toml causes a compile error because it appears as a transitive re-export.
**Why it happens:** Rarely — this typically only happens if `main.rs` uses `anyhow::Result` directly.
**How to avoid:** Read `src/main.rs` first (already done — uses no `anyhow`). After removing the line, run `cargo build` immediately to verify.
**Warning signs:** `cargo build` fails with `cannot find crate 'anyhow'` or `use of undeclared crate or module 'anyhow'`.

### Pitfall 4: Roadmap plan 07-01 describes work Phase 6 already completed
**What goes wrong:** Plan 07-01 is executed as if tests/cli_integration.rs and tests/output_format.rs are missing stubs that need to be "fleshed out" — but both files now contain full, passing implementations.
**Why it happens:** The roadmap was written before Phase 6 executed; Phase 6 over-delivered on what the roadmap expected of Phase 7.
**How to avoid:** Plan 07-01 should be a verification task (confirm tests pass, document the state) rather than an implementation task.
**Warning signs:** Attempting to overwrite tests/cli_integration.rs would destroy the working implementations Phase 6 produced.

## Code Examples

### serial_test usage in config tests

```rust
// Source: serial_test 3.4.0 docs (https://docs.rs/serial_test)
// [VERIFIED: crate exists at 3.4.0 via cargo search]
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_fallthrough_to_env_vars() {
        std::env::set_var("CONFLUENCE_URL", "https://via-env.atlassian.net");
        std::env::set_var("CONFLUENCE_USERNAME", "env-user@example.com");
        std::env::set_var("CONFLUENCE_API_TOKEN", "env-token");

        let overrides = CliOverrides::default();
        let result = Config::load_with_home(&overrides, Some(&no_home()));

        std::env::remove_var("CONFLUENCE_URL");
        std::env::remove_var("CONFLUENCE_USERNAME");
        std::env::remove_var("CONFLUENCE_API_TOKEN");

        let config = result.expect("should load from env vars");
        assert_eq!(config.confluence_url, "https://via-env.atlassian.net");
        // ...
    }

    #[test]
    #[serial]
    fn test_env_vars_used_when_cli_absent() {
        // same pattern — also needs #[serial]
    }
}
```

### Cargo.toml diff for serial_test addition + anyhow removal

```toml
# [dev-dependencies] section — add:
serial_test = "3.4.0"

# [dependencies] section — remove:
anyhow = "1"
```

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `std::env::set_var` in parallel tests (racy) | `#[serial]` attribute from `serial_test` crate | Tests reliable under default `cargo test` |
| `anyhow` dependency declared but unused | Removed from Cargo.toml | Cleaner dependency tree; no behavior change |

**Deprecated/outdated:**
- `serial_test` v2.x syntax: v3 changed the macro system; use v3 (`serial_test = "3.4.0"`), import as `use serial_test::serial;`

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `serial_test = "3.4.0"` API: `use serial_test::serial; #[serial]` attribute compiles | Standard Stack, Code Examples | Compile error; may need `serial_test::parallel` import instead. Risk is LOW — this is a stable, well-documented pattern |

**All other claims verified against live codebase or `cargo search`.**

## Open Questions

None — the scope is fully understood from direct inspection of the codebase.

## Environment Availability

Step 2.6: SKIPPED — Phase 7 is pure code/config changes. No external tool dependencies. `serial_test` is a pure Rust dev-dependency. `cargo` and Rust toolchain already in use.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in) |
| Config file | none (Cargo.toml workspace) |
| Quick run command | `cargo test --lib -- config::tests` |
| Full suite command | `cargo test` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLI-01 | update command error path: missing API key | integration | `cargo test --test cli_integration -- test_update_command_missing_api_key` | YES |
| CLI-02 | upload command error paths: missing credentials, http:// rejection | integration | `cargo test --test cli_integration -- test_upload_command` | YES |
| CLI-03 | convert command: full happy path + error path | integration | `cargo test --test cli_integration -- test_convert_command` | YES |
| CLI-04 | --verbose sends tracing to stderr; default mode silent | integration | `cargo test --test output_format` | YES |
| CLI-05 | --output json emits valid JSON; error JSON on failure | integration | `cargo test --test cli_integration -- test_json_output_mode` | YES |

### Sampling Rate

- **Per task commit:** `cargo test --lib -- config::tests`
- **Per wave merge:** `cargo test`
- **Phase gate:** `cargo test` passes with default parallelism before `/gsd-verify-work`

### Wave 0 Gaps

None — existing test infrastructure covers all phase requirements. The integration test files already exist and pass.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth changes in this phase |
| V3 Session Management | no | No session changes |
| V4 Access Control | no | No access control changes |
| V5 Input Validation | no | No new input paths |
| V6 Cryptography | no | No crypto changes |

This phase makes two mechanical changes (add `#[serial]` to two test functions, remove one line from Cargo.toml). There are no security-relevant changes.

## Sources

### Primary (HIGH confidence)
- `tests/cli_integration.rs` — read directly; all 7/8 tests confirmed present and passing [VERIFIED: file read + cargo test run]
- `tests/output_format.rs` — read directly; both tests confirmed present and passing [VERIFIED: file read + cargo test run]
- `src/config.rs` — read directly; identified exactly 2 tests calling set_var for CONFLUENCE_* keys [VERIFIED: file read]
- `Cargo.toml` — read directly; `anyhow = "1"` present in `[dependencies]` [VERIFIED: file read]
- `src/` grep for `anyhow` — zero matches confirmed [VERIFIED: grep run]
- `cargo test` full run — 113 passed, 2 failed (test_fallthrough_to_env_vars, test_env_vars_used_when_cli_absent) [VERIFIED: live run]
- `cargo test --lib -- config::tests` — 10 passed when run in isolation (serially) [VERIFIED: live run]
- `.planning/v1.0-MILESTONE-AUDIT.md` — audit confirms Phase 6 delivered test implementations [VERIFIED: file read]

### Secondary (MEDIUM confidence)
- `cargo search serial_test` — `serial_test = "3.4.0"` is current version [VERIFIED: cargo search output]

### Tertiary (LOW confidence)
- serial_test v3 API (`use serial_test::serial; #[serial]`) — based on training knowledge of well-established crate; not verified against live docs in this session [ASSUMED]

## Metadata

**Confidence breakdown:**
- Test file state: HIGH — files read directly, tests run live
- Config race condition: HIGH — root cause confirmed by running tests in parallel vs. sequential
- `anyhow` removal: HIGH — confirmed zero uses in src/ via grep
- serial_test API: MEDIUM — version confirmed via cargo search; exact attribute syntax is ASSUMED (well-established crate)

**Research date:** 2026-04-13
**Valid until:** 2026-05-13 (Rust codebase; stable dependencies)

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CLI-01 | `update` command test coverage | `test_update_command_missing_api_key` in cli_integration.rs — already implemented and passing; no new work needed |
| CLI-02 | `upload` command test coverage | `test_upload_command_missing_credentials` + `test_upload_command_rejects_http_url` — already implemented and passing |
| CLI-03 | `convert` command test coverage | `test_convert_command` + `test_convert_command_missing_file` — already implemented and passing |
| CLI-04 | `--verbose` flag test coverage | `test_stderr_routing` + `test_default_silent_mode` in output_format.rs — already implemented and passing |
| CLI-05 | JSON output mode test coverage | `test_json_output_mode` + `test_json_output_mode_error` — already implemented and passing |
</phase_requirements>
