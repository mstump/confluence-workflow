pub mod client;
pub mod types;
pub mod url;

pub use client::ConfluenceClient;
pub use types::Page;
pub use url::extract_page_id;

use async_trait::async_trait;
use crate::error::ConfluenceError;

/// Trait defining the Confluence REST API surface needed by this application.
///
/// The trait boundary enables test mocks without hitting real Confluence APIs.
#[async_trait]
pub trait ConfluenceApi: Send + Sync {
    /// Fetch a Confluence page by ID, expanding body.storage and version.
    async fn get_page(&self, page_id: &str) -> Result<Page, ConfluenceError>;

    /// Update a Confluence page. The caller is responsible for supplying the
    /// correct next version number (current + 1).
    async fn update_page(
        &self,
        page_id: &str,
        title: &str,
        content: &str,
        version: u32,
    ) -> Result<(), ConfluenceError>;

    /// Upload or replace a named attachment on a page.
    async fn upload_attachment(
        &self,
        page_id: &str,
        filename: &str,
        content: Vec<u8>,
        content_type: &str,
    ) -> Result<(), ConfluenceError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::confluence::types::PageBody;
    use crate::confluence::types::PageVersion;
    use crate::confluence::types::StorageRepresentation;

    struct MockConfluenceClient;

    #[async_trait]
    impl ConfluenceApi for MockConfluenceClient {
        async fn get_page(&self, page_id: &str) -> Result<Page, ConfluenceError> {
            Ok(Page {
                id: page_id.to_string(),
                title: "Mock Page".to_string(),
                body: PageBody {
                    storage: StorageRepresentation {
                        value: "<p>mock</p>".to_string(),
                        representation: "storage".to_string(),
                    },
                },
                version: PageVersion { number: 1 },
            })
        }

        async fn update_page(
            &self,
            _page_id: &str,
            _title: &str,
            _content: &str,
            _version: u32,
        ) -> Result<(), ConfluenceError> {
            Ok(())
        }

        async fn upload_attachment(
            &self,
            _page_id: &str,
            _filename: &str,
            _content: Vec<u8>,
            _content_type: &str,
        ) -> Result<(), ConfluenceError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mock_client_compiles_and_works() {
        let client = MockConfluenceClient;
        let page = client.get_page("99999").await.unwrap();
        assert_eq!(page.id, "99999");
        assert_eq!(page.title, "Mock Page");

        client
            .update_page("99999", "Mock Page", "<p>new</p>", 2)
            .await
            .unwrap();

        client
            .upload_attachment("99999", "test.svg", b"<svg/>".to_vec(), "image/svg+xml")
            .await
            .unwrap();
    }
}
