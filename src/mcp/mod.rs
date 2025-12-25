//! MCP (Model Context Protocol) server implementation
//!
//! This module provides an MCP server that exposes web search capabilities
//! as tools that can be used by AI assistants.
//!
//! # Feature Flag
//!
//! This module is only available when the `mcp` feature is enabled:
//!
//! ```toml
//! [dependencies]
//! websearch = { version = "0.1", features = ["mcp"] }
//! ```

#[cfg(feature = "mcp")]
mod server;

#[cfg(feature = "mcp")]
mod schemas;

#[cfg(feature = "mcp")]
pub use server::WebSearchMcpServer;

#[cfg(feature = "mcp")]
pub use schemas::WebSearchResponse;

#[cfg(feature = "mcp")]
pub use server::SearchRequest;
