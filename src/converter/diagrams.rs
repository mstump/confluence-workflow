use crate::config::DiagramConfig;
use crate::error::ConversionError;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Render PlantUML content to SVG bytes.
///
/// Supports two modes based on plantuml_path:
/// - If path ends with ".jar": invokes `java -jar <path> -tsvg -pipe`
/// - Otherwise: invokes `<path> -tsvg -pipe` (CLI wrapper mode)
pub async fn render_plantuml(
    content: &str,
    config: &DiagramConfig,
) -> Result<Vec<u8>, ConversionError> {
    let mut cmd = if config.plantuml_path.ends_with(".jar") {
        let mut c = Command::new("java");
        c.args(["-jar", &config.plantuml_path, "-tsvg", "-pipe"]);
        c
    } else {
        let mut c = Command::new(&config.plantuml_path);
        c.args(["-tsvg", "-pipe"]);
        c
    };

    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| ConversionError::DiagramError {
        diagram_type: "plantuml".to_string(),
        message: format!("Failed to spawn PlantUML process: {e}"),
    })?;

    // Write content to stdin and close the pipe before waiting for output.
    // PlantUML reads stdin in -pipe mode and produces output only after EOF,
    // so writing inline before wait_with_output is safe and correct.
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(content.as_bytes())
            .await
            .map_err(|e| ConversionError::DiagramError {
                diagram_type: "plantuml".to_string(),
                message: format!("Failed to write to PlantUML stdin: {e}"),
            })?;
        // Dropping stdin closes the pipe and signals EOF to the process.
    }

    // Wait with timeout
    let output = tokio::time::timeout(
        Duration::from_secs(config.timeout_secs),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| ConversionError::DiagramTimeout {
        diagram_type: "plantuml".to_string(),
        timeout_secs: config.timeout_secs,
    })?
    .map_err(|e| ConversionError::DiagramError {
        diagram_type: "plantuml".to_string(),
        message: format!("PlantUML process failed: {e}"),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ConversionError::DiagramError {
            diagram_type: "plantuml".to_string(),
            message: format!("PlantUML exited with {}: {stderr}", output.status),
        });
    }

    if output.stdout.is_empty() {
        return Err(ConversionError::DiagramError {
            diagram_type: "plantuml".to_string(),
            message: "PlantUML produced empty output".to_string(),
        });
    }

    Ok(output.stdout)
}

