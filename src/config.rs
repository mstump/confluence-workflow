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

impl DiagramConfig {
    /// Load from environment variables with sensible defaults.
    pub fn from_env() -> Self {
        Self {
            plantuml_path: std::env::var("PLANTUML_PATH")
                .unwrap_or_else(|_| "plantuml".to_string()),
            mermaid_path: std::env::var("MERMAID_PATH")
                .unwrap_or_else(|_| "mmdc".to_string()),
            mermaid_puppeteer_config: std::env::var("MERMAID_PUPPETEER_CONFIG").ok(),
            timeout_secs: std::env::var("DIAGRAM_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        }
    }
}

impl Default for DiagramConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

/// CLI override values — mirrors the optional CLI flags from `Cli`.
#[derive(Debug, Default)]
pub struct CliOverrides {
    pub confluence_url: Option<String>,
    pub confluence_username: Option<String>,
    pub confluence_api_token: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub plantuml_path: Option<String>,
    pub mermaid_path: Option<String>,
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
    /// Load configuration using the waterfall strategy:
    /// CLI override → environment variable → `.env` file → `~/.claude/` fallback.
    ///
    /// Calls `dotenvy::dotenv().ok()` first to load `.env` into the environment,
    /// then resolves each field in priority order.
    pub fn load(overrides: &CliOverrides) -> Result<Self, ConfigError> {
        // Load .env into env vars (non-fatal if file is absent).
        dotenvy::dotenv().ok();
        Self::load_with_home(overrides, dirs::home_dir().as_deref())
    }

    /// Load with an explicit home directory — used in tests to avoid reading real credentials.
    /// Does NOT call dotenvy::dotenv() so tests have full control over the environment.
    pub(crate) fn load_with_home(
        overrides: &CliOverrides,
        home: Option<&Path>,
    ) -> Result<Self, ConfigError> {

        let confluence_url = Self::resolve_required(
            overrides.confluence_url.as_deref(),
            "CONFLUENCE_URL",
            home,
        )?;
        // Normalize: strip trailing slash to prevent double-slash in API paths.
        let confluence_url = confluence_url.trim_end_matches('/').trim().to_string();

        // Threat model T-01-04: validate scheme to prevent accidental HTTP use.
        // Use to_ascii_lowercase() so mixed-case inputs like "HTTPS://" are accepted.
        if !confluence_url.to_ascii_lowercase().starts_with("https://") {
            return Err(ConfigError::Invalid {
                name: "CONFLUENCE_URL",
                reason: "must start with https://",
            });
        }

        let confluence_username = Self::resolve_required(
            overrides.confluence_username.as_deref(),
            "CONFLUENCE_USERNAME",
            home,
        )?;
        let confluence_username = confluence_username.trim().to_string();

        let confluence_api_token = Self::resolve_required(
            overrides.confluence_api_token.as_deref(),
            "CONFLUENCE_API_TOKEN",
            home,
        )?;
        let confluence_api_token = confluence_api_token.trim().to_string();

        let anthropic_api_key = Self::resolve_optional(
            overrides.anthropic_api_key.as_deref(),
            "ANTHROPIC_API_KEY",
            home,
        );

        let anthropic_model = Self::resolve_optional(
            None,
            "ANTHROPIC_MODEL",
            home,
        )
        .unwrap_or_else(|| "claude-haiku-4-5-20251001".to_string());

        let anthropic_concurrency = std::env::var("ANTHROPIC_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(5)
            .min(50); // prevent runaway concurrency

        let plantuml_path = Self::resolve_optional(
            overrides.plantuml_path.as_deref(),
            "PLANTUML_PATH",
            home,
        ).unwrap_or_else(|| "plantuml".to_string());

        let mermaid_path = Self::resolve_optional(
            overrides.mermaid_path.as_deref(),
            "MERMAID_PATH",
            home,
        ).unwrap_or_else(|| "mmdc".to_string());

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

    /// Resolve a required field from the waterfall. Returns `ConfigError::Missing` if
    /// no source provides a non-empty value.
    fn resolve_required(
        cli_override: Option<&str>,
        env_key: &'static str,
        home: Option<&Path>,
    ) -> Result<String, ConfigError> {
        // 1. CLI override
        if let Some(val) = cli_override {
            if !val.is_empty() {
                return Ok(val.to_string());
            }
        }

        // 2. Environment variable (already includes .env via dotenvy)
        if let Ok(val) = std::env::var(env_key) {
            if !val.is_empty() {
                return Ok(val);
            }
        }

        // 3. ~/.claude/ fallback (best-effort stub)
        if let Some(val) = load_from_claude_config(env_key, home) {
            if !val.is_empty() {
                return Ok(val);
            }
        }

        Err(ConfigError::Missing { name: env_key })
    }

    /// Resolve an optional field from the waterfall. Returns `None` if no source provides
    /// a value — this is not an error.
    fn resolve_optional(
        cli_override: Option<&str>,
        env_key: &str,
        home: Option<&Path>,
    ) -> Option<String> {
        // 1. CLI override
        if let Some(val) = cli_override {
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }

        // 2. Environment variable
        if let Ok(val) = std::env::var(env_key) {
            if !val.is_empty() {
                return Some(val);
            }
        }

        // 3. ~/.claude/ fallback (best-effort)
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
    use serial_test::serial;
    use std::path::PathBuf;

    /// A non-existent home path used in tests to prevent reading real ~/.claude/ credentials.
    fn no_home() -> PathBuf {
        PathBuf::from("/nonexistent-test-home-dir-that-cannot-exist")
    }

    // Test 1: Config loads from CLI overrides when all three Confluence fields are provided.
    #[test]
    fn test_load_from_cli_overrides() {
        let overrides = CliOverrides {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_api_token: Some("token123".to_string()),
            anthropic_api_key: Some("ant-key".to_string()),
            ..Default::default()
        };

        // CLI values override everything; use no_home() so ~/.claude/ is never checked.
        let config = Config::load_with_home(&overrides, Some(&no_home()))
            .expect("should load from CLI overrides");

        assert_eq!(config.confluence_url, "https://example.atlassian.net");
        assert_eq!(config.confluence_username, "user@example.com");
        assert_eq!(config.confluence_api_token, "token123");
        assert_eq!(config.anthropic_api_key.as_deref(), Some("ant-key"));
    }

    // Test 2: Config falls through to env vars when CLI overrides are None.
    #[test]
    #[serial]
    fn test_fallthrough_to_env_vars() {
        // Use unique env var names scoped to this test to avoid races.
        // We temporarily set then clear them; run tests sequentially if flaky.
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
        assert_eq!(config.confluence_username, "env-user@example.com");
        assert_eq!(config.confluence_api_token, "env-token");
    }

    // Test 3: Config loads .env file when env vars are not set.
    // dotenvy loads the .env into the process environment before resolution, so the
    // resolution path is identical to env vars.  We verify by setting vars directly
    // (simulating what dotenvy does) and confirming they are picked up.
    #[test]
    #[serial]
    fn test_env_vars_used_when_cli_absent() {
        std::env::set_var("CONFLUENCE_URL", "https://dotenv.atlassian.net");
        std::env::set_var("CONFLUENCE_USERNAME", "dotenv-user");
        std::env::set_var("CONFLUENCE_API_TOKEN", "dotenv-token");

        let overrides = CliOverrides::default();
        let result = Config::load_with_home(&overrides, Some(&no_home()));

        std::env::remove_var("CONFLUENCE_URL");
        std::env::remove_var("CONFLUENCE_USERNAME");
        std::env::remove_var("CONFLUENCE_API_TOKEN");

        let config = result.expect("should load when env vars present");
        assert_eq!(config.confluence_url, "https://dotenv.atlassian.net");
        assert_eq!(config.confluence_username, "dotenv-user");
        assert_eq!(config.confluence_api_token, "dotenv-token");
    }

    // Test 4: Missing confluence_url produces ConfigError::Missing with name "CONFLUENCE_URL".
    #[test]
    #[serial]
    fn test_missing_confluence_url_error() {
        // Provide username + token via CLI; leave URL absent from all sources.
        // Use no_home() so ~/.claude/ fallback returns None.
        let overrides = CliOverrides {
            confluence_username: Some("user@example.com".to_string()),
            confluence_api_token: Some("token".to_string()),
            ..Default::default()
        };
        // Ensure env var not set for this test (remove it, then restore later).
        let saved = std::env::var("CONFLUENCE_URL").ok();
        std::env::remove_var("CONFLUENCE_URL");

        let err = Config::load_with_home(&overrides, Some(&no_home()))
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
        let overrides = CliOverrides {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_api_token: Some("token".to_string()),
            ..Default::default()
        };
        let saved = std::env::var("CONFLUENCE_USERNAME").ok();
        std::env::remove_var("CONFLUENCE_USERNAME");

        let err = Config::load_with_home(&overrides, Some(&no_home()))
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
        let overrides = CliOverrides {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            ..Default::default()
        };
        let saved = std::env::var("CONFLUENCE_API_TOKEN").ok();
        std::env::remove_var("CONFLUENCE_API_TOKEN");

        let err = Config::load_with_home(&overrides, Some(&no_home()))
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
        let overrides = CliOverrides {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_api_token: Some("token".to_string()),
            anthropic_api_key: None,
            ..Default::default()
        };

        // Remove ANTHROPIC_API_KEY from env so only CLI (which is None) is checked.
        let saved = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");

        let result = Config::load_with_home(&overrides, Some(&no_home()));

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
        let overrides = CliOverrides {
            confluence_username: Some("user@example.com".to_string()),
            confluence_api_token: Some("token".to_string()),
            ..Default::default()
        };
        let saved = std::env::var("CONFLUENCE_URL").ok();
        std::env::remove_var("CONFLUENCE_URL");

        // Use no_home() — the ~/.claude/ path won't exist, so fallback returns None → Missing.
        let err = Config::load_with_home(&overrides, Some(&no_home()))
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
        let overrides = CliOverrides {
            confluence_url: Some("https://example.atlassian.net/".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_api_token: Some("token".to_string()),
            ..Default::default()
        };

        let config = Config::load_with_home(&overrides, Some(&no_home()))
            .expect("should load with trailing slash URL");
        assert_eq!(
            config.confluence_url,
            "https://example.atlassian.net",
            "trailing slash should be stripped"
        );
    }

    // Test 10: confluence_url must start with https:// (T-01-04 threat mitigation).
    #[test]
    fn test_confluence_url_must_be_https() {
        let overrides = CliOverrides {
            confluence_url: Some("http://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_api_token: Some("token".to_string()),
            ..Default::default()
        };

        let err = Config::load_with_home(&overrides, Some(&no_home()))
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

    // Test 11: --plantuml-path CLI override propagates through waterfall into diagram_config.
    #[test]
    #[serial]
    fn test_plantuml_path_cli_override() {
        let overrides = CliOverrides {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_api_token: Some("token".to_string()),
            plantuml_path: Some("/custom/plantuml".to_string()),
            ..Default::default()
        };
        let saved = std::env::var("PLANTUML_PATH").ok();
        std::env::remove_var("PLANTUML_PATH");

        let config = Config::load_with_home(&overrides, Some(&no_home()))
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
        let overrides = CliOverrides {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_api_token: Some("token".to_string()),
            mermaid_path: Some("/custom/mmdc".to_string()),
            ..Default::default()
        };
        let saved = std::env::var("MERMAID_PATH").ok();
        std::env::remove_var("MERMAID_PATH");

        let config = Config::load_with_home(&overrides, Some(&no_home()))
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
        let overrides = CliOverrides {
            confluence_url: Some("https://example.atlassian.net".to_string()),
            confluence_username: Some("user@example.com".to_string()),
            confluence_api_token: Some("token".to_string()),
            ..Default::default()
        };
        let saved_p = std::env::var("PLANTUML_PATH").ok();
        let saved_m = std::env::var("MERMAID_PATH").ok();
        std::env::remove_var("PLANTUML_PATH");
        std::env::remove_var("MERMAID_PATH");

        let config = Config::load_with_home(&overrides, Some(&no_home()))
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
