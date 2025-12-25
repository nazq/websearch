//! Real provider integration tests
//!
//! These tests make actual API calls to search providers.
//! They gracefully skip when API keys are not configured.
//!
//! To run these tests:
//! 1. Copy `.cargo/config.toml` and uncomment the API keys you have
//! 2. Run: `cargo test --test provider_integration`
//!
//! Tests will print "ignored, <KEY> not set" when skipping.

use websearch::{web_search, SearchOptions};

/// Helper macro to skip test if env var is not set
macro_rules! require_env {
    ($key:expr) => {
        match std::env::var($key) {
            Ok(val) if !val.is_empty() => val,
            _ => {
                eprintln!(
                    "test {} ... ignored, {} not set",
                    stdext::function_name!(),
                    $key
                );
                return;
            }
        }
    };
}

/// Workaround for function_name in stable Rust
mod stdext {
    macro_rules! function_name {
        () => {{
            fn f() {}
            fn type_name_of<T>(_: T) -> &'static str {
                std::any::type_name::<T>()
            }
            let name = type_name_of(f);
            // Remove "::f" suffix and get just the function name
            &name[..name.len() - 3]
                .rsplit("::")
                .next()
                .unwrap_or("unknown")
        }};
    }
    pub(crate) use function_name;
}

// =============================================================================
// WebSearchAPI.ai Tests
// =============================================================================

#[tokio::test]
async fn test_websearchapi_real_search() {
    let api_key = require_env!("WEBSEARCHAPI_KEY");

    let provider = websearch::providers::WebSearchApiProvider::new(&api_key)
        .expect("Failed to create WebSearchApiProvider");

    let options = SearchOptions {
        query: "rust programming language".to_string(),
        max_results: Some(3),
        provider: Box::new(provider),
        ..Default::default()
    };

    let results = web_search(options).await.expect("Search failed");

    assert!(!results.is_empty(), "Should return results");
    assert!(results[0].url.starts_with("http"), "URL should be valid");
    assert!(!results[0].title.is_empty(), "Title should not be empty");

    // WebSearchAPI.ai should return content
    if let Some(content) = &results[0].content {
        assert!(!content.is_empty(), "Content should not be empty");
        assert!(
            results[0].word_count.unwrap_or(0) > 0,
            "Word count should be positive"
        );
    }
}

#[tokio::test]
async fn test_websearchapi_content_extraction() {
    let api_key = require_env!("WEBSEARCHAPI_KEY");

    let provider = websearch::providers::WebSearchApiProvider::new(&api_key)
        .expect("Failed to create provider")
        .with_content(true)
        .with_content_format("markdown")
        .expect("Failed to set format");

    let options = SearchOptions {
        query: "what is rust programming".to_string(),
        max_results: Some(1),
        provider: Box::new(provider),
        ..Default::default()
    };

    let results = web_search(options).await.expect("Search failed");

    assert!(!results.is_empty(), "Should return results");

    let result = &results[0];
    assert!(result.content.is_some(), "Content should be present");
    assert_eq!(
        result.content_format,
        Some("markdown".to_string()),
        "Format should be markdown"
    );
}

// =============================================================================
// Tavily Tests
// =============================================================================

#[tokio::test]
async fn test_tavily_real_search() {
    let api_key = require_env!("TAVILY_API_KEY");

    let provider =
        websearch::providers::TavilyProvider::new(&api_key).expect("Failed to create provider");

    let options = SearchOptions {
        query: "rust async await".to_string(),
        max_results: Some(3),
        provider: Box::new(provider),
        ..Default::default()
    };

    let results = web_search(options).await.expect("Search failed");

    assert!(!results.is_empty(), "Should return results");
    assert!(results[0].url.starts_with("http"), "URL should be valid");
    assert!(!results[0].title.is_empty(), "Title should not be empty");
    assert!(results[0].snippet.is_some(), "Snippet should be present");
}

#[tokio::test]
async fn test_tavily_advanced_search() {
    let api_key = require_env!("TAVILY_API_KEY");

    let provider = websearch::providers::TavilyProvider::new_advanced(&api_key)
        .expect("Failed to create provider")
        .with_answer(true);

    let options = SearchOptions {
        query: "what is the rust borrow checker".to_string(),
        max_results: Some(3),
        provider: Box::new(provider),
        ..Default::default()
    };

    let results = web_search(options).await.expect("Search failed");
    assert!(!results.is_empty(), "Should return results");
}

// =============================================================================
// Exa Tests
// =============================================================================

