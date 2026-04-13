# Phase 6: Credential Waterfall Fix - Research

**Researched:** 2026-04-13
**Domain:** Rust CLI argument wiring (clap), credential waterfall plumbing
**Confidence:** HIGH

## Summary

This phase fixes a known design gap identified in the v1.0 milestone audit: `CliOverrides.anthropic_api_key` exists in `src/config.rs` but has no corresponding `--anthropic-api-key` CLI flag in `src/cli.rs`, and is hardwired to `None` in both the `update` and `upload` arms of `src/lib.rs` (lines 88 and 166). The waterfall infrastructure (`resolve_optional` in `Config::load`) already handles CLI > env > .env > ~/.claude/ precedence correctly -- the only missing pieces are (1) the clap field, (2) the wiring in `lib.rs`, and (3) fixing a test that exercises the wrong error path.

This is a surgical, low-risk change touching exactly three files with no new dependencies. The entire config waterfall mechanism (`resolve_required`, `resolve_optional`, `load_from_claude_config`) is already implemented and tested.

**Primary recommendation:** Add `--anthropic-api-key` to `Cli` struct, pass it through `CliOverrides` in both `update` and `upload` arms of `run()`, and fix `test_update_command_missing_api_key` to use an `https://` URL so the ANTHROPIC_API_KEY check fires before the HTTPS guard.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SCAF-02 | CLI binary accepts update/upload/convert subcommands; `--anthropic-api-key` flag supported | Add `anthropic_api_key: Option<String>` field with `#[arg(long, env = "ANTHROPIC_API_KEY")]` to `Cli` struct in cli.rs |
| SCAF-03 | Credentials loaded via waterfall: CLI flag > env var > .env > ~/.claude/; CLI flag functional for all credentials including Anthropic API key | Wire `cli.anthropic_api_key` into `CliOverrides` in both update and upload arms of lib.rs `run()` function; waterfall resolution already implemented in `Config::resolve_optional` |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- Run `cargo build` after Rust changes -- zero warnings required
- Run `uv run pytest` for Python tests (not relevant to this phase)
- Run `markdownlint --fix .` after markdown changes
- Pin dependency versions in pyproject.toml (not relevant -- no new deps)
- `thiserror` for structured errors (already in use)
- clap with derive feature for CLI parsing (already in use)

## Standard Stack

No new dependencies required. This phase uses only existing crate features.

### Core (already present)
| Library | Version | Purpose | Status |
|---------|---------|---------|--------|
| clap | 4.6 | CLI argument parsing with `#[derive(Parser)]` | Already in Cargo.toml [VERIFIED: Cargo.toml line 17] |
| dotenvy | 0.15 | .env file loading | Already in Cargo.toml [VERIFIED: Cargo.toml line 23] |
| dirs | 6.0 | Home directory resolution for ~/.claude/ | Already in Cargo.toml [VERIFIED: Cargo.toml line 22] |

### Testing (already present)
| Library | Version | Purpose | Status |
|---------|---------|---------|--------|
| assert_cmd | 2 | CLI binary integration testing | Already in dev-dependencies [VERIFIED: Cargo.toml line 33] |

**Installation:** None needed. Zero new dependencies.

## Architecture Patterns

### Current Credential Waterfall (Working Pattern)

The waterfall is already fully implemented in `src/config.rs`:

```
CLI override (CliOverrides field) → env var → .env (via dotenvy) → ~/.claude/settings.json
```

`Config::resolve_optional` and `Config::resolve_required` both implement this 4-tier waterfall. [VERIFIED: src/config.rs lines 144-196]

The Confluence credentials (`confluence_url`, `confluence_username`, `confluence_token`) already follow this pattern end-to-end: cli.rs field -> CliOverrides -> Config::load. [VERIFIED: src/cli.rs lines 19-27, src/lib.rs lines 84-88]

### The Gap (Exact Lines)

1. **cli.rs**: No `--anthropic-api-key` field in the `Cli` struct. [VERIFIED: src/cli.rs -- struct has confluence_url, confluence_username, confluence_token but no anthropic_api_key]
2. **lib.rs line 88**: `anthropic_api_key: None` in the Update arm's CliOverrides construction. [VERIFIED: src/lib.rs line 88]
3. **lib.rs line 166**: `anthropic_api_key: None` in the Upload arm's CliOverrides construction. [VERIFIED: src/lib.rs line 166]

### Fix Pattern (Follows Existing Convention)

The fix mirrors the existing `confluence_token` pattern exactly:

**cli.rs** -- add field to `Cli` struct:
```rust
/// Anthropic API key (for update command's LLM merge)
#[arg(long, env = "ANTHROPIC_API_KEY")]
pub anthropic_api_key: Option<String>,
```
[Pattern source: src/cli.rs line 27 -- confluence_token uses identical pattern] [VERIFIED]

**lib.rs** -- wire in both arms:
```rust
anthropic_api_key: cli.anthropic_api_key.clone(),  // was: None
```
[Pattern source: src/lib.rs line 87 -- confluence_api_token uses identical pattern] [VERIFIED]

