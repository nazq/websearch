//! MCP Server implementation for websearch
//!
//! Provides a `web_search` tool that can be used by AI assistants
//! to search the web using various providers.

use crate::mcp::schemas::WebSearchResponse;
use crate::providers::*;
use crate::types::{DebugOptions, SearchOptions, SearchProvider};
use crate::web_search;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env;

/// MCP request parameters for web search
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SearchRequest {
    /// The search query string
    pub query: String,

    /// Maximum number of results to return (1-50, default: 5)
    #[serde(default = "default_max_results")]
    pub max_results: u32,

    /// Whether to include full page content when available (default: true)
    #[serde(default = "default_include_content")]
    pub include_content: bool,

    /// Provider to use: duckduckgo, tavily, exa, google, serpapi, arxiv, websearchapi_ai
    /// If not specified, uses the default provider (duckduckgo or WEBSEARCH_DEFAULT_PROVIDER env var)
    #[serde(default)]
    pub provider: Option<String>,
}

fn default_max_results() -> u32 {
    5
}

fn default_include_content() -> bool {
    true
}

/// WebSearch MCP Server
///
/// Exposes web search capabilities as MCP tools that can be used by AI assistants.
#[derive(Clone)]
pub struct WebSearchMcpServer {
    tool_router: ToolRouter<Self>,
    default_provider: String,
}

impl WebSearchMcpServer {
    /// Create a new WebSearch MCP server
    pub fn new() -> Self {
        let default_provider =
            env::var("WEBSEARCH_DEFAULT_PROVIDER").unwrap_or_else(|_| "duckduckgo".to_string());

        Self {
            tool_router: Self::tool_router(),
            default_provider,
        }
    }

    /// Create a new WebSearch MCP server with a specific default provider
    pub fn with_default_provider(provider: &str) -> Self {
        Self {
            tool_router: Self::tool_router(),
            default_provider: provider.to_string(),
        }
    }

    /// Get a provider instance by name
    fn get_provider(
        &self,
        provider_name: &str,
    ) -> Result<Box<dyn SearchProvider>, McpError> {
        match provider_name {
            "duckduckgo" => Ok(Box::new(DuckDuckGoProvider::new())),
            "arxiv" => Ok(Box::new(ArxivProvider::new())),
            "tavily" => {
                let api_key = env::var("TAVILY_API_KEY").map_err(|_| {
                    McpError::invalid_params(
                        "TAVILY_API_KEY environment variable is required",
                        None,
                    )
                })?;
                TavilyProvider::new(&api_key).map(|p| Box::new(p) as Box<dyn SearchProvider>)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))
            }
            "exa" => {
                let api_key = env::var("EXA_API_KEY").map_err(|_| {
                    McpError::invalid_params(
                        "EXA_API_KEY environment variable is required",
                        None,
                    )
                })?;
                ExaProvider::new(&api_key).map(|p| Box::new(p) as Box<dyn SearchProvider>)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))
            }
            "google" => {
                let api_key = env::var("GOOGLE_API_KEY").map_err(|_| {
                    McpError::invalid_params(
                        "GOOGLE_API_KEY environment variable is required",
                        None,
                    )
                })?;
                let cx = env::var("GOOGLE_CX").map_err(|_| {
                    McpError::invalid_params("GOOGLE_CX environment variable is required", None)
                })?;
                GoogleProvider::new(&api_key, &cx)
                    .map(|p| Box::new(p) as Box<dyn SearchProvider>)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))
            }
            "serpapi" => {
                let api_key = env::var("SERPAPI_API_KEY").map_err(|_| {
                    McpError::invalid_params(
                        "SERPAPI_API_KEY environment variable is required",
                        None,
                    )
                })?;
                SerpApiProvider::new(&api_key)
                    .map(|p| Box::new(p) as Box<dyn SearchProvider>)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))
            }
            "websearchapi_ai" => {
                let api_key = env::var("WEBSEARCHAPI_KEY").map_err(|_| {
                    McpError::invalid_params(
                        "WEBSEARCHAPI_KEY environment variable is required",
                        None,
                    )
                })?;
                WebSearchApiProvider::new(&api_key)
                    .map(|p| Box::new(p) as Box<dyn SearchProvider>)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))
            }
            _ => Err(McpError::invalid_params(
                format!(
                    "Unknown provider '{}'. Available providers: duckduckgo, tavily, exa, google, serpapi, arxiv, websearchapi_ai",
                    provider_name
                ),
                None,
            )),
        }
    }
}