/// Render Mermaid content to SVG bytes via mermaid-cli (mmdc).
///
/// Uses tempfile for input/output because mmdc requires file paths.
pub async fn render_mermaid(
    content: &str,
    config: &DiagramConfig,
) -> Result<Vec<u8>, ConversionError> {
    let input_file = tempfile::Builder::new()
        .suffix(".mmd")
        .tempfile()
        .map_err(|e| ConversionError::DiagramError {
            diagram_type: "mermaid".to_string(),
            message: format!("Failed to create temp file: {e}"),
        })?;

    std::fs::write(input_file.path(), content).map_err(|e| ConversionError::DiagramError {
        diagram_type: "mermaid".to_string(),
        message: format!("Failed to write temp file: {e}"),
    })?;

    let output_path = input_file.path().with_extension("svg");

    let mut cmd = Command::new(&config.mermaid_path);
    cmd.args([
        "-i",
        &input_file.path().to_string_lossy(),
        "-o",
        &output_path.to_string_lossy(),
        "-e",
        "svg",
    ]);

    if let Some(ref puppeteer_config) = config.mermaid_puppeteer_config {
        cmd.args(["--puppeteerConfigFile", puppeteer_config]);
    }

    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let timeout_result = tokio::time::timeout(
        Duration::from_secs(config.timeout_secs),
        cmd.output(),
    )
    .await;

    let output = match timeout_result {
        Err(_) => {
            // Timeout — attempt best-effort cleanup of any partial output file
            let _ = std::fs::remove_file(&output_path);
            return Err(ConversionError::DiagramTimeout {
                diagram_type: "mermaid".to_string(),
                timeout_secs: config.timeout_secs,
            });
        }
        Ok(res) => res.map_err(|e| ConversionError::DiagramError {
            diagram_type: "mermaid".to_string(),
            message: format!("Mermaid process failed: {e}"),
        })?,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ConversionError::DiagramError {
            diagram_type: "mermaid".to_string(),
            message: format!("mmdc exited with {}: {stderr}", output.status),
        });
    }

    let svg_bytes = std::fs::read(&output_path).map_err(|e| {
        // Clean up before returning so the file is not leaked on this error path.
        let _ = std::fs::remove_file(&output_path);
        ConversionError::DiagramError {
            diagram_type: "mermaid".to_string(),
            message: format!("Failed to read SVG output: {e}"),
        }
    })?;

    // Clean up output file (input auto-cleaned by tempfile)
    let _ = std::fs::remove_file(&output_path);

    if svg_bytes.is_empty() {
        return Err(ConversionError::DiagramError {
            diagram_type: "mermaid".to_string(),
            message: "mmdc produced empty SVG output".to_string(),
        });
    }

    Ok(svg_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DiagramConfig;

    fn config_with_defaults() -> DiagramConfig {
        DiagramConfig {
            plantuml_path: "plantuml".to_string(),
            mermaid_path: "mmdc".to_string(),
            mermaid_puppeteer_config: None,
            timeout_secs: 30,
        }
    }

    #[tokio::test]
    async fn test_render_plantuml_invalid_binary_returns_error() {
        let config = DiagramConfig {
            plantuml_path: "nonexistent-plantuml-binary-xyz".to_string(),
            ..config_with_defaults()
        };
        let result = render_plantuml("@startuml\nAlice -> Bob\n@enduml", &config).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ConversionError::DiagramError { diagram_type, .. } => {
                assert_eq!(diagram_type, "plantuml");
            }
            other => panic!("Expected DiagramError, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_render_mermaid_invalid_binary_returns_error() {
        let config = DiagramConfig {
            mermaid_path: "nonexistent-mmdc-binary-xyz".to_string(),
            ..config_with_defaults()
        };
        let result = render_mermaid("graph TD\n  A --> B", &config).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ConversionError::DiagramError { diagram_type, .. } => {
                assert_eq!(diagram_type, "mermaid");
            }
            other => panic!("Expected DiagramError, got: {other:?}"),
        }
    }

    // Integration tests that require real binaries installed
    #[tokio::test]
    async fn test_render_plantuml_integration() {
        // Skip if plantuml not available
        if std::process::Command::new("plantuml")
            .arg("-version")
            .output()
            .is_err()
        {
            eprintln!("Skipping: plantuml not installed");
            return;
        }
        let config = config_with_defaults();
        let svg = render_plantuml("@startuml\nAlice -> Bob: Hello\n@enduml", &config)
            .await
            .unwrap();
        assert!(!svg.is_empty(), "SVG output should not be empty");
        let svg_str = String::from_utf8_lossy(&svg);
        assert!(svg_str.contains("<svg"), "Output should contain <svg tag");
    }

    #[tokio::test]
    async fn test_render_mermaid_integration() {
        // Skip if mmdc not available
        if std::process::Command::new("mmdc")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping: mmdc not installed");
            return;
        }
        let config = config_with_defaults();
        let result = render_mermaid("graph TD\n    A[Start] --> B[End]", &config).await;
        match result {
            Ok(svg) => {
                assert!(!svg.is_empty(), "SVG output should not be empty");
                let svg_str = String::from_utf8_lossy(&svg);
                assert!(svg_str.contains("<svg"), "Output should contain <svg tag");
            }
            Err(ConversionError::DiagramError { message, .. })
                if message.contains("Chrome") || message.contains("puppeteer") =>
            {
                eprintln!("Skipping: mmdc requires Chrome/puppeteer setup");
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }
}
