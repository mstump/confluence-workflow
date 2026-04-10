// Stub — full implementation in Task 2
use crate::error::ConfluenceError;
use crate::confluence::{ConfluenceApi, Page};
use async_trait::async_trait;

#[allow(dead_code)]
pub struct ConfluenceClient {
    pub(crate) client: reqwest::Client,
    pub(crate) base_url: String,
    pub(crate) auth_header: String,
}

impl ConfluenceClient {
    pub fn new(base_url: &str, username: &str, api_token: &str) -> Self {
        use base64::Engine as _;
        let credentials = format!("{}:{}", username, api_token);
        let auth_header = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials)
        );
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build reqwest client");
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_header,
        }
    }
}

#[async_trait]
impl ConfluenceApi for ConfluenceClient {
    async fn get_page(&self, _page_id: &str) -> Result<Page, ConfluenceError> {
        unimplemented!("Task 2")
    }

    async fn update_page(
        &self,
        _page_id: &str,
        _title: &str,
        _content: &str,
        _version: u32,
    ) -> Result<(), ConfluenceError> {
        unimplemented!("Task 2")
    }

    async fn upload_attachment(
        &self,
        _page_id: &str,
        _filename: &str,
        _content: Vec<u8>,
        _content_type: &str,
    ) -> Result<(), ConfluenceError> {
        unimplemented!("Task 2")
    }
}

pub async fn update_page_with_retry(
    _client: &dyn ConfluenceApi,
    _page_id: &str,
    _content: &str,
    _max_retries: u32,
) -> Result<(), ConfluenceError> {
    unimplemented!("Task 2")
}
