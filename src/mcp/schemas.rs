//! MCP request/response schemas for the websearch tool

use crate::types::SearchResult;
use serde::{Deserialize, Serialize};

/// Response from the web_search MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResponse {
    /// The original query
    pub query: String,

    /// Search results
    pub results: Vec<SearchResult>,

    /// Provider that was used
    pub provider: String,

    /// Total number of results
    pub result_count: usize,
}

impl WebSearchResponse {
    /// Create a new WebSearchResponse
    pub fn new(query: String, results: Vec<SearchResult>, provider: String) -> Self {
        let result_count = results.len();
        Self {
            query,
            results,
            provider,
            result_count,
        }
    }
}
