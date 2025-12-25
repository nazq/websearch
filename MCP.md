# WebSearch MCP Server

Model Context Protocol (MCP) server that provides web search capabilities to AI assistants.

## Build

```bash
cargo build --release --features mcp --bin websearch-mcp
```

Binary location: `target/release/websearch-mcp`

## Transport Modes

The server supports two transport modes:

| Mode | Use Case | Command |
|------|----------|---------|
| `stdio` | Claude Desktop, local MCP clients | `./websearch-mcp` (default) |
| `http` | Docker, Kubernetes, remote clients | `./websearch-mcp --transport http` |

### Stdio Mode (Default)

For Claude Desktop and local MCP clients. Communicates via stdin/stdout.

```bash
./websearch-mcp
```

### HTTP Mode

For containerized deployments. Exposes HTTP endpoint with streamable HTTP transport.

```bash
./websearch-mcp --transport http --bind-addr 0.0.0.0:3000
# or via environment variables
WEBSEARCH_TRANSPORT=http WEBSEARCH_BIND_ADDR=0.0.0.0:3000 ./websearch-mcp
```

Endpoints:
- `POST /mcp` - MCP protocol endpoint (streamable HTTP)
- `GET /health` - Health check (returns "OK")

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `WEBSEARCH_TRANSPORT` | Transport mode: `stdio` or `http` (default: `stdio`) | No |
| `WEBSEARCH_BIND_ADDR` | HTTP bind address (default: `0.0.0.0:3000`) | No |
| `WEBSEARCH_DEFAULT_PROVIDER` | Default search provider (default: `duckduckgo`) | No |
| `WEBSEARCHAPI_KEY` | WebSearchAPI.ai API key | For websearchapi_ai |
| `TAVILY_API_KEY` | Tavily API key | For tavily |
| `EXA_API_KEY` | Exa API key | For exa |
| `GOOGLE_API_KEY` | Google Custom Search API key | For google |
| `GOOGLE_CX` | Google Custom Search Engine ID | For google |
| `SERPAPI_API_KEY` | SerpAPI key | For serpapi |

### Available Providers

| Provider | API Key Required | Features |
|----------|-----------------|----------|
| `duckduckgo` | No | Basic web search |
| `arxiv` | No | Academic papers |
| `websearchapi_ai` | Yes | LLM-ready markdown content |
| `tavily` | Yes | AI-powered search with answers |
| `exa` | Yes | Semantic search with content |
| `google` | Yes | Google Custom Search |
| `serpapi` | Yes | Google results via SerpAPI |

## Client Configuration

### Claude Desktop

Config file locations:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **Linux**: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "websearch": {
      "command": "/absolute/path/to/websearch-mcp",
      "env": {
        "WEBSEARCH_DEFAULT_PROVIDER": "websearchapi_ai",
        "WEBSEARCHAPI_KEY": "wsa_xxx"
      }
    }
  }
}
```

### Claude Code

Add to `.claude/settings.json` (project) or `~/.claude/settings.json` (global):

```json
{
  "mcpServers": {
    "websearch": {
      "command": "/absolute/path/to/websearch-mcp",
      "env": {
        "WEBSEARCHAPI_KEY": "wsa_xxx"
      }
    }
  }
}
```

## MCP Tools

### web_search

Search the web and return results with optional full page content.

**Parameters:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `query` | string | (required) | Search query |
| `max_results` | integer | 5 | Number of results (1-50) |
| `provider` | string | default provider | Which provider to use |
| `include_content` | boolean | true | Include full page content |

**Example:**

```json
{
  "name": "web_search",
  "arguments": {
    "query": "rust async programming",
    "max_results": 3,
    "provider": "websearchapi_ai"
  }
}
```

**Response:**

```json
{
  "query": "rust async programming",
  "results": [
    {
      "url": "https://example.com/article",
      "title": "Article Title",
      "snippet": "Brief description...",
      "domain": "example.com",
      "provider": "websearchapi_ai",
      "content": "Full markdown content...",
      "content_format": "markdown",
      "word_count": 1234
    }
  ],
  "provider": "websearchapi_ai",
  "result_count": 3
}
```

### list_providers

List available search providers and their configuration status.

**Parameters:** None

**Example Response:**

```
Available Search Providers:

✓ DuckDuckGo (duckduckgo)
  No API key required

✓ ArXiv (arxiv)
  No API key required (academic papers)

✓ WebSearchAPI.ai (websearchapi_ai)
  Requires WEBSEARCHAPI_KEY (LLM-ready content)

✗ Tavily (tavily)
  Requires TAVILY_API_KEY

Default provider: websearchapi_ai
```

## Testing

Test the server manually with JSON-RPC over stdio:

```bash
# Start server and send initialization sequence
cat << 'EOF' | WEBSEARCHAPI_KEY="your-key" ./target/release/websearch-mcp
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
EOF
```

## Docker Deployment

Build and run with Docker:

```bash
# Build image
docker build -f Dockerfile.mcp -t websearch-mcp .

# Run container
docker run -d --name websearch-mcp \
  -p 3000:3000 \
  -e WEBSEARCHAPI_KEY=wsa_xxx \
  -e WEBSEARCH_DEFAULT_PROVIDER=websearchapi_ai \
  websearch-mcp

# Test health
curl http://localhost:3000/health
```

### Docker Compose

```yaml
services:
  websearch-mcp:
    image: websearch-mcp:latest
    container_name: websearch-mcp
    ports:
      - "3000:3000"
    environment:
      WEBSEARCH_TRANSPORT: http
      WEBSEARCH_DEFAULT_PROVIDER: websearchapi_ai
      WEBSEARCHAPI_KEY: ${WEBSEARCHAPI_KEY}
    healthcheck:
      test: ["CMD", "curl", "-sf", "http://localhost:3000/health"]
      interval: 10s
      timeout: 5s
      retries: 5
```

### Connecting from MCP Clients

For HTTP-based MCP clients (like fp-agent-srv):

```json
{
  "name": "websearch",
  "url": "http://websearch-mcp:3000/mcp",
  "transport": "streamable-http"
}
```

## Architecture

```
┌─────────────────┐   stdio/http   ┌──────────────────┐
│   MCP Client    │◄──────────────►│  websearch-mcp   │
│ (Claude, etc.)  │   JSON-RPC     │                  │
└─────────────────┘                └────────┬─────────┘
                                            │
                                            ▼
                                   ┌──────────────────┐
                                   │ Search Providers │
                                   ├──────────────────┤
                                   │ • DuckDuckGo     │
                                   │ • WebSearchAPI   │
                                   │ • Tavily         │
                                   │ • Exa            │
                                   │ • Google         │
                                   │ • SerpAPI        │
                                   │ • ArXiv          │
                                   └──────────────────┘
```

The MCP server:
1. Receives JSON-RPC requests (stdio or HTTP)
2. Parses tool calls and extracts parameters
3. Routes to the appropriate search provider
4. Returns results as JSON-RPC responses

## Troubleshooting

### Server won't start
- Ensure the binary was built with `--features mcp`
- Check the binary path is absolute in config

### Provider not available
- Run `list_providers` tool to check status
- Verify environment variables are set in the config

### No results returned
- Check API key is valid
- Try a different provider (e.g., `duckduckgo` requires no key)

### Content not included
- Set `include_content: true` in request
- Use a provider that supports content extraction (`websearchapi_ai`, `exa`, `tavily`)
