//! MCP over stdio (design 016 E2): JSON-RPC 2.0, one message per line. Only what a tool server
//! needs: `initialize`, `ping`, `tools/list`, `tools/call`; notifications are ignored.

use crate::bridge::BridgeError;
use crate::tools;
use serde_json::{Value, json};
use std::io::{BufRead, Write};

/// Protocol versions this server speaks (newest first).
pub const VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// Whatever runs the tools – the bridge to the app, a fake in tests.
pub trait Tools {
    fn call(&mut self, tool: &str, args: Value) -> Result<Value, BridgeError>;
}

/// Answers one JSON-RPC message; `None` for notifications.
pub fn handle(msg: &Value, tools_impl: &mut dyn Tools) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let id = id?; // a notification (no id): nothing to answer
    let result = match method {
        "initialize" => {
            let asked = msg["params"]["protocolVersion"].as_str().unwrap_or("");
            let version = if VERSIONS.contains(&asked) {
                asked
            } else {
                VERSIONS[0]
            };
            Ok(json!({
                "protocolVersion": version,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "editlicious", "version": env!("CARGO_PKG_VERSION")},
                "instructions": "Drives the running editlicious text editor (enable it in Settings → AI control). \
            Lines and columns are 1-based. Use list_documents first; read_range and find never return a whole huge file."
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools::list()})),
        "tools/call" => {
            let name = msg["params"]["name"].as_str().unwrap_or("");
            if !tools::names().iter().any(|n| n == name) {
                Err((-32602, format!("unknown tool {name}")))
            } else {
                let args = msg["params"]
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                Ok(match tools_impl.call(name, args) {
                    Ok(v) => json!({
                        "content": [{"type": "text", "text": text_of(&v)}],
                        "structuredContent": structured(&v),
                        "isError": false
                    }),
                    Err(e) => json!({
                        "content": [{"type": "text", "text": format!("{}: {}", e.code, e.message)}],
                        "isError": true
                    }),
                })
            }
        }
        _ => Err((-32601, format!("method not found: {method}"))),
    };
    Some(match result {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
        Err((code, message)) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
        }
    })
}

fn text_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        v => serde_json::to_string_pretty(v).unwrap_or_default(),
    }
}

/// `structuredContent` must be an object.
fn structured(v: &Value) -> Value {
    if v.is_object() {
        v.clone()
    } else {
        json!({ "result": v })
    }
}

/// Reads messages from `input` until it ends, answers on `output`.
pub fn serve(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    tools_impl: &mut dyn Tools,
) -> std::io::Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            return Ok(());
        }
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        let answer = match serde_json::from_str::<Value>(text) {
            Ok(Value::Array(batch)) => {
                let out: Vec<Value> = batch.iter().filter_map(|m| handle(m, tools_impl)).collect();
                (!out.is_empty()).then_some(Value::Array(out))
            }
            Ok(msg) => handle(&msg, tools_impl),
            Err(e) => Some(json!({
                "jsonrpc": "2.0", "id": null,
                "error": {"code": -32700, "message": format!("parse error: {e}")}
            })),
        };
        if let Some(a) = answer {
            writeln!(output, "{a}")?;
            output.flush()?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(Vec<(String, Value)>);

    impl Tools for Fake {
        fn call(&mut self, tool: &str, args: Value) -> Result<Value, BridgeError> {
            self.0.push((tool.into(), args.clone()));
            match tool {
                "get_state" => Ok(json!({"revision": 4})),
                "list_documents" => Ok(json!([{"id": 1}])),
                _ => Err(BridgeError::new("revision_conflict", "changed")),
            }
        }
    }

    fn run(lines: &[Value]) -> (Vec<Value>, Fake) {
        let input: String = lines.iter().map(|l| format!("{l}\n")).collect();
        let mut out = Vec::new();
        let mut fake = Fake(Vec::new());
        serve(&mut input.as_bytes(), &mut out, &mut fake).unwrap();
        let answers = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        (answers, fake)
    }

    #[test]
    fn a_session_like_claude_code_runs_it() {
        let (a, fake) = run(&[
            json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_state","arguments":{"id":1}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"type_text","arguments":{"text":"x"}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"list_documents"}}),
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"rm_rf"}}),
            json!({"jsonrpc":"2.0","id":6,"method":"resources/list"}),
            json!({"jsonrpc":"2.0","id":7,"method":"ping"}),
        ]);
        assert_eq!(a.len(), 8, "the notification gets no answer");
        assert_eq!(a[0]["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(a[0]["result"]["serverInfo"]["name"], "editlicious");
        assert_eq!(a[1]["result"]["tools"].as_array().unwrap().len(), 18);
        assert_eq!(a[2]["result"]["isError"], false);
        assert_eq!(a[2]["result"]["structuredContent"]["revision"], 4);
        assert!(
            a[2]["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("4")
        );
        assert_eq!(a[3]["result"]["isError"], true);
        assert_eq!(
            a[3]["result"]["content"][0]["text"],
            "revision_conflict: changed"
        );
        assert_eq!(a[4]["result"]["structuredContent"]["result"][0]["id"], 1);
        assert_eq!(a[5]["error"]["code"], -32602);
        assert_eq!(a[6]["error"]["code"], -32601);
        assert_eq!(a[7]["result"], json!({}));
        assert_eq!(fake.0.len(), 3);
        assert_eq!(fake.0[1], ("type_text".into(), json!({"text": "x"})));
    }

    #[test]
    fn unknown_versions_get_the_newest_and_bad_json_an_error() {
        let (a, _) = run(&[
            json!({"jsonrpc":"2.0","id":"a","method":"initialize","params":{"protocolVersion":"1999-01-01"}}),
        ]);
        assert_eq!(a[0]["result"]["protocolVersion"], VERSIONS[0]);
        assert_eq!(a[0]["id"], "a");
        let mut out = Vec::new();
        serve(&mut "{nope\n".as_bytes(), &mut out, &mut Fake(vec![])).unwrap();
        assert!(String::from_utf8(out).unwrap().contains("-32700"));
    }
}
