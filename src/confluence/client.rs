use crate::confluence::{ConfluenceApi, Page};
use crate::error::ConfluenceError;
use async_trait::async_trait;
use base64::Engine as _;
use serde_json::json;

/// HTTP client for the Confluence REST API v1.
///
/// Uses Basic Auth (username + API token) over HTTPS.
pub struct ConfluenceClient {
    client: reqwest::Client,
    base_url: String,
    auth_header: String,
}

impl ConfluenceClient {
    /// Build a new client.
    ///
    /// `base_url` should be the Confluence root (e.g. `https://domain.atlassian.net`).
    /// Trailing slashes are stripped automatically.
    pub fn new(base_url: &str, username: &str, api_token: &str) -> Self {
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
    async fn get_page(&self, page_id: &str) -> Result<Page, ConfluenceError> {
        let url = format!(
            "{}/rest/api/content/{}?expand=body.storage,version",
            self.base_url, page_id
        );
        tracing::debug!("GET {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        match response.status().as_u16() {
            200 => {
                let page = response
                    .json::<Page>()
                    .await
                    .map_err(ConfluenceError::Deserialize)?;
                Ok(page)
            }
            401 => Err(ConfluenceError::Unauthorized),
            404 => Err(ConfluenceError::PageNotFound(page_id.to_string())),
            other => Err(ConfluenceError::UnexpectedStatus(other)),
        }
    }

    async fn update_page(
        &self,
        page_id: &str,
        title: &str,
        content: &str,
        version: u32,
    ) -> Result<(), ConfluenceError> {
        let url = format!("{}/rest/api/content/{}", self.base_url, page_id);

        let body = json!({
            "version": { "number": version, "minorEdit": true },
            "title": title,
            "type": "page",
            "body": {
                "storage": {
                    "value": content,
                    "representation": "storage"
                }
            }
        });

        let response = self
            .client
            .put(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        match response.status().as_u16() {
            200 => {
                tracing::info!("Updated page {} to version {}", page_id, version);
                Ok(())
            }
            401 => Err(ConfluenceError::Unauthorized),
            404 => Err(ConfluenceError::PageNotFound(page_id.to_string())),
            409 => Err(ConfluenceError::VersionConflict {
                page_id: page_id.to_string(),
                attempted_version: version,
            }),
            other => Err(ConfluenceError::UnexpectedStatus(other)),
        }
    }

    async fn upload_attachment(
        &self,
        page_id: &str,
        filename: &str,
        content: Vec<u8>,
        content_type: &str,
    ) -> Result<(), ConfluenceError> {
        // POST /child/attachment creates a new attachment but returns 400 if one with
        // the same filename already exists. To make `upload` idempotent across reruns,
        // look up any existing attachment by filename first and PUT to its /data
        // endpoint to update; otherwise POST to create.
        let existing_id = self.find_attachment_id(page_id, filename).await?;

        let file_part = reqwest::multipart::Part::bytes(content)
            .file_name(filename.to_string())
            .mime_str(content_type)
            .map_err(|e| ConfluenceError::Multipart(e.to_string()))?;
        let form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("minorEdit", "true");

        let url = match &existing_id {
            Some(id) => format!(
                "{}/rest/api/content/{}/child/attachment/{}/data",
                self.base_url, page_id, id
            ),
            None => format!(
                "{}/rest/api/content/{}/child/attachment",
                self.base_url, page_id
            ),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .header("X-Atlassian-Token", "nocheck")
            .multipart(form)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ConfluenceError::AttachmentUpload {
                page_id: page_id.to_string(),
                filename: filename.to_string(),
                status: response.status().as_u16(),
            })
        }
    }
}

impl ConfluenceClient {
    /// Look up an existing attachment on the page by filename.
    /// Returns the attachment's content ID if found, None if no such attachment exists.
    ///
    /// Surfaces auth/permission/server errors as `ConfluenceError` rather than swallowing
    /// them — otherwise a 401/403 on lookup would silently fall through to POST-create and
    /// the caller would see a confusing 400 instead of the real failure.
    async fn find_attachment_id(
        &self,
        page_id: &str,
        filename: &str,
    ) -> Result<Option<String>, ConfluenceError> {
        // Filenames in this workflow are always of the form "diagram_N.svg" — URL-safe — so
        // no percent-encoding is needed here. If we ever support arbitrary filenames the
        // querystring should be encoded.
        let url = format!(
            "{}/rest/api/content/{}/child/attachment?filename={}&limit=1",
            self.base_url, page_id, filename
        );
        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        match response.status().as_u16() {
            200 => {
                let body: serde_json::Value = response
                    .json()
                    .await
                    .map_err(ConfluenceError::Deserialize)?;
                let id = body
                    .get("results")
                    .and_then(|r| r.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|first| first.get("id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Ok(id)
            }
            // The page itself doesn't exist — no attachments to look up, but this is
            // a real "not found" the caller should hear about. Propagate.
            404 => Err(ConfluenceError::PageNotFound(page_id.to_string())),
            401 => Err(ConfluenceError::Unauthorized),
            other => Err(ConfluenceError::UnexpectedStatus(other)),
        }
    }
}

/// Update a page, re-fetching the current version and retrying on 409 conflicts.
///
/// This mitigates the TOCTOU race condition where a concurrent editor changes
/// the page version between our fetch and our update. On 409 we fetch the new
/// current version and retry, up to `max_retries` additional attempts.
pub async fn update_page_with_retry(
    client: &dyn ConfluenceApi,
    page_id: &str,
    content: &str,
    max_retries: u32,
) -> Result<(), ConfluenceError> {
    let mut last_err = None;

    for attempt in 0..=max_retries {
        let page = client.get_page(page_id).await?;
        let next_version = page.version.number + 1;

        match client
            .update_page(page_id, &page.title, content, next_version)
            .await
        {
            Ok(()) => return Ok(()),
            Err(ConfluenceError::VersionConflict { .. }) if attempt < max_retries => {
                tracing::warn!(
                    "Version conflict on page {} (attempt {}/{}), retrying",
                    page_id,
                    attempt + 1,
                    max_retries
                );
                last_err = Some(ConfluenceError::VersionConflict {
                    page_id: page_id.to_string(),
                    attempted_version: next_version,
                });
                // continue loop
            }
            Err(e) => return Err(e),
        }
    }

    Err(last_err.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn page_json(id: &str, version: u32) -> serde_json::Value {
        json!({
            "id": id,
            "title": "Test Page",
            "body": {
                "storage": {
                    "value": "<p>Hello</p>",
                    "representation": "storage"
                }
            },
            "version": { "number": version }
        })
    }

    #[tokio::test]
    async fn test_get_page_200() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/rest/api/content/12345"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(page_json("12345", 7)),
            )
            .mount(&mock_server)
            .await;

        let client = ConfluenceClient::new(&mock_server.uri(), "user", "token");
        let page = client.get_page("12345").await.unwrap();
        assert_eq!(page.id, "12345");
        assert_eq!(page.version.number, 7);
    }

    #[tokio::test]
    async fn test_get_page_401() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/rest/api/content/12345"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = ConfluenceClient::new(&mock_server.uri(), "user", "bad_token");
        let err = client.get_page("12345").await.unwrap_err();
        assert!(matches!(err, ConfluenceError::Unauthorized));
    }

