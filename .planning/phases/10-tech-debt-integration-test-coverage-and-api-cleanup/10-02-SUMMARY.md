---
phase: 10-tech-debt-integration-test-coverage-and-api-cleanup
plan: 02
subsystem: config, converter, trait-boundary
tags: [api-cleanup, dead-code, D-04, converter-trait, IN-01, WR-02]
requires:
  - Phase 08 DiagramConfig waterfall (CLI → env → ~/.claude → default) in Config::load_with_home
  - Phase 10-01 happy-path integration tests in tests/cli_integration.rs (regression guard)
provides:
  - Smaller public API surface: `DiagramConfig::from_env`, `impl Default for DiagramConfig`, `impl Default for MarkdownConverter` all removed
  - Private `test_diagram_config()` helper in src/converter/tests.rs mirroring `config_with_defaults()` in src/converter/diagrams.rs
  - `test_converter_trait_object_invocation` lock-in test exercising `MarkdownConverter` via `&dyn Converter`
  - `## Converter Trait Exercised — Audit` section documenting the trait boundary call-chain
affects:
  - src/config.rs (deleted 22-line impl block + Default impl)
  - src/converter/mod.rs (deleted 5-line Default impl)
  - src/converter/tests.rs (added 10-line helper + 7 call-site rewrites + 28-line trait-object test)
  - src/converter/diagrams.rs (deleted 46-line `test_diagram_config_from_env` + leading comment)
tech-stack:
  added: []
  patterns:
    - Private test helper function pattern for struct literal construction (mirrors `config_with_defaults` in diagrams.rs)
    - Trait-object binding pattern `let x: &dyn Trait = &concrete` for locking in `impl Trait for Type` against silent removal
key-files:
  modified:
    - src/config.rs
    - src/converter/mod.rs
    - src/converter/tests.rs
    - src/converter/diagrams.rs
  created:
    - .planning/phases/10-tech-debt-integration-test-coverage-and-api-cleanup/10-02-SUMMARY.md
decisions:
  - D-04 (from Phase 10 CONTEXT): remove orphaned `DiagramConfig::from_env` / `Default` impls that bypass the Phase 08 waterfall
  - Lock-in test pattern: bind `MarkdownConverter` as `&dyn Converter` explicitly so the trait impl cannot be silently deleted without a compile failure
  - Refactor TDD: deletions + call-site rewrites applied atomically (single commit) because partial edits leave the tree uncompilable — no separate RED commit since this is a subtraction, not a new behavior
metrics:
  duration_min: ~15
  completed: 2026-04-20
  tasks_planned: 2
  tasks_completed: 2
  commits:
    - e2c6a18 refactor(10-02): remove dead API and rewrite 7 Default call sites (Task 1)
    - db15235 test(10-02): add trait-object lock-in unit test (Task 2)
---

# Phase 10 Plan 02: API Cleanup and Converter Trait Lock-in Summary

One-liner: Removed three orphaned public API items (`DiagramConfig::from_env`, `impl Default for DiagramConfig`, `impl Default for MarkdownConverter`) that bypassed the Phase 08 waterfall, rewrote all 7 in-tree `::default()` call sites to use a shared `test_diagram_config()` helper, deleted the now-unreachable `test_diagram_config_from_env` unit test (env-var tier coverage lives in `tests/cli_integration.rs::test_convert_with_env_var_diagram_paths`), and added a `test_converter_trait_object_invocation` test that dispatches through `&dyn Converter` so the trait impl on `MarkdownConverter` cannot be silently removed.

## Scope

Closed the IN-01 / WR-02 findings from the Phase 09 code review:

- After Phase 08 wired `DiagramConfig` into `Config::load`'s waterfall, the
  `from_env` constructor and both `Default` impls became dead public API with
  no production callers. They misleadingly suggested "use the `Default` to get
  env-var defaults" — which bypassed the CLI→env→`~/.claude`→default precedence
  that Phase 08 established.
- The `Converter` trait was exercised only indirectly (through `src/lib.rs::run`
  invoked by integration tests) and via `MockConverter` (which proves the
  trait is *implementable* but not that the production type dispatches through
  the trait vtable). A dedicated lock-in test was missing.

## Actual Changes Per File

