use super::*;
use crate::error::ConversionError;

/// Mock converter that returns a fixed result — proves the trait is implementable.
struct MockConverter;

#[async_trait::async_trait]
impl Converter for MockConverter {
    async fn convert(&self, markdown: &str) -> Result<ConvertResult, ConversionError> {
        Ok(ConvertResult {
            storage_xml: format!("<p>{}</p>", markdown),
            attachments: vec![Attachment {
                filename: "test.svg".to_string(),
                content: b"<svg/>".to_vec(),
                content_type: "image/svg+xml".to_string(),
            }],
        })
    }
}

#[tokio::test]
async fn test_mock_converter_compiles_and_works() {
    let converter = MockConverter;
    let result = converter.convert("hello").await.unwrap();
    assert_eq!(result.storage_xml, "<p>hello</p>");
    assert_eq!(result.attachments.len(), 1);
    assert_eq!(result.attachments[0].filename, "test.svg");
    assert_eq!(result.attachments[0].content, b"<svg/>");
    assert_eq!(result.attachments[0].content_type, "image/svg+xml");
}

#[test]
fn test_convert_result_has_storage_xml_and_attachments() {
    let result = ConvertResult {
        storage_xml: "<p>test</p>".to_string(),
        attachments: vec![],
    };
    assert_eq!(result.storage_xml, "<p>test</p>");
    assert!(result.attachments.is_empty());
}

#[test]
fn test_attachment_struct_fields() {
    let att = Attachment {
        filename: "diagram.svg".to_string(),
        content: vec![1, 2, 3],
        content_type: "image/svg+xml".to_string(),
    };
    assert_eq!(att.filename, "diagram.svg");
    assert_eq!(att.content, vec![1, 2, 3]);
    assert_eq!(att.content_type, "image/svg+xml");
}

#[test]
fn test_conversion_error_variants() {
    let render_err = ConversionError::RenderError("bad markdown".to_string());
    assert!(format!("{}", render_err).contains("bad markdown"));

    let diagram_err = ConversionError::DiagramError {
        diagram_type: "plantuml".to_string(),
        message: "jar not found".to_string(),
    };
    assert!(format!("{}", diagram_err).contains("plantuml"));
    assert!(format!("{}", diagram_err).contains("jar not found"));

    let timeout_err = ConversionError::DiagramTimeout {
        diagram_type: "mermaid".to_string(),
        timeout_secs: 30,
    };
    assert!(format!("{}", timeout_err).contains("30s"));
    assert!(format!("{}", timeout_err).contains("mermaid"));
}

#[test]
fn test_conversion_error_into_app_error() {
    use crate::error::AppError;

    let conversion_err = ConversionError::RenderError("test".to_string());
    let app_err: AppError = conversion_err.into();
    assert!(format!("{}", app_err).contains("test"));
}