#[tokio::test]
async fn test_exa_real_search() {
    let api_key = require_env!("EXA_API_KEY");

    let provider =
        websearch::providers::ExaProvider::new(&api_key).expect("Failed to create provider");

    let options = SearchOptions {
        query: "rust memory safety".to_string(),
        max_results: Some(3),
        provider: Box::new(provider),
        ..Default::default()
    };

    let results = web_search(options).await.expect("Search failed");

    assert!(!results.is_empty(), "Should return results");
    assert!(results[0].url.starts_with("http"), "URL should be valid");
}

#[tokio::test]
async fn test_exa_with_contents() {
    let api_key = require_env!("EXA_API_KEY");

    let provider = websearch::providers::ExaProvider::new(&api_key)
        .expect("Failed to create provider")
        .with_contents(true);

    let options = SearchOptions {
        query: "rust ownership model explained".to_string(),
        max_results: Some(2),
        provider: Box::new(provider),
        ..Default::default()
    };

    let results = web_search(options).await.expect("Search failed");

    assert!(!results.is_empty(), "Should return results");

    // When with_contents is true, content should be populated
    if let Some(content) = &results[0].content {
        assert!(!content.is_empty(), "Content should not be empty");
    }
}

// =============================================================================
// Google Tests
// =============================================================================

#[tokio::test]
async fn test_google_real_search() {
    let api_key = require_env!("GOOGLE_API_KEY");
    let cx = require_env!("GOOGLE_CX");

    let provider = websearch::providers::GoogleProvider::new(&api_key, &cx)
        .expect("Failed to create provider");

    let options = SearchOptions {
        query: "rust programming tutorials".to_string(),
        max_results: Some(5),
        provider: Box::new(provider),
        ..Default::default()
    };

    let results = web_search(options).await.expect("Search failed");

    assert!(!results.is_empty(), "Should return results");
    assert!(results[0].url.starts_with("http"), "URL should be valid");
    assert!(results[0].domain.is_some(), "Domain should be present");
}

// =============================================================================
// SerpAPI Tests
// =============================================================================

#[tokio::test]
async fn test_serpapi_real_search() {
    let api_key = require_env!("SERPAPI_API_KEY");

    let provider =
        websearch::providers::SerpApiProvider::new(&api_key).expect("Failed to create provider");

    let options = SearchOptions {
        query: "rust vs go performance".to_string(),
        max_results: Some(5),
        provider: Box::new(provider),
        ..Default::default()
    };

    let results = web_search(options).await.expect("Search failed");

    assert!(!results.is_empty(), "Should return results");
    assert!(results[0].url.starts_with("http"), "URL should be valid");
}

// =============================================================================
// ArXiv Tests (no API key required)
// =============================================================================

#[tokio::test]
async fn test_arxiv_real_search() {
    let provider = websearch::providers::ArxivProvider::new();

    let options = SearchOptions {
        query: "machine learning rust".to_string(),
        max_results: Some(3),
        provider: Box::new(provider),
        ..Default::default()
    };

    let results = web_search(options).await.expect("Search failed");

    // ArXiv may return 0 results for niche queries, that's OK
    if !results.is_empty() {
        assert!(
            results[0].url.contains("arxiv.org"),
            "URL should be arxiv.org"
        );
        assert_eq!(
            results[0].domain,
            Some("arxiv.org".to_string()),
            "Domain should be arxiv.org"
        );
    }
}

#[tokio::test]
async fn test_arxiv_by_id() {
    let provider = websearch::providers::ArxivProvider::new();

    let options = SearchOptions {
        query: "".to_string(),
        id_list: Some("2301.00001".to_string()),
        max_results: Some(1),
        provider: Box::new(provider),
        ..Default::default()
    };

    let results = web_search(options).await.expect("Search failed");

    assert!(!results.is_empty(), "Should return the paper");
    assert!(
        results[0].url.contains("2301.00001"),
        "URL should contain paper ID"
    );
}

// =============================================================================
// DuckDuckGo Tests (no API key required, but may be rate-limited)
// =============================================================================

#[tokio::test]
#[ignore = "DuckDuckGo may rate-limit automated requests"]
async fn test_duckduckgo_real_search() {
    let provider = websearch::providers::DuckDuckGoProvider::new();

    let options = SearchOptions {
        query: "rust programming language".to_string(),
        max_results: Some(5),
        provider: Box::new(provider),
        ..Default::default()
    };

    let results = web_search(options).await.expect("Search failed");

    assert!(!results.is_empty(), "Should return results");
    assert!(results[0].url.starts_with("http"), "URL should be valid");
}
