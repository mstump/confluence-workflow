use crate::cli::Cli;
use crate::error::ConfigError;
use std::path::Path;

/// Configuration for diagram rendering subprocesses.
#[derive(Debug, Clone)]
pub struct DiagramConfig {
    /// Path to PlantUML executable. Can be:
    /// - "plantuml" (Homebrew CLI wrapper, default)
    /// - "/path/to/plantuml.jar" (JAR mode -- will invoke as java -jar <path>)
    pub plantuml_path: String,

    /// Path to mermaid-cli executable (default: "mmdc")
    pub mermaid_path: String,

    /// Optional puppeteer config file for mermaid-cli
    pub mermaid_puppeteer_config: Option<String>,

    /// Timeout in seconds for each diagram render subprocess (default: 30)
    pub timeout_secs: u64,
}

/// Resolved, validated application configuration.
#[derive(Debug)]
pub struct Config {
    pub confluence_url: String,
    pub confluence_username: String,
    pub confluence_api_token: String,
    pub anthropic_api_key: Option<String>,
    pub anthropic_model: String,
    pub anthropic_concurrency: usize,
    pub diagram_config: DiagramConfig,
}

impl Config {
    /// Load configuration from already-parsed CLI values.
    ///
    /// Clap-derive has already resolved CLI flag to env var for every `Option<String>`
    /// field on `&Cli` (via `#[arg(long, env = "...")]`). This function fills in the
    /// `~/.claude/settings.json` credential fallback tier and applies defaults for
    /// non-credential fields.
    ///
    /// Note: `dotenvy::dotenv().ok()` is called in `src/main.rs` before `Cli::parse()`,
    /// NOT here, so clap's env-var resolution sees `.env`-sourced values.
    pub fn load(cli: &Cli) -> Result<Self, ConfigError> {
        Self::load_with_home(cli, dirs::home_dir().as_deref())
    }