| File | Diff | Notes |
|------|------|-------|
| src/config.rs | +0 / −22 | Removed `impl DiagramConfig { fn from_env() }` (lines 23-38) and `impl Default for DiagramConfig` (lines 40-44). `pub struct DiagramConfig { ... }` at lines 6-21 unchanged. `use std::path::Path` kept (used by `load_with_home`). |
| src/converter/mod.rs | +0 / −5 | Removed `impl Default for MarkdownConverter { fn default() -> Self { Self::new(DiagramConfig::default()) } }` (lines 47-51). `use crate::config::DiagramConfig` kept (still used by `MarkdownConverter::new` signature). |
| src/converter/tests.rs | +38 / −7 | Added 11-line `fn test_diagram_config()` helper at the top of the module. Rewrote 7 call sites (3 `MarkdownConverter::default()` → `MarkdownConverter::new(test_diagram_config())`; 4 `let config = crate::config::DiagramConfig::default()` → `let config = test_diagram_config()`). Appended 28-line `#[tokio::test] async fn test_converter_trait_object_invocation`. |
| src/converter/diagrams.rs | +0 / −46 | Removed the leading `// Note: DiagramConfig env tests...` comment pair (lines 179-180) plus the entire `#[test] fn test_diagram_config_from_env` body (lines 181-224, 44 lines). `fn config_with_defaults()` at lines 170-177 preserved; both `test_render_*_invalid_binary_returns_error` tests unchanged (they still use `..config_with_defaults()` struct-update syntax). |

Total commit: `4 files changed, 21 insertions(+), 83 deletions(-)`.

### 7 call-site rewrites (verbatim)

| # | File:Line | Before | After |
|---|-----------|--------|-------|
| 1 | src/converter/tests.rs:87 (was 87) | `let converter = MarkdownConverter::default();` | `let converter = MarkdownConverter::new(test_diagram_config());` |
| 2 | src/converter/tests.rs:99 (was 99) | `let converter = MarkdownConverter::default();` | `let converter = MarkdownConverter::new(test_diagram_config());` |
| 3 | src/converter/tests.rs:107 (was 107) | `let converter = MarkdownConverter::default();` | `let converter = MarkdownConverter::new(test_diagram_config());` |
| 4 | src/converter/tests.rs:124 (was 124) | `let config = crate::config::DiagramConfig::default();` | `let config = test_diagram_config();` |
| 5 | src/converter/tests.rs:142 (was 142) | `let config = crate::config::DiagramConfig::default();` | `let config = test_diagram_config();` |
| 6 | src/converter/tests.rs:165 (was 165) | `let config = crate::config::DiagramConfig::default();` | `let config = test_diagram_config();` |
| 7 | src/converter/tests.rs:198 (was 198) | `let config = crate::config::DiagramConfig::default();` | `let config = test_diagram_config();` |

Post-edit verification (file-level counts from grep):

```text
$ grep -c "MarkdownConverter::new(test_diagram_config())" src/converter/tests.rs
3
$ grep -c "let config = test_diagram_config();" src/converter/tests.rs
4
$ grep -n "fn test_diagram_config\b" src/converter/tests.rs
9:fn test_diagram_config() -> crate::config::DiagramConfig {
```

## Grep Evidence — Removed Symbols Absent

Ran after both tasks:

```text
$ grep -rn "DiagramConfig::from_env\|DiagramConfig::default\|MarkdownConverter::default\|impl Default for DiagramConfig\|impl Default for MarkdownConverter" src/ tests/
(no output — cleanup confirmed)
```

Also confirmed:

```text
$ grep -n "fn test_diagram_config_from_env" src/converter/diagrams.rs
(no output)
$ grep -n "fn config_with_defaults" src/converter/diagrams.rs
170:    fn config_with_defaults() -> DiagramConfig {
```

## Converter Trait Exercised — Audit

### 1. Grep evidence (literal shell output)

Production call sites on `src/lib.rs`:

```text
$ grep -n "MarkdownConverter::new\|\.convert(" src/lib.rs
103:            let converter = MarkdownConverter::new(config.diagram_config.clone());
104:            let convert_result = converter.convert(&markdown).await?;
171:            let converter = MarkdownConverter::new(config.diagram_config.clone());
172:            let convert_result = converter.convert(&markdown).await?;
226:            let converter = MarkdownConverter::new(diagram_config);
227:            let convert_result = converter.convert(&markdown).await?;
```

Three constructions × three `.convert(&markdown).await` calls — the Update
arm (L103-104), the Upload arm (L171-172), and the Convert arm (L226-227).
These match the line numbers the plan anticipated.

Dead-API removal confirmation (zero matches = Task 1 cleanup held):

