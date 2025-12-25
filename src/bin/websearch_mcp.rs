//! WebSearch MCP Server Binary
//!
//! A Model Context Protocol (MCP) server that exposes web search capabilities
//! as tools that can be used by AI assistants like Claude Desktop.
//!
//! # Transport Modes
//!
//! - **stdio** (default): For Claude Desktop and local MCP clients
//! - **http**: For containerized deployments (Docker, Kubernetes)
//!
//! # Usage
//!
//! ```bash
//! # Build with MCP feature
//! cargo build --release --features mcp --bin websearch-mcp
//!
//! # Run in stdio mode (Claude Desktop)
//! ./target/release/websearch-mcp
//!
//! # Run in HTTP mode (Docker/K8s)
//! ./target/release/websearch-mcp --transport http
//! # or
//! WEBSEARCH_TRANSPORT=http ./target/release/websearch-mcp
//! ```
//!
//! # Environment Variables
//!
//! - `WEBSEARCH_TRANSPORT`: Transport mode: `stdio` or `http` (default: stdio)
//! - `WEBSEARCH_BIND_ADDR`: HTTP bind address (default: 0.0.0.0:3000)
//! - `WEBSEARCH_DEFAULT_PROVIDER`: Default search provider (default: duckduckgo)
//! - `TAVILY_API_KEY`: API key for Tavily provider
//! - `EXA_API_KEY`: API key for Exa provider
//! - `GOOGLE_API_KEY` + `GOOGLE_CX`: API keys for Google provider
//! - `SERPAPI_API_KEY`: API key for SerpAPI provider
//! - `WEBSEARCHAPI_KEY`: API key for WebSearchAPI.ai provider
//!
//! # Claude Desktop Configuration (stdio mode)
//!
//! Add to your Claude Desktop config:
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "websearch": {
//!       "command": "/path/to/websearch-mcp",
//!       "env": {
//!         "WEBSEARCH_DEFAULT_PROVIDER": "duckduckgo"
//!       }
//!     }
//!   }
//! }
//! ```
//!
//! # Docker Deployment (http mode)
//!
//! ```yaml
//! websearch-mcp:
//!   image: websearch-mcp:latest
//!   environment:
//!     WEBSEARCH_TRANSPORT: http
//!     WEBSEARCHAPI_KEY: ${WEBSEARCHAPI_KEY}
//!   ports:
//!     - "3000:3000"
//! ```

use clap::Parser;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use websearch::mcp::WebSearchMcpServer;

#[derive(Parser, Debug)]
#[command(name = "websearch-mcp")]
#[command(about = "WebSearch MCP Server - Search the web via MCP protocol")]
struct Args {
    /// Transport mode: stdio or http
    #[arg(long, env = "WEBSEARCH_TRANSPORT", default_value = "stdio")]
    transport: String,

    /// Bind address for HTTP mode
    #[arg(long, env = "WEBSEARCH_BIND_ADDR", default_value = "0.0.0.0:3000")]
    bind_addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    eprintln!("websearch-mcp: Starting MCP server...");
    eprintln!(
        "websearch-mcp: Default provider: {}",
        std::env::var("WEBSEARCH_DEFAULT_PROVIDER").unwrap_or_else(|_| "duckduckgo".to_string())
    );

    match args.transport.as_str() {
        "stdio" => run_stdio().await,
        "http" => run_http(&args.bind_addr).await,
        other => {
            eprintln!(
                "websearch-mcp: Unknown transport '{}', use 'stdio' or 'http'",
                other
            );
            std::process::exit(1);
        }
    }
}

async fn run_stdio() -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::service::ServiceExt;
    use tokio::io::{stdin, stdout};

    eprintln!("websearch-mcp: Using stdio transport");

    let server = WebSearchMcpServer::new();
    let transport = (stdin(), stdout());
    let service = server.serve(transport).await?;

    eprintln!("websearch-mcp: Server running, waiting for requests...");

    let quit_reason = service.waiting().await?;
    eprintln!("websearch-mcp: Server stopped: {:?}", quit_reason);

    Ok(())
}

async fn run_http(bind_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    use axum::Router;
    use rmcp::transport::{
        streamable_http_server::{session::local::LocalSessionManager, tower::StreamableHttpService},
        StreamableHttpServerConfig,
    };

    eprintln!("websearch-mcp: Using HTTP transport on {}", bind_addr);

    let mcp_service: StreamableHttpService<WebSearchMcpServer, LocalSessionManager> =
        StreamableHttpService::new(
            || Ok(WebSearchMcpServer::new()),
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig {
                stateful_mode: false,
                sse_keep_alive: None,
                cancellation_token: CancellationToken::new(),
            },
        );

    let app = Router::new()
        .nest_service("/mcp", mcp_service)
        .route("/health", axum::routing::get(|| async { "OK" }));

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    eprintln!("websearch-mcp: Listening on {}", bind_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