impl Default for WebSearchMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl WebSearchMcpServer {
    /// Search the web using various providers
    ///
    /// Returns search results with URLs, titles, snippets, and optionally full page content.
    /// Supports multiple providers including DuckDuckGo (default), Tavily, Exa, Google, and more.
    #[tool(
        name = "web_search",
        description = "Search the web and return results with URLs, titles, snippets, and optionally full page content. Supports multiple providers including DuckDuckGo (no API key), Tavily (AI-powered), Exa (semantic), WebSearchAPI.ai (LLM-ready content), Google, SerpAPI, and ArXiv (academic papers)."
    )]
    async fn web_search(
        &self,
        params: Parameters<SearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        let provider_name = params
            .provider
            .as_deref()
            .unwrap_or(&self.default_provider);

        let provider = self.get_provider(provider_name)?;

        let options = SearchOptions {
            query: params.query.clone(),
            max_results: Some(params.max_results.min(50)),
            provider,
            debug: Some(DebugOptions {
                enabled: false,
                log_requests: false,
                log_responses: false,
            }),
            ..Default::default()
        };

        let results = web_search(options).await.map_err(|e| {
            McpError::internal_error(format!("Search failed: {}", e), None)
        })?;

        let response = WebSearchResponse::new(params.query, results, provider_name.to_string());

        // Convert to JSON string for the response
        let json_str = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(format!("Failed to serialize response: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(json_str)]))
    }

    /// List available search providers
    #[tool(
        name = "list_providers",
        description = "List all available search providers and their status (whether required API keys are configured)"
    )]
    async fn list_providers(&self) -> Result<CallToolResult, McpError> {
        let providers = vec![
            ("duckduckgo", "DuckDuckGo", "No API key required", true),
            ("arxiv", "ArXiv", "No API key required (academic papers)", true),
            (
                "tavily",
                "Tavily",
                "Requires TAVILY_API_KEY",
                env::var("TAVILY_API_KEY").is_ok(),
            ),
            (
                "exa",
                "Exa",
                "Requires EXA_API_KEY",
                env::var("EXA_API_KEY").is_ok(),
            ),
            (
                "google",
                "Google",
                "Requires GOOGLE_API_KEY and GOOGLE_CX",
                env::var("GOOGLE_API_KEY").is_ok() && env::var("GOOGLE_CX").is_ok(),
            ),
            (
                "serpapi",
                "SerpAPI",
                "Requires SERPAPI_API_KEY",
                env::var("SERPAPI_API_KEY").is_ok(),
            ),
            (
                "websearchapi_ai",
                "WebSearchAPI.ai",
                "Requires WEBSEARCHAPI_KEY (LLM-ready content)",
                env::var("WEBSEARCHAPI_KEY").is_ok(),
            ),
        ];

        let mut output = String::from("Available Search Providers:\n\n");
        for (id, name, description, available) in providers {
            let status = if available { "✓" } else { "✗" };
            output.push_str(&format!(
                "{} {} ({})\n  {}\n\n",
                status, name, id, description
            ));
        }
        output.push_str(&format!("Default provider: {}", self.default_provider));

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for WebSearchMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "WebSearch MCP Server - Search the web using multiple providers. \
                 Use 'web_search' to search and 'list_providers' to see available providers."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = WebSearchMcpServer::new();
        assert_eq!(server.default_provider, "duckduckgo");
    }

    #[test]
    fn test_server_with_provider() {
        let server = WebSearchMcpServer::with_default_provider("arxiv");
        assert_eq!(server.default_provider, "arxiv");
    }

    #[test]
    fn test_get_provider_duckduckgo() {
        let server = WebSearchMcpServer::new();
        let provider = server.get_provider("duckduckgo");
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().name(), "duckduckgo");
    }

    #[test]
    fn test_get_provider_arxiv() {
        let server = WebSearchMcpServer::new();
        let provider = server.get_provider("arxiv");
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().name(), "arxiv");
    }

    #[test]
    fn test_get_provider_unknown() {
        let server = WebSearchMcpServer::new();
        let provider = server.get_provider("unknown");
        assert!(provider.is_err());
    }
}
