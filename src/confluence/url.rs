use crate::error::ConfluenceError;
use std::sync::OnceLock;
use regex::Regex;

static RE_EDIT_V2: OnceLock<Regex> = OnceLock::new();
static RE_PAGES: OnceLock<Regex> = OnceLock::new();
static RE_PAGE_ID: OnceLock<Regex> = OnceLock::new();

fn re_edit_v2() -> &'static Regex {
    RE_EDIT_V2.get_or_init(|| Regex::new(r"/pages/edit-v2/(\d+)").unwrap())
}

fn re_pages() -> &'static Regex {
    RE_PAGES.get_or_init(|| Regex::new(r"/pages/(\d+)").unwrap())
}

fn re_page_id() -> &'static Regex {
    RE_PAGE_ID.get_or_init(|| Regex::new(r"[?&]pageId=(\d+)").unwrap())
}

/// Extract the page ID from a Confluence page URL.
///
/// Supports three URL patterns:
/// - `/pages/edit-v2/12345/...` (checked first — more specific)
/// - `/pages/12345/...`
/// - `?pageId=12345` or `&pageId=12345`
pub fn extract_page_id(url: &str) -> Result<String, ConfluenceError> {
    if let Some(caps) = re_edit_v2().captures(url) {
        return Ok(caps[1].to_string());
    }
    if let Some(caps) = re_pages().captures(url) {
        return Ok(caps[1].to_string());
    }
    if let Some(caps) = re_page_id().captures(url) {
        return Ok(caps[1].to_string());
    }
    Err(ConfluenceError::InvalidPageUrl(url.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_page_id_slash_pages() {
        let result = extract_page_id("/wiki/spaces/SPACE/pages/12345/Title");
        assert_eq!(result.unwrap(), "12345");
    }

    #[test]
    fn test_extract_page_id_edit_v2() {
        let result = extract_page_id("/pages/edit-v2/67890/Title");
        assert_eq!(result.unwrap(), "67890");
    }

    #[test]
    fn test_extract_page_id_query_param() {
        let result = extract_page_id("?pageId=11111");
        assert_eq!(result.unwrap(), "11111");
    }

    #[test]
    fn test_extract_page_id_full_url() {
        let result = extract_page_id(
            "https://domain.atlassian.net/wiki/spaces/SPACE/pages/12345/Title",
        );
        assert_eq!(result.unwrap(), "12345");
    }

    #[test]
    fn test_extract_page_id_no_match() {
        let result = extract_page_id("https://example.com/no-page-id");
        assert!(matches!(result, Err(ConfluenceError::InvalidPageUrl(_))));
    }
}
