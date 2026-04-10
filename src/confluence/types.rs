use serde::Deserialize;

/// Full Confluence page response from the REST API v1.
#[derive(Debug, Clone, Deserialize)]
pub struct Page {
    pub id: String,
    pub title: String,
    pub body: PageBody,
    pub version: PageVersion,
}

/// The body container returned when `expand=body.storage` is requested.
#[derive(Debug, Clone, Deserialize)]
pub struct PageBody {
    pub storage: StorageRepresentation,
}

/// The Confluence storage format (XML) and its representation name.
#[derive(Debug, Clone, Deserialize)]
pub struct StorageRepresentation {
    pub value: String,
    pub representation: String,
}

/// The version object returned alongside the page.
#[derive(Debug, Clone, Deserialize)]
pub struct PageVersion {
    pub number: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_deserializes_from_confluence_json() {
        let json = r#"{"id":"12345","title":"Test Page","body":{"storage":{"value":"<p>Hello</p>","representation":"storage"}},"version":{"number":42}}"#;
        let page: Page = serde_json::from_str(json).unwrap();
        assert_eq!(page.id, "12345");
        assert_eq!(page.title, "Test Page");
        assert_eq!(page.body.storage.value, "<p>Hello</p>");
        assert_eq!(page.body.storage.representation, "storage");
        assert_eq!(page.version.number, 42);
    }
}
