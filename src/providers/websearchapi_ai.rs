//! WebSearchAPI.ai provider
//!
//! Google-powered search with built-in content extraction for LLM applications.
//! Provides markdown-formatted content ready for AI consumption.

use crate::{
    error::{SearchError, SearchResult},
    types::{SearchOptions, SearchProvider, SearchResult as SearchResultType},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// WebSearchAPI.ai search request structure
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebSearchApiRequest {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_content: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_length: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safe_search: Option<bool>,
}

/// Individual search result from WebSearchAPI.ai (in "organic" array)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebSearchApiResult {
    title: String,
    url: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    position: Option<u32>,
    #[serde(default)]
    score: Option<f64>,
}

/// WebSearchAPI.ai API response structure
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebSearchApiResponse {
    #[serde(default)]
    organic: Vec<WebSearchApiResult>,
    #[serde(default)]
    answer: Option<String>,
    #[serde(default)]
    response_time: Option<f64>,
}

/// WebSearchAPI.ai search provider
///
/// Google-powered search with built-in content extraction optimized for LLMs.
///
/// # Example
/// ```no_run
/// use websearch::providers::WebSearchApiProvider;
///
/// let provider = WebSearchApiProvider::new("your_api_key")?;
/// // Or with content extraction disabled for faster responses:
/// let provider = WebSearchApiProvider::new("your_api_key")?
///     .with_content(false);
/// # Ok::<(), websearch::error::SearchError>(())
/// ```
#[derive(Debug, Clone)]
pub struct WebSearchApiProvider {
    api_key: String,
    base_url: String,
    include_content: bool,
    content_format: String,
    include_domains: Option<Vec<String>>,
    exclude_domains: Option<Vec<String>>,
}

impl WebSearchApiProvider {
    /// Create a new WebSearchAPI.ai provider with the given API key
    ///
    /// By default, content extraction is enabled with markdown format.
    pub fn new(api_key: &str) -> SearchResult<Self> {
        if api_key.is_empty() {
            return Err(SearchError::ConfigError(
                "WebSearchAPI.ai API key is required".to_string(),
            ));
        }

        Ok(Self {
            api_key: api_key.to_string(),
            base_url: "https://api.websearchapi.ai/ai-search".to_string(),
            include_content: true,
            content_format: "markdown".to_string(),
            include_domains: None,
            exclude_domains: None,
        })
    }

    /// Enable or disable content extraction
    ///
    /// When enabled, the API returns full page content in addition to snippets.
    /// Disabling this can reduce latency and credit usage.
    pub fn with_content(mut self, include: bool) -> Self {
        self.include_content = include;
        self
    }

    /// Set content format
    ///
    /// Supported formats: "markdown" (default), "text", "html"
    pub fn with_content_format(mut self, format: &str) -> SearchResult<Self> {
        let valid_formats = ["markdown", "text", "html"];
        if !valid_formats.contains(&format) {
            return Err(SearchError::ConfigError(format!(
                "Invalid content format '{}'. Must be one of: {:?}",
                format, valid_formats
            )));
        }
        self.content_format = format.to_string();
        Ok(self)
    }

    /// Limit search to specific domains
    ///
    /// # Example
    /// ```no_run
    /// # use websearch::providers::WebSearchApiProvider;
    /// let provider = WebSearchApiProvider::new("key")?
    ///     .with_include_domains(vec!["docs.rs".to_string(), "crates.io".to_string()]);
    /// # Ok::<(), websearch::error::SearchError>(())
    /// ```
    pub fn with_include_domains(mut self, domains: Vec<String>) -> Self {
        self.include_domains = Some(domains);
        self
    }

    /// Exclude specific domains from search
    pub fn with_exclude_domains(mut self, domains: Vec<String>) -> Self {
        self.exclude_domains = Some(domains);
        self
    }

    /// Set custom base URL (for testing or enterprise endpoints)
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }
}

#[async_trait::async_trait]
impl SearchProvider for WebSearchApiProvider {
    fn name(&self) -> &str {
        "websearchapi_ai"
    }