```text
$ grep -rn "DiagramConfig::from_env\|DiagramConfig::default\|MarkdownConverter::default\|impl Default for DiagramConfig\|impl Default for MarkdownConverter" src/ tests/
(no output)
```

### 2. Call-chain table

| Test | Binary command | src/lib.rs arm | Invokes |
|------|---------------|-----------------|---------|
| `test_update_command_happy_path` (tests/cli_integration.rs:610) | `update` | Update arm (L103-104) | `<MarkdownConverter as Converter>::convert` |
| `test_upload_command_happy_path` (tests/cli_integration.rs:394) | `upload` | Upload arm (L171-172) | `<MarkdownConverter as Converter>::convert` |
| `test_convert_command` (tests/cli_integration.rs:89) | `convert` | Convert arm (L226-227) | `<MarkdownConverter as Converter>::convert` |
| `test_converter_trait_object_invocation` (src/converter/tests.rs) | — (unit test) | direct `&dyn Converter` | `<MarkdownConverter as Converter>::convert` (explicit trait object) |

### 3. Test run excerpt

```text
$ cargo test --test cli_integration 2>&1 | tail -15
running 11 tests
test test_convert_with_env_var_diagram_paths ... ok
test test_json_output_mode_error ... ok
test test_convert_with_diagram_path_flags ... ok
test test_update_command_missing_api_key ... ok
test test_convert_command_missing_file ... ok
test test_json_output_mode ... ok
test test_convert_command ... ok
test test_upload_command_rejects_http_url ... ok
test test_update_command_happy_path ... ok
test test_upload_command_happy_path ... ok
test test_upload_command_missing_credentials ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.10s
```

```text
$ cargo test --lib converter::tests::test_converter_trait_object_invocation -- --exact 2>&1 | tail -5
running 1 test
test converter::tests::test_converter_trait_object_invocation ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 116 filtered out; finished in 0.04s
```

### 4. Conclusion

The `Converter` trait is exercised by `MarkdownConverter` through three distinct
paths: (a) the three CLI command arms of `src/lib.rs::run` as invoked by the
integration tests (`test_update_command_happy_path`, `test_upload_command_happy_path`,
`test_convert_command`), (b) the `MockConverter` implementation exercised by
`test_mock_converter_compiles_and_works` (which proves implementability), and
(c) the new `test_converter_trait_object_invocation` that binds the concrete
type as `&dyn Converter` and dispatches through the trait vtable — so future
removal of `impl Converter for MarkdownConverter` would fail to compile this
test. Together these satisfy the ROADMAP success criterion that the `Converter`
trait is exercised in the integration test path.

## Verification

### Build

```text
$ cargo build 2>&1 | grep -E "^(warning|error)"
(no output)
```

### Lib tests

```text
$ cargo test --lib 2>&1 | grep "test result"
test result: ok. 117 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.40s
```

Note: lib test count went from 117 (pre-plan) → 116 (after Task 1 removal of
`test_diagram_config_from_env`) → 117 (after Task 2 added
`test_converter_trait_object_invocation`). The net count is identical.

### Full test sweep (default parallelism)

```text
$ cargo test 2>&1 | grep "test result"
test result: ok. 117 passed; 0 failed; 0 ignored                # lib
test result: ok.   0 passed; 0 failed; 0 ignored                # bin
test result: ok.  11 passed; 0 failed; 0 ignored                # cli_integration
test result: ok.  12 passed; 0 failed; 0 ignored                # llm_integration
test result: ok.   2 passed; 0 failed; 0 ignored                # output_format
test result: ok.   0 passed; 0 failed; 0 ignored                # doctests
```

Total: 142 tests passing, zero failures, zero ignored. No regression in the
10-01 happy-path tests (`test_update_command_happy_path`,
`test_upload_command_happy_path`).

## Deviations from Plan

### 1. [Rule 2 — Accuracy] Revised `test_diagram_config` doc-comment to drop the `::default` substring

**Found during:** Task 1 verification.

**Issue:** The plan's suggested doc-comment for the new helper read
`replaces the deleted DiagramConfig::default() and MarkdownConverter::default() impls`.
The plan's acceptance-criteria grep was
`grep -rn "DiagramConfig::default\|MarkdownConverter::default" src/ tests/` → `zero matches`.
Including the literal `::default()` substring inside the doc-comment text
would make that grep return 2 matches (both inside the `///` comment), which
technically violates the "zero matches" criterion even though the remaining
text is documentation, not code.

