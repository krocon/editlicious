//! The tools of the MCP server (design 016 E5): names, descriptions and input schemas. Lines and
//! columns are 1-based; a missing `id` means the active document.

use serde_json::{Value, json};

fn id() -> Value {
    json!({"type": "integer", "description": "Document id (from list_documents); default: the active document"})
}

fn revision() -> Value {
    json!({"type": "integer", "description": "Fail with revision_conflict if the document changed since this revision (get_state)"})
}

fn pos() -> Value {
    json!({
        "type": "object",
        "properties": {
            "line": {"type": "integer", "minimum": 1},
            "column": {"type": "integer", "minimum": 1}
        },
        "required": ["line"]
    })
}

fn tempo() -> Value {
    json!({
        "type": "string",
        "enum": ["instant", "human", "even", "original"],
        "description": "instant (default): all at once; human: like a person typing (for screen videos); even: the same pause before every character"
    })
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        }
    })
}

/// Every tool with its schema (`tools/list`).
pub fn list() -> Vec<Value> {
    vec![
        tool(
            "list_documents",
            "Lists the open documents of the running editlicious (id, name, path, lines, unsaved changes, which one is active).",
            json!({}),
            &[],
        ),
        tool(
            "open_document",
            "Opens a text file (absolute path) in a new tab, or activates it if it is open.",
            json!({"path": {"type": "string"}}),
            &["path"],
        ),
        tool(
            "new_document",
            "Opens a new, empty document.",
            json!({}),
            &[],
        ),
        tool(
            "activate_document",
            "Shows a document (switches the tab).",
            json!({"id": id()}),
            &["id"],
        ),
        tool(
            "save_document",
            "Saves a document to its file, or to `path` (absolute) – keeps encoding and line endings.",
            json!({"id": id(), "path": {"type": "string"}}),
            &[],
        ),
        tool(
            "get_state",
            "Revision, line count, encoding, line ending, language, unsaved changes and the selections of a document.",
            json!({"id": id()}),
            &[],
        ),
        tool(
            "read_range",
            "Reads lines from_line..to_line (inclusive, 1-based); stops at max_chars (default 20000, at most 200000). Never reads a whole huge file.",
            json!({
                "id": id(),
                "from_line": {"type": "integer", "minimum": 1},
                "to_line": {"type": "integer", "minimum": 1},
                "max_chars": {"type": "integer", "minimum": 1, "maximum": 200000}
            }),
            &["from_line"],
        ),
        tool(
            "find",
            "Finds matches (line, column, the line around it) – at most max_hits (default 50, at most 1000).",
            json!({
                "id": id(),
                "pattern": {"type": "string"},
                "regex": {"type": "boolean"},
                "case": {"type": "boolean", "description": "case-sensitive"},
                "word": {"type": "boolean", "description": "whole words only"},
                "max_hits": {"type": "integer", "minimum": 1, "maximum": 1000}
            }),
            &["pattern"],
        ),
        tool(
            "goto",
            "Puts the caret at line/column (1-based) and scrolls there.",
            json!({"id": id(), "line": {"type": "integer", "minimum": 1}, "column": {"type": "integer", "minimum": 1}}),
            &["line"],
        ),
        tool(
            "select",
            "Selects from one position to another (1-based).",
            json!({"id": id(), "from": pos(), "to": pos()}),
            &["from", "to"],
        ),
        tool(
            "type_text",
            "Types text at the caret (replacing the selection), visibly in the editor. With tempo human it types like a person – waits until done. One undo step.",
            json!({
                "id": id(),
                "text": {"type": "string"},
                "tempo": tempo(),
                "cpm": {"type": "number", "description": "characters per minute for human (default 330)"},
                "seed": {"type": "integer", "description": "same seed = same rhythm"},
                "typos": {"type": "number", "description": "chance of a corrected typo per letter, 0..0.3"},
                "ms": {"type": "integer", "description": "pause per character for even"},
                "expected_revision": revision()
            }),
            &["text"],
        ),
        tool(
            "replace_selection",
            "Replaces the selection (or inserts at the caret) with text at once. One undo step.",
            json!({"id": id(), "text": {"type": "string"}, "expected_revision": revision()}),
            &["text"],
        ),
        tool(
            "run_command",
            "Runs one engine command, e.g. {\"type\":\"duplicateLines\"}, {\"type\":\"sortLines\",\"options\":{\"unique\":true}}, {\"type\":\"move\",\"to\":\"wordRight\",\"select\":true}. One undo step.",
            json!({"id": id(), "command": {"type": "object"}, "expected_revision": revision()}),
            &["command"],
        ),
        tool(
            "list_actions",
            "Lists the actions of the app (id and label) that run_action can run – the same as the command palette.",
            json!({}),
            &[],
        ),
        tool(
            "run_action",
            "Runs an action of the app (as from the command palette) on a document.",
            json!({"id": id(), "action": {"type": "string"}}),
            &["action"],
        ),
        tool(
            "play_script",
            "Plays a typing script (editlicious .edscript language: type, pause, goto, select, move, find, do …) into a document; waits until done. One undo step.",
            json!({"id": id(), "script": {"type": "string"}, "tempo": tempo(), "expected_revision": revision()}),
            &["script"],
        ),
        tool(
            "start_recording",
            "Starts recording what happens in a document.",
            json!({"id": id()}),
            &[],
        ),
        tool(
            "stop_recording",
            "Stops the recording; with path (absolute) saves it (.edscript = as a script, else .edrec.jsonl).",
            json!({"id": id(), "path": {"type": "string"}}),
            &[],
        ),
    ]
}

pub fn names() -> Vec<String> {
    list()
        .iter()
        .filter_map(|t| t["name"].as_str().map(str::to_owned))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_a_unique_name_a_description_and_an_object_schema() {
        let names = names();
        assert_eq!(names.len(), 18);
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len());
        for t in list() {
            assert!(!t["description"].as_str().unwrap().is_empty());
            assert_eq!(t["inputSchema"]["type"], "object");
            for r in t["inputSchema"]["required"].as_array().unwrap() {
                assert!(
                    t["inputSchema"]["properties"]
                        .get(r.as_str().unwrap())
                        .is_some()
                );
            }
        }
    }
}