    async fn search(&self, options: &SearchOptions) -> SearchResult<Vec<SearchResultType>> {
        if options.query.is_empty() {
            return Err(SearchError::InvalidInput(
                "Query cannot be empty".to_string(),
            ));
        }

        let timeout_duration = std::time::Duration::from_millis(options.timeout.unwrap_or(15000));
        let client = reqwest::Client::builder()
            .timeout(timeout_duration)
            .build()
            .map_err(|e| {
                SearchError::ConfigError(format!("Failed to create HTTP client: {e}"))
            })?;

        // WebSearchAPI.ai max is 20
        let max_results = options.max_results.unwrap_or(5).min(20);

        let request_body = WebSearchApiRequest {
            query: options.query.clone(),
            max_results: Some(max_results),
            include_content: Some(self.include_content),
            content_format: if self.include_content {
                Some(self.content_format.clone())
            } else {
                None
            },
            content_length: if self.include_content {
                Some("medium".to_string())
            } else {
                None
            },
            include_domains: self.include_domains.clone(),
            exclude_domains: self.exclude_domains.clone(),
            country: options.region.clone(),
            language: options.language.clone(),
            safe_search: options.safe_search.as_ref().map(|s| s.to_string() != "off"),
        };

        let response = client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SearchError::HttpError {
                message: format!("Failed to send request to WebSearchAPI.ai: {e}"),
                status_code: None,
                response_body: None,
            })?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| SearchError::HttpError {
            message: format!("Failed to read WebSearchAPI.ai response: {e}"),
            status_code: Some(status.as_u16()),
            response_body: None,
        })?;

        if !status.is_success() {
            let error_msg = match status.as_u16() {
                400 => "Bad request - check your query parameters",
                401 => "Unauthorized - check your API key",
                402 => "Payment required - check your account credits",
                403 => "Forbidden - API key may be invalid or suspended",
                429 => "Rate limit exceeded - too many requests",
                500..=599 => "WebSearchAPI.ai server error - try again later",
                _ => "Unknown error occurred",
            };

            return Err(SearchError::HttpError {
                message: format!("WebSearchAPI.ai API error ({status}): {error_msg}"),
                status_code: Some(status.as_u16()),
                response_body: Some(response_text),
            });
        }

        let api_response: WebSearchApiResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                SearchError::ParseError(format!(
                    "Failed to parse WebSearchAPI.ai response: {e}. Response: {response_text}"
                ))
            })?;

        // Convert WebSearchAPI.ai results to our standard format
        let results: Vec<SearchResultType> = api_response
            .organic
            .into_iter()
            .map(|result| {
                // Store the original result as raw data
                let raw_value = serde_json::to_value(&result).unwrap_or_default();

                // Calculate word count if content is present
                let word_count = result.content.as_ref().map(|c| {
                    c.split_whitespace().count() as u32
                });

                // Determine content format if content is present
                let content_format = if result.content.is_some() {
                    Some(self.content_format.clone())
                } else {
                    None
                };

                SearchResultType {
                    url: result.url.clone(),
                    title: result.title,
                    snippet: result.description,
                    domain: extract_domain(&result.url),
                    published_date: None, // WebSearchAPI.ai doesn't provide this
                    provider: Some("websearchapi_ai".to_string()),
                    raw: Some(raw_value),
                    // New LLM-ready content fields
                    content: result.content,
                    content_format,
                    word_count,
                }
            })
            .collect();

        Ok(results)
    }

    fn config(&self) -> HashMap<String, String> {
        let mut config = HashMap::new();
        config.insert("provider".to_string(), "websearchapi_ai".to_string());
        config.insert("api_key".to_string(), "***".to_string());
        config.insert("base_url".to_string(), self.base_url.clone());
        config.insert(
            "include_content".to_string(),
            self.include_content.to_string(),
        );
        config.insert("content_format".to_string(), self.content_format.clone());
        if let Some(ref domains) = self.include_domains {
            config.insert("include_domains".to_string(), domains.join(","));
        }
        if let Some(ref domains) = self.exclude_domains {
            config.insert("exclude_domains".to_string(), domains.join(","));
        }
        config
    }
}

/// Extract domain from URL
fn extract_domain(url: &str) -> Option<String> {
    if let Ok(parsed_url) = url::Url::parse(url) {
        parsed_url.host_str().map(|host| host.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websearchapi_provider_new() {
        // Valid API key
        let provider = WebSearchApiProvider::new("test-api-key");
        assert!(provider.is_ok());

        // Empty API key
        let provider = WebSearchApiProvider::new("");
        assert!(provider.is_err());
        match provider.unwrap_err() {
            SearchError::ConfigError(msg) => assert!(msg.contains("required")),
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_websearchapi_provider_configuration() {
        let provider = WebSearchApiProvider::new("test-api-key")
            .unwrap()
            .with_content(false);

        assert!(!provider.include_content);
        assert_eq!(provider.content_format, "markdown");
    }

    #[test]
    fn test_websearchapi_content_format_validation() {
        let provider = WebSearchApiProvider::new("test-api-key").unwrap();

        // Valid formats
        assert!(provider.clone().with_content_format("markdown").is_ok());
        assert!(provider.clone().with_content_format("text").is_ok());
        assert!(provider.clone().with_content_format("html").is_ok());

        // Invalid format
        assert!(provider.with_content_format("invalid").is_err());
    }

    #[test]
    fn test_websearchapi_domain_filters() {
        let provider = WebSearchApiProvider::new("test-api-key")
            .unwrap()
            .with_include_domains(vec!["docs.rs".to_string()])
            .with_exclude_domains(vec!["spam.com".to_string()]);

        assert_eq!(
            provider.include_domains,
            Some(vec!["docs.rs".to_string()])
        );
        assert_eq!(
            provider.exclude_domains,
            Some(vec!["spam.com".to_string()])
        );
    }

    #[test]
    fn test_websearchapi_provider_name() {
        let provider = WebSearchApiProvider::new("test-api-key").unwrap();
        assert_eq!(provider.name(), "websearchapi_ai");
    }

    #[test]
    fn test_websearchapi_provider_config() {
        let provider = WebSearchApiProvider::new("test-api-key").unwrap();
        let config = provider.config();

        assert_eq!(config.get("provider"), Some(&"websearchapi_ai".to_string()));
        assert_eq!(config.get("api_key"), Some(&"***".to_string()));
        assert!(config.contains_key("base_url"));
        assert!(config.contains_key("include_content"));
        assert!(config.contains_key("content_format"));
    }

    #[tokio::test]
    async fn test_websearchapi_search_empty_query() {
        let provider = WebSearchApiProvider::new("test-api-key").unwrap();
        let options = SearchOptions {
            query: "".to_string(),
            provider: Box::new(provider),
            ..Default::default()
        };

        let result = options.provider.search(&options).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            SearchError::InvalidInput(msg) => assert!(msg.contains("empty")),
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain("http://subdomain.example.org"),
            Some("subdomain.example.org".to_string())
        );
        assert_eq!(extract_domain("invalid-url"), None);
        assert_eq!(extract_domain(""), None);
    }
}