    /// Load with an explicit home directory — used in tests to avoid reading real credentials.
    pub(crate) fn load_with_home(
        cli: &Cli,
        home: Option<&Path>,
    ) -> Result<Self, ConfigError> {

        let confluence_url = Self::resolve_required(
            cli.confluence_url.as_deref(),
            "CONFLUENCE_URL",
            home,
        )?;
        // Normalize: strip trailing slash to prevent double-slash in API paths.
        let confluence_url = confluence_url.trim_end_matches('/').trim().to_string();

        // Threat model T-01-04: validate scheme to prevent accidental HTTP use.
        // Use to_ascii_lowercase() so mixed-case inputs like "HTTPS://" are accepted.
        // D-01: narrowly exempt http://localhost and http://127.0.0.1 for integration
        // testing (wiremock binds to loopback); external hosts remain rejected.
        let url_lower = confluence_url.to_ascii_lowercase();
        if !url_lower.starts_with("https://")
            && !url_lower.starts_with("http://localhost")
            && !url_lower.starts_with("http://127.0.0.1")
        {
            return Err(ConfigError::Invalid {
                name: "CONFLUENCE_URL",
                reason: "must start with https:// (or http://localhost for testing)",
            });
        }

        let confluence_username = Self::resolve_required(
            cli.confluence_username.as_deref(),
            "CONFLUENCE_USERNAME",
            home,
        )?;
        let confluence_username = confluence_username.trim().to_string();

        // NOTE: Cli field is `confluence_token` (not `confluence_api_token`);
        // env_key stays `"CONFLUENCE_API_TOKEN"` so the ~/.claude/ lookup uses
        // the external key name.
        let confluence_api_token = Self::resolve_required(
            cli.confluence_token.as_deref(),
            "CONFLUENCE_API_TOKEN",
            home,
        )?;
        let confluence_api_token = confluence_api_token.trim().to_string();

        let anthropic_api_key = Self::resolve_optional(
            cli.anthropic_api_key.as_deref(),
            "ANTHROPIC_API_KEY",
            home,
        );

        let anthropic_model = std::env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-haiku-4-5-20251001".to_string());

        let anthropic_concurrency = std::env::var("ANTHROPIC_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(5)
            .max(1)   // prevent zero-permit deadlock
            .min(50); // prevent runaway concurrency

        // Diagram paths: Cli tier (clap-resolved from CLI flag OR env var) to default.
        // No ~/.claude/ tier per D-03/D-05 — credentials only get that tier.
        let plantuml_path = cli.plantuml_path.clone()
            .unwrap_or_else(|| "plantuml".to_string());
        let mermaid_path = cli.mermaid_path.clone()
            .unwrap_or_else(|| "mmdc".to_string());

        let mermaid_puppeteer_config = std::env::var("MERMAID_PUPPETEER_CONFIG").ok();
        let timeout_secs = std::env::var("DIAGRAM_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        let diagram_config = DiagramConfig {
            plantuml_path,
            mermaid_path,
            mermaid_puppeteer_config,
            timeout_secs,
        };

        Ok(Config {
            confluence_url,
            confluence_username,
            confluence_api_token,
            anthropic_api_key,
            anthropic_model,
            anthropic_concurrency,
            diagram_config,
        })
    }

    /// Resolve a required field. Returns `ConfigError::Missing` if no source
    /// provides a non-empty value.
    ///
    /// Tier 1 (Cli) already includes the env-var value because clap-derive's
    /// `#[arg(long, env = "...")]` attribute resolves it at `Cli::parse()` time.
    /// Tier 2 is the `~/.claude/settings.json` credential fallback.
    fn resolve_required(
        cli_value: Option<&str>,
        env_key: &'static str,
        home: Option<&Path>,
    ) -> Result<String, ConfigError> {
        // 1. Cli tier (already includes env var via clap-derive)
        if let Some(val) = cli_value {
            if !val.is_empty() {
                return Ok(val.to_string());
            }
        }

        // 2. ~/.claude/ fallback (best-effort)
        if let Some(val) = load_from_claude_config(env_key, home) {
            if !val.is_empty() {
                return Ok(val);
            }
        }

        Err(ConfigError::Missing { name: env_key })
    }

    /// Resolve an optional field. Returns `None` if no source provides a value
    /// — this is not an error.
    ///
    /// Tier 1 (Cli) already includes the env-var value because clap-derive's
    /// `#[arg(long, env = "...")]` attribute resolves it at `Cli::parse()` time.
    /// Tier 2 is the `~/.claude/settings.json` credential fallback.
    fn resolve_optional(
        cli_value: Option<&str>,
        env_key: &str,
        home: Option<&Path>,
    ) -> Option<String> {
        // 1. Cli tier (already includes env var via clap-derive)
        if let Some(val) = cli_value {
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }

        // 2. ~/.claude/ fallback (best-effort)
        load_from_claude_config(env_key, home)
    }
}

/// Best-effort attempt to load a key from `{home}/.claude/settings.json`.
///
/// Looks for the key at the top level of the JSON object.  Returns `None` if
/// the home directory cannot be determined, the file is absent, or the key is
/// not present — never returns an error.
fn load_from_claude_config(key: &str, home: Option<&Path>) -> Option<String> {
    let home = home?;
    let settings_path = home.join(".claude").join("settings.json");

    let content = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => {
            tracing::debug!(
                "~/.claude/settings.json not found or unreadable, skipping"
            );
            return None;
        }
    };

    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => {
            tracing::debug!("~/.claude/settings.json is not valid JSON, skipping");
            return None;
        }
    };

    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands, OutputFormat};
    use serial_test::serial;
    use std::path::PathBuf;

    /// A non-existent home path used in tests to prevent reading real ~/.claude/ credentials.
    fn no_home() -> PathBuf {
        PathBuf::from("/nonexistent-test-home-dir-that-cannot-exist")
    }

    /// Build a Cli struct populated with None/default values. Tests override
    /// individual fields with struct-update syntax:
    /// `Cli { confluence_url: Some(...), ..cli_blank() }`.
    fn cli_blank() -> Cli {
        Cli {
            confluence_url: None,
            confluence_username: None,
            confluence_token: None,
            anthropic_api_key: None,
            plantuml_path: None,
            mermaid_path: None,
            verbose: false,
            output: OutputFormat::Human,
            command: Commands::Convert {
                markdown_path: PathBuf::new(),
                output_dir: PathBuf::new(),
            },
        }
    }

    // Test 1: Config loads from CLI overrides when all three Confluence fields are provided.
    #[test]
    fn test_load_from_cli_overrides() {
        let cli = Cli {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_token: Some("token123".to_string()),
            anthropic_api_key: Some("ant-key".to_string()),
            ..cli_blank()
        };

        // CLI values override everything; use no_home() so ~/.claude/ is never checked.
        let config = Config::load_with_home(&cli, Some(&no_home()))
            .expect("should load from CLI overrides");

        assert_eq!(config.confluence_url, "https://example.atlassian.net");
        assert_eq!(config.confluence_username, "user@example.com");
        assert_eq!(config.confluence_api_token, "token123");
        assert_eq!(config.anthropic_api_key.as_deref(), Some("ant-key"));
    }

    // test_fallthrough_to_env_vars and test_env_vars_used_when_cli_absent were removed
    // in Phase 09 — they tested the Config::resolve_required env-var tier that now
    // lives exclusively in clap-derive's #[arg(env = "...")] attribute. Env-var tier
    // coverage moved to tests/cli_integration.rs::test_convert_with_env_var_diagram_paths.

    // Test 4: Missing confluence_url produces ConfigError::Missing with name "CONFLUENCE_URL".
    #[test]
    #[serial]
    fn test_missing_confluence_url_error() {
        // Provide username + token via CLI; leave URL absent from all sources.
        // Use no_home() so ~/.claude/ fallback returns None.
        let cli = Cli {
            confluence_username: Some("user@example.com".to_string()),
            confluence_token: Some("token".to_string()),
            ..cli_blank()
        };
        // Ensure env var not set for this test (remove it, then restore later).
        let saved = std::env::var("CONFLUENCE_URL").ok();
        std::env::remove_var("CONFLUENCE_URL");

        let err = Config::load_with_home(&cli, Some(&no_home()))
            .expect_err("should fail when CONFLUENCE_URL missing");

        if let Some(v) = saved {
            std::env::set_var("CONFLUENCE_URL", v);
        }

        match err {
            ConfigError::Missing { name } => {
                assert!(
                    name.contains("CONFLUENCE_URL"),
                    "error name should contain CONFLUENCE_URL, got: {name}"
                );
            }
            other => panic!("expected ConfigError::Missing, got: {other:?}"),
        }
    }

    // Test 5: Missing confluence_username produces ConfigError::Missing with name "CONFLUENCE_USERNAME".
    #[test]
    #[serial]
    fn test_missing_confluence_username_error() {
        let cli = Cli {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_token: Some("token".to_string()),
            ..cli_blank()
        };
        let saved = std::env::var("CONFLUENCE_USERNAME").ok();
        std::env::remove_var("CONFLUENCE_USERNAME");

        let err = Config::load_with_home(&cli, Some(&no_home()))
            .expect_err("should fail when CONFLUENCE_USERNAME missing");

        if let Some(v) = saved {
            std::env::set_var("CONFLUENCE_USERNAME", v);
        }

        match err {
            ConfigError::Missing { name } => {
                assert_eq!(name, "CONFLUENCE_USERNAME");
            }
            other => panic!("expected ConfigError::Missing, got: {other:?}"),
        }
    }

    // Test 6: Missing confluence_api_token produces ConfigError::Missing with name "CONFLUENCE_API_TOKEN".
    #[test]
    #[serial]
    fn test_missing_confluence_api_token_error() {
        let cli = Cli {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            ..cli_blank()
        };
        let saved = std::env::var("CONFLUENCE_API_TOKEN").ok();
        std::env::remove_var("CONFLUENCE_API_TOKEN");

        let err = Config::load_with_home(&cli, Some(&no_home()))
            .expect_err("should fail when CONFLUENCE_API_TOKEN missing");

        if let Some(v) = saved {
            std::env::set_var("CONFLUENCE_API_TOKEN", v);
        }

        match err {
            ConfigError::Missing { name } => {
                assert_eq!(name, "CONFLUENCE_API_TOKEN");
            }
            other => panic!("expected ConfigError::Missing, got: {other:?}"),
        }
    }

    // Test 7: anthropic_api_key is Optional and does not cause an error when absent.
    #[test]
    #[serial]
    fn test_anthropic_api_key_optional() {
        let cli = Cli {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_token: Some("token".to_string()),
            anthropic_api_key: None,
            ..cli_blank()
        };

        // Remove ANTHROPIC_API_KEY from env so only CLI (which is None) is checked.
        let saved = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");

        let result = Config::load_with_home(&cli, Some(&no_home()));

        if let Some(v) = saved {
            std::env::set_var("ANTHROPIC_API_KEY", v);
        }

        let config = result.expect("should succeed without anthropic_api_key");
        assert!(
            config.anthropic_api_key.is_none(),
            "anthropic_api_key should be None when not provided"
        );
    }

    // Test 8: ~/.claude/ fallback stub — when no other source provides a value, the code
    // attempts to read ~/.claude/ before erroring.  Verified by:
    // (a) using a non-existent home dir path so the read returns None, and
    // (b) confirming the resulting error is ConfigError::Missing (not a FileRead error).
    #[test]
    #[serial]
    fn test_claude_config_fallback_stub_then_error() {
        // Provide only username + token; omit URL so we reach the ~/.claude/ attempt.
        let cli = Cli {
            confluence_username: Some("user@example.com".to_string()),
            confluence_token: Some("token".to_string()),
            ..cli_blank()
        };
        let saved = std::env::var("CONFLUENCE_URL").ok();
        std::env::remove_var("CONFLUENCE_URL");

        // Use no_home() — the ~/.claude/ path won't exist, so fallback returns None → Missing.
        let err = Config::load_with_home(&cli, Some(&no_home()))
            .expect_err("should fail after ~/.claude/ stub");

        if let Some(v) = saved {
            std::env::set_var("CONFLUENCE_URL", v);
        }

        match err {
            ConfigError::Missing { name } => {
                assert!(
                    name.contains("CONFLUENCE_URL"),
                    "should report missing CONFLUENCE_URL, got: {name}"
                );
            }
            other => panic!("expected ConfigError::Missing after ~/.claude/ attempt, got: {other:?}"),
        }
    }

    // Test 9: confluence_url trailing slash is stripped.
    #[test]
    fn test_confluence_url_trailing_slash_stripped() {
        let cli = Cli {
            confluence_url: Some("https://example.atlassian.net/".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_token: Some("token".to_string()),
            ..cli_blank()
        };

        let config = Config::load_with_home(&cli, Some(&no_home()))
            .expect("should load with trailing slash URL");
        assert_eq!(
            config.confluence_url,
            "https://example.atlassian.net",
            "trailing slash should be stripped"
        );
    }

    // Test 10: non-localhost http:// URLs are rejected (T-01-04 threat mitigation;
    // D-01 exempts localhost — see companion test below).
    #[test]
    fn test_confluence_url_must_be_https() {
        let cli = Cli {
            confluence_url: Some("http://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_token: Some("token".to_string()),
            ..cli_blank()
        };

        let err = Config::load_with_home(&cli, Some(&no_home()))
            .expect_err("should reject http:// URL");
        match err {
            ConfigError::Invalid { name, reason } => {
                assert_eq!(name, "CONFLUENCE_URL", "error should reference CONFLUENCE_URL");
                assert!(
                    reason.contains("https://"),
                    "error reason should mention https://, got: {reason}"
                );
            }
            other => panic!("expected ConfigError::Invalid for http URL, got: {other:?}"),
        }
    }

    // Test: http://localhost and http://127.0.0.1 are accepted for integration testing (D-01).
    #[test]
    fn test_confluence_url_localhost_exemption() {
        for url in ["http://localhost:8080", "http://127.0.0.1:9999", "HTTP://LOCALHOST:1234"] {
            let cli = Cli {
                confluence_url: Some(url.to_string()),
                confluence_username: Some("user@example.com".to_string()),
                confluence_token: Some("token".to_string()),
                ..cli_blank()
            };
            let cfg = Config::load_with_home(&cli, Some(&no_home()))
                .unwrap_or_else(|e| panic!("{url} should be allowed, got: {e:?}"));
            assert_eq!(
                cfg.confluence_url.to_ascii_lowercase(),
                url.to_ascii_lowercase(),
                "stored URL should match the input (case-insensitive compare)"
            );
        }
    }

    // Test 11: --plantuml-path CLI override propagates through waterfall into diagram_config.
    #[test]
    #[serial]
    fn test_plantuml_path_cli_override() {
        let cli = Cli {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_token: Some("token".to_string()),
            plantuml_path: Some("/custom/plantuml".to_string()),
            ..cli_blank()
        };
        let saved = std::env::var("PLANTUML_PATH").ok();
        std::env::remove_var("PLANTUML_PATH");

        let config = Config::load_with_home(&cli, Some(&no_home()))
            .expect("should load with plantuml_path override");

        match saved {
            Some(v) => std::env::set_var("PLANTUML_PATH", v),
            None => std::env::remove_var("PLANTUML_PATH"),
        }

        assert_eq!(config.diagram_config.plantuml_path, "/custom/plantuml");
    }

    // Test 12: --mermaid-path CLI override propagates through waterfall into diagram_config.
    #[test]
    #[serial]
    fn test_mermaid_path_cli_override() {
        let cli = Cli {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_token: Some("token".to_string()),
            mermaid_path: Some("/custom/mmdc".to_string()),
            ..cli_blank()
        };
        let saved = std::env::var("MERMAID_PATH").ok();
        std::env::remove_var("MERMAID_PATH");

        let config = Config::load_with_home(&cli, Some(&no_home()))
            .expect("should load with mermaid_path override");

        match saved {
            Some(v) => std::env::set_var("MERMAID_PATH", v),
            None => std::env::remove_var("MERMAID_PATH"),
        }

        assert_eq!(config.diagram_config.mermaid_path, "/custom/mmdc");
    }

    // Test 13: When no CLI override or env var is set, diagram paths default to "plantuml"/"mmdc".
    #[test]
    #[serial]
    fn test_diagram_config_defaults_when_no_override() {
        let cli = Cli {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_token: Some("token".to_string()),
            ..cli_blank()
        };
        let saved_p = std::env::var("PLANTUML_PATH").ok();
        let saved_m = std::env::var("MERMAID_PATH").ok();
        std::env::remove_var("PLANTUML_PATH");
        std::env::remove_var("MERMAID_PATH");

        let config = Config::load_with_home(&cli, Some(&no_home()))
            .expect("should load with default diagram paths");

        match saved_p {
            Some(v) => std::env::set_var("PLANTUML_PATH", v),
            None => std::env::remove_var("PLANTUML_PATH"),
        }
        match saved_m {
            Some(v) => std::env::set_var("MERMAID_PATH", v),
            None => std::env::remove_var("MERMAID_PATH"),
        }

        assert_eq!(config.diagram_config.plantuml_path, "plantuml");
        assert_eq!(config.diagram_config.mermaid_path, "mmdc");
    }
}