Note: `cli.anthropic_api_key` needs `.clone()` because it is used in the Update arm and the `cli` struct is moved into the match. Since both Update and Upload construct CliOverrides before any other use of `cli` fields, a single `.clone()` on the Option suffices. Actually, looking at the code more carefully: `cli` is moved into `match cli.command`, so `cli.anthropic_api_key` is accessible in each arm because the match destructures `cli.command` while `cli.confluence_url` etc. are accessed as `cli.field_name`. This is the same pattern used for `cli.confluence_url`, `cli.confluence_username`, and `cli.confluence_token`. [VERIFIED: src/lib.rs lines 79-88]

### Test Fix Pattern

The existing test `test_update_command_missing_api_key` (tests/cli_integration.rs line 219) uses `http://localhost:19999` as the Confluence URL. The HTTPS guard in `config.rs` line 93 fires BEFORE the ANTHROPIC_API_KEY check in `lib.rs` line 93, so the test passes on the wrong error.

**Fix:** Change the test's `--confluence-url` to `https://localhost:19999`. This satisfies the HTTPS guard, and since ANTHROPIC_API_KEY is removed from env, the correct `ConfigError::Missing { name: "ANTHROPIC_API_KEY" }` error will fire... BUT wait -- `ANTHROPIC_API_KEY` is resolved via `resolve_optional` (not `resolve_required`), so `Config::load` will succeed with `anthropic_api_key: None`. The actual error fires at `lib.rs` line 93: `config.anthropic_api_key.clone().ok_or_else(...)` which produces `AppError::Config(ConfigError::Missing { name: "ANTHROPIC_API_KEY" })`.

So the fix is: change `http://localhost:19999` to `https://localhost:19999` and tighten the assertion to specifically check for "ANTHROPIC_API_KEY" in stderr (remove the loose `|| stderr.contains("Error")` fallback). [VERIFIED: tests/cli_integration.rs lines 219-251, src/lib.rs lines 92-97, src/config.rs lines 92-98]

Also update the page_url parameter from `http://localhost:19999/wiki/...` to `https://localhost:19999/wiki/...` for consistency. [VERIFIED: tests/cli_integration.rs line 231]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI arg parsing | Manual arg parsing | clap `#[arg(long, env)]` | Already in use; the `env` feature auto-reads env vars as fallback [VERIFIED: clap 4.6 in Cargo.toml] |
| .env loading | Manual file parsing | dotenvy (already called in Config::load) | Already integrated [VERIFIED: src/config.rs line 73] |
| Home dir resolution | Manual HOME reading | dirs crate (already used) | Cross-platform [VERIFIED: src/config.rs line 74] |

## Common Pitfalls

### Pitfall 1: clap `env` Attribute Conflicting with Waterfall
**What goes wrong:** Adding `env = "ANTHROPIC_API_KEY"` to the clap `#[arg]` means clap itself will read the env var and populate the field, bypassing the waterfall in `Config::resolve_optional`.
**Why it happens:** clap's `env` feature makes the CLI flag also read from the env var automatically.
**How to avoid:** This is actually fine in this codebase. Looking at the existing pattern: `confluence_token` has `#[arg(long, env = "CONFLUENCE_API_TOKEN")]`. When clap populates the field from the env var, it becomes `Some(value)` which then goes into `CliOverrides` and takes the CLI-override path in `resolve_required`/`resolve_optional`. The env var tier in the waterfall also reads it. The net effect is correct: the CLI flag value (or env var value via clap) takes highest precedence. The only subtle case: if a user sets ANTHROPIC_API_KEY in the environment, clap will populate the field, and it will be treated as a "CLI override" in the waterfall. This is the same behavior as the Confluence credentials and is acceptable. [VERIFIED: existing pattern in src/cli.rs lines 19-27]
**Warning signs:** None -- this is the established pattern.

### Pitfall 2: Test HTTPS Guard Order
**What goes wrong:** Test uses `http://` URL, HTTPS guard fires before the intended error check.
**Why it happens:** `Config::load` validates HTTPS before returning, so `anthropic_api_key` check in `lib.rs` never executes.
**How to avoid:** Use `https://` URL in test. The connection will never be attempted because the missing API key check fires first. [VERIFIED: src/config.rs lines 92-98, src/lib.rs lines 92-97]
**Warning signs:** Test assertion using `||` disjunction that matches on generic "Error" string.

### Pitfall 3: Forgetting the Upload Arm
**What goes wrong:** Only wiring `anthropic_api_key` in the Update arm, leaving Upload with `None`.
**Why it happens:** Upload does not currently use the API key (no LLM merge), but the `CliOverrides` struct should still be consistent.
**How to avoid:** Wire in both arms. Even though Upload does not validate the API key, the `CliOverrides` should faithfully represent what the user provided. Future features may need it. [VERIFIED: src/lib.rs lines 84-88, 162-166]

## Code Examples