    #[tokio::test]
    async fn test_get_page_404() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/rest/api/content/99999"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let client = ConfluenceClient::new(&mock_server.uri(), "user", "token");
        let err = client.get_page("99999").await.unwrap_err();
        assert!(matches!(err, ConfluenceError::PageNotFound(_)));
    }

    #[tokio::test]
    async fn test_update_page_200() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/rest/api/content/12345"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(page_json("12345", 3)),
            )
            .mount(&mock_server)
            .await;

        let client = ConfluenceClient::new(&mock_server.uri(), "user", "token");
        client
            .update_page("12345", "Test Page", "<p>new</p>", 3)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_update_page_409() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/rest/api/content/12345"))
            .respond_with(ResponseTemplate::new(409))
            .mount(&mock_server)
            .await;

        let client = ConfluenceClient::new(&mock_server.uri(), "user", "token");
        let err = client
            .update_page("12345", "Test Page", "<p>new</p>", 3)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ConfluenceError::VersionConflict {
                page_id: _,
                attempted_version: 3
            }
        ));
    }

    #[tokio::test]
    async fn test_update_page_with_retry_succeeds_on_second_attempt() {
        let mock_server = MockServer::start().await;

        // First GET: returns version 5
        Mock::given(method("GET"))
            .and(path("/rest/api/content/42"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(page_json("42", 5)),
            )
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        // First PUT (version 6): returns 409
        Mock::given(method("PUT"))
            .and(path("/rest/api/content/42"))
            .respond_with(ResponseTemplate::new(409))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        // Second GET: returns version 6 (concurrent edit happened)
        Mock::given(method("GET"))
            .and(path("/rest/api/content/42"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(page_json("42", 6)),
            )
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        // Second PUT (version 7): succeeds
        Mock::given(method("PUT"))
            .and(path("/rest/api/content/42"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(page_json("42", 7)),
            )
            .mount(&mock_server)
            .await;

        let client = ConfluenceClient::new(&mock_server.uri(), "user", "token");
        update_page_with_retry(&client, "42", "<p>new</p>", 3)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_update_page_with_retry_exhausted() {
        let mock_server = MockServer::start().await;

        // GET always succeeds
        Mock::given(method("GET"))
            .and(path("/rest/api/content/42"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(page_json("42", 1)),
            )
            .mount(&mock_server)
            .await;

        // PUT always 409
        Mock::given(method("PUT"))
            .and(path("/rest/api/content/42"))
            .respond_with(ResponseTemplate::new(409))
            .mount(&mock_server)
            .await;

        let client = ConfluenceClient::new(&mock_server.uri(), "user", "token");
        let err = update_page_with_retry(&client, "42", "<p>new</p>", 2)
            .await
            .unwrap_err();
        assert!(matches!(err, ConfluenceError::VersionConflict { .. }));
    }

    #[tokio::test]
    async fn test_upload_attachment_sends_x_atlassian_token_header() {
        let mock_server = MockServer::start().await;
        // upload_attachment first GETs to check whether the attachment already exists;
        // return an empty results list so the create path runs.
        Mock::given(method("GET"))
            .and(path("/rest/api/content/12345/child/attachment"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results":[]})))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/rest/api/content/12345/child/attachment"))
            .and(header("X-Atlassian-Token", "nocheck"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results":[]})))
            .mount(&mock_server)
            .await;

        let client = ConfluenceClient::new(&mock_server.uri(), "user", "token");
        client
            .upload_attachment("12345", "diagram.svg", b"<svg/>".to_vec(), "image/svg+xml")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_upload_attachment_updates_existing_via_data_endpoint() {
        let mock_server = MockServer::start().await;
        // Lookup returns one existing attachment with id "att-99".
        Mock::given(method("GET"))
            .and(path("/rest/api/content/12345/child/attachment"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{"id": "att-99"}]
            })))
            .mount(&mock_server)
            .await;
        // The PUT-via-POST to /data must be hit; the bare POST must not.
        Mock::given(method("POST"))
            .and(path("/rest/api/content/12345/child/attachment/att-99/data"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results":[]})))
            .mount(&mock_server)
            .await;

        let client = ConfluenceClient::new(&mock_server.uri(), "user", "token");
        client
            .upload_attachment("12345", "diagram.svg", b"<svg/>".to_vec(), "image/svg+xml")
            .await
            .unwrap();
    }
}
