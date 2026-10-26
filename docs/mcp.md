# AI control (MCP)

editlicious can be driven by an AI assistant such as Claude Code through the
[Model Context Protocol](https://modelcontextprotocol.io) (prompt 016). The AI uses the same engine commands as the
keyboard – it opens files, reads parts of them, types (also like a person, for screen videos), runs actions and
records.

## Turn it on

1. Settings → **AI control** → *Let an AI drive the editor (MCP)*. It is off by default.
2. Copy the line shown there into a terminal, e.g.

   ```bash
   claude mcp add editlicious -- "/Applications/editlicious.app/Contents/MacOS/editlicious-mcp"
   ```

The status bar shows **AI** while an assistant is connected (click it to disconnect, or run *Disconnect the AI*).
While the AI types, the status bar shows **AI typing ▶ 37 %** and Escape stops it. Everything the AI changes in one
call is one undo step.

## Security

- `editlicious-mcp` is a small program next to the app. It talks MCP over stdio and forwards the calls to the
  running app over **127.0.0.1** with a random key that changes at every start; port and key are in
  `mcp.json` in the config folder, readable only by your user.
- With the setting off there is neither a port nor that file.
- No shell, no file access beyond what the editor itself does: `open_document` opens like the open dialog,
  `save_document` saves open documents (or to an explicit path), scripts cannot read files.
- Reading is bounded: `read_range` at most 200,000 characters, `find` at most 1,000 hits – a 1 GB file never goes
  over the bridge as a whole. Document contents are never logged.

## Tools

Lines and columns start at 1; without `id` a tool works on the active document.

| Tool | |
|---|---|
| `list_documents`, `open_document(path)`, `new_document`, `activate_document(id)`, `save_document(id, path?)` | documents and tabs |
| `get_state(id)` | revision, lines, encoding, line ending, unsaved, selections |
| `read_range(id, from_line, to_line?, max_chars?)`, `find(id, pattern, regex?, case?, word?, max_hits?)` | reading, bounded |
| `goto(id, line, column?)`, `select(id, from, to)` | caret and selection |
| `type_text(id, text, tempo?, cpm?, seed?, typos?, ms?, expected_revision?)` | typing – `tempo: human` types like a person and waits until done |
| `replace_selection(id, text, expected_revision?)`, `run_command(id, command)` | edits; `command` = any engine command, e.g. `{"type":"sortLines","options":{"unique":true}}` |
| `list_actions`, `run_action(id, action)` | the actions of the command palette |
| `play_script(id, script, tempo?)` | a typing script ([`scripts.md`](scripts.md)) |
| `start_recording(id)`, `stop_recording(id, path?)` | recordings |

`expected_revision` (from `get_state`) protects against typing over changes you made meanwhile: the call fails with
`revision_conflict` and the current state.
