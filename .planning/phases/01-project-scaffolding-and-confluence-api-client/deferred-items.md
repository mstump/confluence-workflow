# Deferred Items — Phase 01

## Pre-existing Issues (Out of Scope for Plan 03)

### Config test env var pollution under parallel test execution

**Discovered during:** Plan 03 final verification (`cargo test`)
**Affects:** `config::tests::test_env_vars_used_when_cli_absent`, `config::tests::test_fallthrough_to_env_vars`
**Root cause:** Config tests use `std::env::set_var` / `remove_var` which are not thread-safe when tests run in parallel. The wiremock tests added in Plan 03 may share the test thread pool and cause env var state to bleed between tests.
**Workaround:** `cargo test --lib -- --test-threads=1` passes all 25 tests.
**Fix required:** Plan 02 config tests should use `serial_test` crate or restructure to avoid global env mutation. This is a pre-existing issue introduced in Plan 02.
**Priority:** Low — does not affect production code; only test runner parallelism.