### Exact Change 1: cli.rs -- Add Field
```rust
// Source: follows existing pattern at src/cli.rs line 27
// Add after confluence_token field:

/// Anthropic API key (for update command's LLM merge)
#[arg(long, env = "ANTHROPIC_API_KEY")]
pub anthropic_api_key: Option<String>,
```

### Exact Change 2: lib.rs -- Update arm (line 88)
```rust
// Source: src/lib.rs line 88
// Change from:
anthropic_api_key: None,
// Change to:
anthropic_api_key: cli.anthropic_api_key.clone(),
```

### Exact Change 3: lib.rs -- Upload arm (line 166)
```rust
// Source: src/lib.rs line 166
// Change from:
anthropic_api_key: None,
// Change to:
anthropic_api_key: cli.anthropic_api_key.clone(),
```

### Exact Change 4: test fix -- tests/cli_integration.rs
```rust
// Source: tests/cli_integration.rs line 223
// Change from:
.arg("http://localhost:19999")
// Change to:
.arg("https://localhost:19999")

// Source: tests/cli_integration.rs line 231
// Change from:
.arg("http://localhost:19999/wiki/spaces/TEST/pages/12345/Title")
// Change to:
.arg("https://localhost:19999/wiki/spaces/TEST/pages/12345/Title")

// Source: tests/cli_integration.rs lines 244-248
// Tighten assertion from:
stderr.contains("ANTHROPIC_API_KEY") || stderr.contains("Error")
// Change to:
stderr.contains("ANTHROPIC_API_KEY")
```

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) + assert_cmd 2 |
| Config file | Cargo.toml [dev-dependencies] |
| Quick run command | `cargo test --test cli_integration test_update_command_missing_api_key` |
| Full suite command | `cargo test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SCAF-02 | --anthropic-api-key flag accepted by CLI | integration | `cargo test --test cli_integration test_update_command_missing_api_key -x` | Yes (needs fix) |
| SCAF-03 | CLI flag value reaches Config via waterfall | unit | `cargo test --lib config::tests::test_load_from_cli_overrides -x` | Yes (already passes) |
| SCAF-03 | Missing API key produces correct error (not HTTPS error) | integration | `cargo test --test cli_integration test_update_command_missing_api_key -x` | Yes (needs fix) |

### Sampling Rate
- **Per task commit:** `cargo test --test cli_integration -x && cargo test --lib config::tests -x`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
None -- existing test infrastructure covers all phase requirements. The test file exists and the test function exists; only the test content needs fixing.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | API key passed via CLI flag or env var; never logged |
| V3 Session Management | no | N/A |
| V4 Access Control | no | N/A |
| V5 Input Validation | yes | clap handles type validation; empty string check in resolve_optional |
| V6 Cryptography | no | N/A |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| API key visible in process listing (`ps aux`) | Information Disclosure | Accept env var as primary method; CLI flag is opt-in convenience. Document risk in --help text. [ASSUMED] |
| API key logged to tracing output | Information Disclosure | The key passes through `CliOverrides` -> `Config` -> `AnthropicClient`. No tracing statements log the key value. [VERIFIED: src/config.rs, src/lib.rs -- no tracing of key values] |
| HTTPS guard bypass | Tampering | Config::load validates `https://` scheme before any API call. This phase does not modify that guard. [VERIFIED: src/config.rs lines 92-98] |

**Note on CLI key exposure:** Passing secrets via CLI flags makes them visible in `/proc/<pid>/cmdline` on Linux and `ps aux` output. This is an inherent limitation of CLI flags for secrets. The env var path is more secure. The `env = "ANTHROPIC_API_KEY"` annotation on the clap field means users can use the env var transparently without the CLI flag. This matches the pattern already established for `CONFLUENCE_API_TOKEN`. [ASSUMED -- standard security guidance for CLI tools]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | API key in process listing is acceptable risk for a CLI tool, matching confluence_token pattern | Security Domain | Low -- same pattern already in use for Confluence credentials |
| A2 | Document process-listing risk in --help text | Security Domain | Very low -- cosmetic; --help text is not in scope for this phase |

## Open Questions

None. The gap is fully characterized by the milestone audit, the fix pattern is established by existing code, and all three files are identified.

## Sources

### Primary (HIGH confidence)
- `src/cli.rs` -- current Cli struct, existing clap patterns
- `src/lib.rs` -- run() function, both Update and Upload arms with hardwired `None`
- `src/config.rs` -- CliOverrides struct, Config::load, resolve_optional, resolve_required
- `tests/cli_integration.rs` -- test_update_command_missing_api_key with wrong URL
- `.planning/v1.0-MILESTONE-AUDIT.md` -- gap identification for SCAF-02, SCAF-03
- `src/error.rs` -- ConfigError::Missing variant

### Secondary (MEDIUM confidence)
- None needed -- all findings are from direct codebase inspection

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- no new dependencies, all existing
- Architecture: HIGH -- fix pattern mirrors existing working code exactly
- Pitfalls: HIGH -- identified from direct code inspection, all verified

**Research date:** 2026-04-13
**Valid until:** 2026-05-13 (stable -- Rust codebase, no external API changes)
