//! ed-mcp – the MCP side of editlicious (design 016): a stdio MCP server that forwards tool
//! calls over a local bridge to the running app.
//!
//! - [`bridge`]    messages between sidecar and app, the endpoint file, token check
//! - [`tools`]     the tools and their JSON schemas
//! - [`server`]    JSON-RPC 2.0 / MCP over lines (initialize, ping, tools/list, tools/call)
//! - [`client`]    the bridge client of the sidecar (TCP to 127.0.0.1)

pub mod bridge;
pub mod client;
pub mod server;
pub mod tools;
