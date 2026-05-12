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

/// Render Mermaid content to SVG bytes via the `mermaid-core` library (in-process).
///
/// Synchronous: no subprocess, no temp files, no async wait. Diagrams in this
/// workflow are small enough that the CPU work completes well under a frame's
/// worth of time; spawn_blocking would be over-engineering. The caller is
/// already async but doesn't need to .await this.
pub fn render_mermaid(content: &str) -> Result<Vec<u8>, ConversionError> {
    let output = mermaid_core::render(content, &mermaid_core::RenderConfig::default())
        .map_err(|e| ConversionError::DiagramError {
            diagram_type: "mermaid".to_string(),
            message: format!("mermaid-core failed: {e}"),
        })?;

    let svg = output.into_svg().map_err(|e| ConversionError::DiagramError {
        diagram_type: "mermaid".to_string(),
        message: format!("mermaid-core returned non-SVG output: {e}"),
    })?;

    if svg.is_empty() {
        return Err(ConversionError::DiagramError {
            diagram_type: "mermaid".to_string(),
            message: "mermaid-core produced empty SVG output".to_string(),
        });
    }

    Ok(svg.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DiagramConfig;

    fn config_with_defaults() -> DiagramConfig {
        DiagramConfig {
            plantuml_path: "plantuml".to_string(),
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

    #[test]
    fn test_render_mermaid_invalid_syntax_returns_error() {
        // With the in-process renderer, the only way `render_mermaid` can fail
        // is invalid mermaid syntax. Garbage in → DiagramError out.
        let result = render_mermaid("this is not mermaid syntax !!!");
        assert!(result.is_err(), "expected error from invalid mermaid syntax");
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

    #[test]
    fn test_render_mermaid_basic() {
        // mermaid-core is a compile-time dep — no installation, no skip path.
        let svg = render_mermaid("graph TD\n    A[Start] --> B[End]")
            .expect("render_mermaid should succeed for a basic flowchart");
        assert!(!svg.is_empty(), "SVG output should not be empty");
        let svg_str = String::from_utf8_lossy(&svg);
        assert!(svg_str.contains("<svg"), "Output should contain <svg tag");
    }
}
