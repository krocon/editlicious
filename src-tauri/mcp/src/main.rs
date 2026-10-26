//! `editlicious-mcp` – MCP server (stdio) for the running editlicious (design 016).
//!
//! Claude Code: `claude mcp add editlicious -- /path/to/editlicious-mcp`
//! The app must run with Settings → AI control (MCP) on. Logs nothing of the documents.

use ed_mcp::bridge;
use ed_mcp::client::BridgeClient;
use ed_mcp::server;
use std::io::{BufReader, stdin, stdout};

fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("editlicious-mcp {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let mut tools = BridgeClient::new(bridge::endpoint_path());
    let mut input = BufReader::new(stdin().lock());
    let mut output = stdout().lock();
    if let Err(e) = server::serve(&mut input, &mut output, &mut tools) {
        eprintln!("editlicious-mcp: {e}");
        std::process::exit(1);
    }
}