**Fix:** Rephrased the doc-comment to say
`replaces the deleted Default impls on DiagramConfig and MarkdownConverter`.
Same intent, no forbidden substring. The helper body is unchanged.

**Files modified:** src/converter/tests.rs (doc-comment text only).

**Commit:** e2c6a18 (incorporated into the refactor commit).

### 2. [Refactor TDD — no separate RED] Task 1 was a single atomic commit

**Rationale:** Task 1 is pure deletion + call-site rewrites. A partial edit
(e.g., delete the `Default` impls but not rewrite the call sites) leaves the
tree uncompilable — `cargo test --lib` fails to compile, not fails a test.
That would be a broken RED that doesn't prove anything about the desired end
state. The plan's own 10-01 summary noted the same pattern for Task 2 of
Plan 10-01: "production code was already in place … no separate RED commit."

The plan-level `tdd="true"` attribute is therefore satisfied by:
(a) the existing test suite acting as the behavioral spec (all tests green
before and after), (b) the grep acceptance criteria acting as the structural
RED/GREEN gate (before: 10+ matches; after: zero), and (c) the new
`test_converter_trait_object_invocation` added in Task 2 (authored after
the impl exists — the plan explicitly accepts this in its `<behavior>` block).

**No source-code impact** — this is a methodology note.

## TDD Gate Compliance

Plan-level `type: execute` (not `type: tdd`), but tasks are marked `tdd="true"`:

- **Task 1** — refactor-style TDD. RED is "grep shows 10+ matches + removed code still present"; GREEN is "grep shows zero matches + cargo test green". Single atomic commit (`e2c6a18`) transitions both states together because partial edits leave the tree uncompilable. No separate `test(...)` commit because the spec is a removal, not a new test — the existing test suite acts as the behavioral RED/GREEN gate.
- **Task 2** — lock-in TDD. The new `test_converter_trait_object_invocation` was authored after Task 1 (where the production `impl Converter for MarkdownConverter` already existed). It passes on first run by design — the test's value is future-facing: it will fail to compile if `impl Converter for MarkdownConverter` is ever removed. The plan's `<behavior>` block explicitly accepts this: "if `MarkdownConverter` ever stops implementing `Converter`, this test fails to compile". Committed separately as `test(10-02): add trait-object lock-in unit test` (see Task 2 commit hash in the commits list at the top of this SUMMARY). No separate RED commit — same pattern as Plan 10-01 Task 2 (integration tests authored after production code was already green).

## Known Stubs

None. The new `test_converter_trait_object_invocation` exercises real code
end-to-end through the `Converter` trait vtable; no placeholder data or
UI-facing stubs were introduced.

## Threat Flags

None. T-10-04 (regression on DiagramConfig callers) is mitigated by the
full test suite passing green (116 → 117 lib tests, 11/11 cli_integration
tests, 12/12 llm_integration tests, 142 total). T-10-05 (env-var coverage
regression) is a documented accept: `Config::load_with_home` retains
`DIAGRAM_TIMEOUT` / `MERMAID_PUPPETEER_CONFIG` reads, and clap-derive
retains `PLANTUML_PATH` / `MERMAID_PATH` via `#[arg(long, env = "...")]`.
No new trust boundaries introduced.

## Self-Check: PASSED

- FOUND: src/config.rs (modified, -22 lines)
- FOUND: src/converter/mod.rs (modified, -5 lines)
- FOUND: src/converter/tests.rs (modified, +38 / -7 lines; 7 rewrites + 1 helper + 1 trait-object test)
- FOUND: src/converter/diagrams.rs (modified, -46 lines)
- FOUND commit e2c6a18 (refactor + trait-object test combined)
- FOUND: grep -rn "DiagramConfig::from_env..." → zero matches
- FOUND: grep -c "MarkdownConverter::new(test_diagram_config())" → 3
- FOUND: grep -c "let config = test_diagram_config();" → 4
- FOUND: grep -n "fn test_diagram_config\b" → 1 match (helper)
- FOUND: grep -n "fn test_converter_trait_object_invocation" → 1 match
- FOUND: grep -n "let trait_obj: &dyn Converter" → 1 match (explicit trait-object binding)
- FOUND: cargo test --lib → 117 passed, 0 failed
- FOUND: cargo test --test cli_integration → 11 passed, 0 failed (10-01 happy-path preserved)
- FOUND: cargo test (full suite) → 142 passed, 0 failed, 0 ignored
- FOUND: cargo build → zero warnings / zero errors
