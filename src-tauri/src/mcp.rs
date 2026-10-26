//! AI control over MCP, the app's side (design 016): the bridge server on 127.0.0.1 that the
//! `editlicious-mcp` sidecar talks to, and the tools it runs on the editor sessions. Pure apart
//! from the [`Ui`] it tells about changes – tests use a fake.

use crate::document::model::{DocInfo, LineEnding, Pos, SaveRequest, Sel};
use crate::document::registry::{Handle, Registry, lock_doc};
use crate::document::{Document, load};
use crate::editor::model::Command;
use crate::editor::playback::{self, Item, Tempo};
use crate::editor::{MemoryClipboard, Session, script};
use crate::error::{FsError, FsErrorCode};
use crate::playback::{Control, PlaybackEvent};
use crate::search::{self, Query};
use ed_mcp::bridge::{
    self, BridgeError, Endpoint, Hello, HelloAnswer, MAX_LINE_BYTES, Request, Response,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Default and largest number of characters `read_range` returns.
pub const READ_DEFAULT: usize = 20_000;
pub const READ_MAX: usize = 200_000;
/// Default and largest number of `find` hits.
pub const FIND_DEFAULT: usize = 50;
pub const FIND_MAX: usize = 1000;
/// Characters of a `find` preview.
const PREVIEW: usize = 160;

/// What the tools tell the UI (Tauri events in the app, a recorder in tests).
pub trait Ui: Send + Sync {
    /// A document was opened or created: add a tab.
    fn opened(&self, info: &DocInfo, activate: bool);
    fn activate(&self, id: u64);
    /// The text or cursors changed: draw again.
    fn changed(&self, id: u64);
    /// Run an action of the app on a document.
    fn action(&self, id: u64, action: &str);
    fn recording(&self, id: u64, on: bool);
    /// Events of a playback the AI started (frames, progress, end).
    fn playback(&self, id: u64, event: PlaybackEvent);
    /// Number of connected AI clients.
    fn status(&self, connections: usize);
}

/// What the UI tells the tools.
#[derive(Debug, Default)]
pub struct UiState {
    pub active: Option<u64>,
    /// `(id, label)` of the app's actions.
    pub actions: Vec<(String, String)>,
}

pub type Playing = Arc<Mutex<HashMap<u64, Arc<Control>>>>;

/// The tools, run on the sessions of the app.
pub struct Tools {
    pub sessions: Arc<Registry<Session>>,
    pub playing: Playing,
    pub ui_state: Arc<Mutex<UiState>>,
    pub ui: Arc<dyn Ui>,
    pub version: String,
}

fn bad(msg: impl Into<String>) -> FsError {
    FsError::new(FsErrorCode::InvalidRequest, msg, None)
}

fn opt_u64(args: &Value, key: &str) -> Result<Option<u64>, FsError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .map(Some)
            .ok_or_else(|| bad(format!("{key} must be a positive integer"))),
    }
}

fn opt_str<'a>(args: &'a Value, key: &str) -> Result<Option<&'a str>, FsError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        _ => Err(bad(format!("{key} must be a text"))),
    }
}

fn req_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, FsError> {
    opt_str(args, key)?.ok_or_else(|| bad(format!("{key} is missing")))
}

fn opt_bool(args: &Value, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// A 1-based `{line, column}` → 0-based position.
fn pos_arg(v: &Value, what: &str) -> Result<Pos, FsError> {
    let line = v
        .get("line")
        .and_then(Value::as_u64)
        .filter(|&l| l >= 1)
        .ok_or_else(|| bad(format!("{what}.line must be ≥ 1")))?;
    let col = v.get("column").and_then(Value::as_u64).unwrap_or(1).max(1);
    Ok(Pos::new(line as usize - 1, col as usize - 1))
}

fn pos_json(p: Pos) -> Value {
    json!({"line": p.line + 1, "column": p.ch + 1})
}

fn absolute(path: &str) -> Result<PathBuf, FsError> {
    let p = PathBuf::from(path);
    if path.is_empty() || path.contains('\0') || !p.is_absolute() {
        return Err(FsError::new(
            FsErrorCode::InvalidPath,
            "the path must be absolute",
            None,
        ));
    }
    Ok(p)
}

/// The tempo of `type_text`/`play_script`.
fn tempo_arg(args: &Value) -> Result<Tempo, FsError> {
    let num = |k: &str| args.get(k).and_then(Value::as_f64);
    Ok(match opt_str(args, "tempo")?.unwrap_or("instant") {
        "instant" => Tempo::Instant,
        "original" => Tempo::Original,
        "even" => Tempo::Even {
            ms: num("ms").unwrap_or(60.0).clamp(0.0, 10_000.0) as u64,
        },
        "human" => Tempo::Human {
            cpm: num("cpm")
                .unwrap_or(playback::DEFAULT_CPM)
                .clamp(30.0, 3000.0),
            seed: args.get("seed").and_then(Value::as_u64).unwrap_or(1),
            typos: num("typos").unwrap_or(0.0).clamp(0.0, 0.3),
        },
        t => return Err(bad(format!("unknown tempo {t}"))),
    })
}

impl Tools {
    fn doc_id(&self, args: &Value) -> Result<u64, FsError> {
        match opt_u64(args, "id")? {
            Some(id) => Ok(id),
            None => self
                .ui_state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .active
                .or_else(|| self.sessions.ids().last().copied())
                .ok_or_else(|| {
                    FsError::new(FsErrorCode::UnknownDocument, "no document is open", None)
                }),
        }
    }

    fn handle(&self, args: &Value) -> Result<(u64, Handle<Session>), FsError> {
        let id = self.doc_id(args)?;
        Ok((id, self.sessions.get(id)?))
    }

    fn is_playing(&self, id: u64) -> bool {
        self.playing
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&id)
    }

    /// Checks before an edit: no playback, the expected revision.
    fn may_write(&self, id: u64, s: &Session, args: &Value) -> Result<(), FsError> {
        if self.is_playing(id) {
            return Err(FsError::new(
                FsErrorCode::PlaybackRunning,
                "a playback runs in this document",
                None,
            ));
        }
        if let Some(rev) = opt_u64(args, "expected_revision")?
            && rev != s.doc.revision()
        {
            return Err(FsError::new(
                FsErrorCode::RevisionConflict,
                format!(
                    "the document changed: revision {} (expected {rev}); state: {}",
                    s.doc.revision(),
                    state_of(id, s)
                ),
                None,
            ));
        }
        Ok(())
    }

    /// Runs commands as one undo step and tells the UI.
    fn edit(&self, args: &Value, cmds: Vec<Command>) -> Result<Value, FsError> {
        let (id, h) = self.handle(args)?;
        let mut s = lock_doc(&h);
        self.may_write(id, &s, args)?;
        let mut clip = MemoryClipboard::default();
        s.doc.begin_group();
        let mut result = Ok(());
        for c in cmds {
            if let Err(e) = s.run(c, &mut clip) {
                result = Err(e);
                break;
            }
        }
        s.doc.end_group();
        drop(s);
        self.ui.changed(id);
        result?;
        Ok(state_of(id, &lock_doc(&h)))
    }

    /// Plays timeline steps: at once under the lock, else in real time like a playback (the
    /// UI shows it, Escape stops it); waits until done.
    fn play(
        &self,
        args: &Value,
        steps: Vec<playback::Step>,
        tempo: Tempo,
    ) -> Result<Value, FsError> {
        let (id, h) = self.handle(args)?;
        {
            let s = lock_doc(&h);
            self.may_write(id, &s, args)?;
        }
        if tempo == Tempo::Instant {
            playback::run_all(&mut lock_doc(&h), steps)?;
            self.ui.changed(id);
            return Ok(json!({"reason": "finished", "state": state_of(id, &lock_doc(&h))}));
        }
        let control = Arc::new(Control::default());
        {
            let mut p = self.playing.lock().unwrap_or_else(|e| e.into_inner());
            if p.contains_key(&id) {
                return Err(FsError::new(
                    FsErrorCode::PlaybackRunning,
                    "a playback runs in this document",
                    None,
                ));
            }
            p.insert(id, control.clone());
        }
        let reason = Mutex::new(String::new());
        crate::playback::run(h.clone(), steps, control, |e| {
            if let PlaybackEvent::Done { reason: r } = &e {
                *reason.lock().unwrap_or_else(|e| e.into_inner()) = r.clone();
            }
            self.ui.playback(id, e);
        });
        self.playing
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
        let reason = reason.into_inner().unwrap_or_else(|e| e.into_inner());
        Ok(json!({"reason": reason, "state": state_of(id, &lock_doc(&h))}))
    }

    /// Runs one tool.
    pub fn call(&self, tool: &str, args: &Value) -> Result<Value, FsError> {
        let args = if args.is_null() { &json!({}) } else { args };
        match tool {
            "list_documents" => {
                let active = self
                    .ui_state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .active;
                let docs: Vec<Value> = self
                    .sessions
                    .ids()
                    .into_iter()
                    .filter_map(|id| {
                        let h = self.sessions.get(id).ok()?;
                        let s = lock_doc(&h);
                        let i = s.doc.info(id);
                        Some(json!({
                            "id": id, "name": i.name, "path": i.path, "lines": i.line_count,
                            "unsaved": i.dirty, "revision": i.revision,
                            "active": active == Some(id)
                        }))
                    })
                    .collect();
                Ok(json!({ "documents": docs }))
            }
            "open_document" => {
                let path = absolute(req_str(args, "path")?)?;
                // open already? then show it
                for id in self.sessions.ids() {
                    if let Ok(h) = self.sessions.get(id) {
                        let s = lock_doc(&h);
                        if s.doc.path.as_deref() == Some(path.as_path()) {
                            let info = s.doc.info(id);
                            drop(s);
                            self.ui.activate(id);
                            return Ok(serde_json::to_value(info).unwrap_or_default());
                        }
                    }
                }
                let loaded = load::load_file(&path, &AtomicBool::new(false), &mut |_, _| {})?;
                let doc = Document::from_loaded(loaded, &path);
                let id = self.sessions.reserve_id();
                self.sessions.insert(id, Session::new(doc, id));
                let info = lock_doc(&self.sessions.get(id)?).doc.info(id);
                self.ui.opened(&info, true);
                Ok(serde_json::to_value(info).unwrap_or_default())
            }
            "new_document" => {
                let id = self.sessions.reserve_id();
                let le = if cfg!(windows) {
                    LineEnding::Crlf
                } else {
                    LineEnding::Lf
                };
                self.sessions
                    .insert(id, Session::new(Document::new_empty(le), id));
                let info = lock_doc(&self.sessions.get(id)?).doc.info(id);
                self.ui.opened(&info, true);
                Ok(serde_json::to_value(info).unwrap_or_default())
            }
            "activate_document" => {
                let (id, _) = self.handle(args)?;
                self.ui.activate(id);
                Ok(json!({"active": id}))
            }
            "save_document" => {
                let (id, h) = self.handle(args)?;
                let path = opt_str(args, "path")?
                    .map(absolute)
                    .transpose()?
                    .map(|p| p.to_string_lossy().into_owned());
                let mut s = lock_doc(&h);
                s.doc.save(&SaveRequest {
                    path,
                    ..Default::default()
                })?;
                let info = s.doc.info(id);
                drop(s);
                self.ui.changed(id);
                Ok(serde_json::to_value(info).unwrap_or_default())
            }
            "get_state" => {
                let (id, h) = self.handle(args)?;
                let s = lock_doc(&h);
                Ok(state_of(id, &s))
            }
            "read_range" => {
                let (_, h) = self.handle(args)?;
                let s = lock_doc(&h);
                Ok(read_range(&s.doc, args)?)
            }
            "find" => {
                let (_, h) = self.handle(args)?;
                let s = lock_doc(&h);
                let query = Query {
                    pattern: req_str(args, "pattern")?.to_owned(),
                    regex: opt_bool(args, "regex"),
                    case_sensitive: opt_bool(args, "case"),
                    whole_word: opt_bool(args, "word"),
                };
                let re = search::compile(&query)?;
                let max = opt_u64(args, "max_hits")?
                    .map_or(FIND_DEFAULT, |m| m as usize)
                    .clamp(1, FIND_MAX);
                let rope = s.doc.rope();
                let found = search::first_matches(rope, &re, max + 1);
                let more = found.len() > max;
                let hits: Vec<Value> = found
                    .into_iter()
                    .take(max)
                    .map(|(a, b)| {
                        let p = s.doc.char_to_pos(a);
                        let from = p.ch.saturating_sub(PREVIEW / 3);
                        json!({
                            "line": p.line + 1,
                            "column": p.ch + 1,
                            "length": b - a,
                            "preview": s.doc.line_slice(p.line, from, from + PREVIEW),
                        })
                    })
                    .collect();
                Ok(json!({"hits": hits, "more": more}))
            }
            "goto" => {
                let p = pos_arg(args, "goto")?;
                let (id, h) = self.handle(args)?;
                lock_doc(&h).run(
                    Command::GoTo {
                        line: p.line,
                        ch: p.ch,
                        select: false,
                    },
                    &mut MemoryClipboard::default(),
                )?;
                self.ui.changed(id);
                Ok(state_of(id, &lock_doc(&h)))
            }
            "select" => {
                let from = pos_arg(args.get("from").unwrap_or(&Value::Null), "from")?;
                let to = pos_arg(args.get("to").unwrap_or(&Value::Null), "to")?;
                let (id, h) = self.handle(args)?;
                lock_doc(&h).run(
                    Command::Select {
                        sels: vec![Sel {
                            anchor: from,
                            head: to,
                        }],
                    },
                    &mut MemoryClipboard::default(),
                )?;
                self.ui.changed(id);
                Ok(state_of(id, &lock_doc(&h)))
            }
            "type_text" => {
                let text = req_str(args, "text")?.replace("\r\n", "\n");
                let tempo = tempo_arg(args)?;
                let steps = playback::timeline(&[Item::Type(text)], tempo);
                self.play(args, steps, tempo)
            }
            "replace_selection" => {
                let text = req_str(args, "text")?.to_owned();
                self.edit(args, vec![Command::Insert { text }])
            }
            "run_command" => {
                let cmd: Command =
                    serde_json::from_value(args.get("command").cloned().unwrap_or(Value::Null))
                        .map_err(|e| bad(format!("not an editor command: {e}")))?;
                self.edit(args, vec![cmd])
            }
            "list_actions" => {
                let st = self.ui_state.lock().unwrap_or_else(|e| e.into_inner());
                let list: Vec<Value> = st
                    .actions
                    .iter()
                    .map(|(id, label)| json!({"id": id, "label": label}))
                    .collect();
                Ok(json!({ "actions": list }))
            }
            "run_action" => {
                let action = req_str(args, "action")?;
                let known = self
                    .ui_state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .actions
                    .iter()
                    .any(|(a, _)| a == action);
                if !known {
                    return Err(bad(format!("unknown action {action} – see list_actions")));
                }
                let (id, _) = self.handle(args)?;
                if self.is_playing(id) {
                    return Err(FsError::new(
                        FsErrorCode::PlaybackRunning,
                        "a playback runs in this document",
                        None,
                    ));
                }
                self.ui.action(id, action);
                Ok(json!({"requested": action, "id": id}))
            }
            "play_script" => {
                let text = req_str(args, "script")?;
                let items = script::parse(text, &mut |f| {
                    Err(bad(format!("type @{f}: files are not read over MCP")))
                })?;
                let tempo = match opt_str(args, "tempo")? {
                    Some(_) => tempo_arg(args)?,
                    None => Tempo::Original,
                };
                let steps = playback::timeline(&items, tempo);
                let instant = steps.iter().all(|s| s.at == 0);
                self.play(args, steps, if instant { Tempo::Instant } else { tempo })
            }
            "start_recording" => {
                let (id, h) = self.handle(args)?;
                lock_doc(&h).start_recording(&self.version);
                self.ui.recording(id, true);
                Ok(json!({"recording": id}))
            }
            "stop_recording" => {
                let (id, h) = self.handle(args)?;
                let path = opt_str(args, "path")?.map(absolute).transpose()?;
                let mut s = lock_doc(&h);
                let info = s
                    .stop_recording()
                    .map(crate::playback::RecordingInfo::of)
                    .ok_or_else(|| {
                        FsError::new(FsErrorCode::NoRecording, "no recording ran", None)
                    })?;
                if let Some(p) = &path {
                    let script = p.extension().is_some_and(|e| e == "edscript");
                    crate::playback::save(&s, p, script)?;
                }
                drop(s);
                self.ui.recording(id, false);
                Ok(json!({"steps": info.steps, "durationMs": info.duration_ms, "saved": path}))
            }
            t => Err(bad(format!("unknown tool {t}"))),
        }
    }
}

/// What `get_state` and edits answer.
pub fn state_of(id: u64, s: &Session) -> Value {
    let i = s.doc.info(id);
    let sels: Vec<Value> = s
        .sels_primary_last()
        .into_iter()
        .map(|x| json!({"anchor": pos_json(x.anchor), "head": pos_json(x.head)}))
        .collect();
    json!({
        "id": id,
        "name": i.name,
        "path": i.path,
        "revision": i.revision,
        "lines": i.line_count,
        "chars": i.char_count,
        "encoding": i.encoding,
        "lineEnding": i.line_ending,
        "unsaved": i.dirty,
        "recording": s.is_recording(),
        "selections": sels,
    })
}

fn read_range(doc: &Document, args: &Value) -> Result<Value, FsError> {
    let count = doc.line_count();
    let from = opt_u64(args, "from_line")?
        .ok_or_else(|| bad("from_line is missing"))?
        .max(1) as usize;
    if from > count {
        return Err(bad(format!(
            "from_line {from} is after the last line {count}"
        )));
    }
    let to = opt_u64(args, "to_line")?
        .map_or(from + 99, |t| t as usize)
        .clamp(from, count);
    let max = opt_u64(args, "max_chars")?
        .map_or(READ_DEFAULT, |m| m as usize)
        .clamp(1, READ_MAX);
    let mut text = String::new();
    let mut used = 0usize;
    let mut last = from;
    let mut truncated = false;
    for line in from - 1..to {
        let len = doc.line_len(line);
        let room = max - used;
        let piece = doc.line_slice(line, 0, len.min(room));
        used += piece.chars().count();
        text.push_str(&piece);
        last = line + 1;
        if len > room {
            truncated = true;
            break;
        }
        if line + 1 < to {
            text.push('\n');
            used += 1;
            if used >= max {
                truncated = true;
                break;
            }
        }
    }
    Ok(json!({
        "from_line": from,
        "to_line": last,
        "lines": count,
        "truncated": truncated,
        "text": text,
    }))
}

// ---------------------------------------------------------------- bridge server

/// The running bridge: listens on 127.0.0.1, writes the endpoint file, stops on drop.
pub struct Server {
    pub port: u16,
    endpoint_file: PathBuf,
    stop: Arc<AtomicBool>,
    connections: Arc<AtomicUsize>,
    streams: Arc<Mutex<Vec<TcpStream>>>,
}

impl Server {
    pub fn start(tools: Arc<Tools>, endpoint_file: &Path) -> Result<Self, FsError> {
        let listener =
            TcpListener::bind(("127.0.0.1", 0)).map_err(|e| FsError::from_io(&e, endpoint_file))?;
        let port = listener
            .local_addr()
            .map_err(|e| FsError::from_io(&e, endpoint_file))?
            .port();
        let token = bridge::new_token();
        bridge::write_endpoint(
            endpoint_file,
            &Endpoint {
                port,
                token: token.clone(),
                pid: std::process::id(),
            },
        )
        .map_err(|e| FsError::from_io(&e, endpoint_file))?;
        let stop = Arc::new(AtomicBool::new(false));
        let connections = Arc::new(AtomicUsize::new(0));
        let streams = Arc::new(Mutex::new(Vec::new()));
        {
            let (stop, connections, streams) = (stop.clone(), connections.clone(), streams.clone());
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    if let Ok(c) = stream.try_clone() {
                        streams.lock().unwrap_or_else(|e| e.into_inner()).push(c);
                    }
                    let (tools, token, connections) =
                        (tools.clone(), token.clone(), connections.clone());
                    std::thread::spawn(move || serve(stream, &tools, &token, &connections));
                }
            });
        }
        Ok(Self {
            port,
            endpoint_file: endpoint_file.to_path_buf(),
            stop,
            connections,
            streams,
        })
    }

    pub fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    /// Closes every connection (the listener stays).
    pub fn disconnect(&self) {
        for s in self
            .streams
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain(..)
        {
            let _ = s.shutdown(Shutdown::Both);
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.disconnect();
        // wake the accept loop so it sees `stop`
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        let _ = std::fs::remove_file(&self.endpoint_file);
    }
}

fn read_line(r: &mut BufReader<TcpStream>, line: &mut String) -> std::io::Result<usize> {
    line.clear();
    r.by_ref().take(MAX_LINE_BYTES as u64).read_line(line)
}

/// One connection: token first, then requests until it closes. Never logs contents.
fn serve(stream: TcpStream, tools: &Tools, token: &str, connections: &AtomicUsize) {
    let Ok(mut w) = stream.try_clone() else {
        return;
    };
    let mut r = BufReader::new(stream);
    let mut line = String::new();
    if read_line(&mut r, &mut line).unwrap_or(0) == 0 {
        return;
    }
    let ok =
        serde_json::from_str::<Hello>(&line).is_ok_and(|h| bridge::same_token(&h.token, token));
    let answer = serde_json::to_string(&HelloAnswer { ok }).unwrap_or_default();
    if writeln!(w, "{answer}").is_err() || !ok {
        return;
    }
    let n = connections.fetch_add(1, Ordering::SeqCst) + 1;
    tools.ui.status(n);
    while read_line(&mut r, &mut line).unwrap_or(0) > 0 {
        let resp = match serde_json::from_str::<Request>(&line) {
            Ok(req) => {
                let id = req.id;
                match tools.call(&req.tool, &req.args) {
                    Ok(v) => Response {
                        id,
                        result: Some(v),
                        error: None,
                    },
                    Err(e) => Response {
                        id,
                        result: None,
                        error: Some(BridgeError::new(&code_of(&e), e.message)),
                    },
                }
            }
            Err(e) => Response {
                id: 0,
                result: None,
                error: Some(BridgeError::new("invalid_request", e.to_string())),
            },
        };
        let text = serde_json::to_string(&resp).unwrap_or_default();
        if writeln!(w, "{text}").is_err() {
            break;
        }
    }
    let n = connections.fetch_sub(1, Ordering::SeqCst) - 1;
    tools.ui.status(n);
}

fn code_of(e: &FsError) -> String {
    serde_json::to_value(e.code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "io".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed_mcp::client::BridgeClient;
    use ed_mcp::server::Tools as _;

    #[derive(Default)]
    struct FakeUi(Mutex<Vec<String>>);

    impl Ui for FakeUi {
        fn opened(&self, info: &DocInfo, _: bool) {
            self.push(format!("opened:{}", info.name));
        }
        fn activate(&self, id: u64) {
            self.push(format!("activate:{id}"));
        }
        fn changed(&self, id: u64) {
            self.push(format!("changed:{id}"));
        }
        fn action(&self, id: u64, action: &str) {
            self.push(format!("action:{id}:{action}"));
        }
        fn recording(&self, id: u64, on: bool) {
            self.push(format!("recording:{id}:{on}"));
        }
        fn playback(&self, id: u64, e: PlaybackEvent) {
            if let PlaybackEvent::Done { reason } = e {
                self.push(format!("playback:{id}:{reason}"));
            }
        }
        fn status(&self, n: usize) {
            self.push(format!("status:{n}"));
        }
    }

    impl FakeUi {
        fn push(&self, s: String) {
            self.0.lock().unwrap().push(s);
        }
        fn log(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
    }

    fn tools(text: &str) -> (Tools, Arc<FakeUi>) {
        let sessions = Arc::new(Registry::default());
        let id = sessions.reserve_id();
        sessions.insert(id, Session::new(Document::from_text(text), id));
        let ui = Arc::new(FakeUi::default());
        let t = Tools {
            sessions,
            playing: Arc::default(),
            ui_state: Arc::new(Mutex::new(UiState {
                active: Some(id),
                actions: vec![("edit.sortAsc".into(), "Sort lines".into())],
            })),
            ui: ui.clone(),
            version: "test".into(),
        };
        (t, ui)
    }

    fn text(t: &Tools) -> String {
        lock_doc(&t.sessions.get(1).unwrap()).doc.rope().to_string()
    }

    #[test]
    fn type_select_replace_and_undo_in_one_step_each() {
        let (t, ui) = tools("hello world");
        t.call("goto", &json!({"line": 1, "column": 7})).unwrap();
        t.call("type_text", &json!({"text": "big\nnew "})).unwrap();
        assert_eq!(text(&t), "hello big\nnew world");
        let st = t
            .call(
                "select",
                &json!({"from": {"line": 2, "column": 5}, "to": {"line": 2, "column": 10}}),
            )
            .unwrap();
        assert_eq!(
            st["selections"][0]["head"],
            json!({"line": 2, "column": 10})
        );
        let rev = st["revision"].as_u64().unwrap();
        t.call(
            "replace_selection",
            &json!({"text": "planet", "expected_revision": rev}),
        )
        .unwrap();
        assert_eq!(text(&t), "hello big\nnew planet");
        // a stale revision
        let e = t
            .call(
                "replace_selection",
                &json!({"text": "x", "expected_revision": rev}),
            )
            .unwrap_err();
        assert_eq!(e.code, FsErrorCode::RevisionConflict);
        t.call("run_command", &json!({"command": {"type": "undo"}}))
            .unwrap();
        assert_eq!(text(&t), "hello big\nnew world");
        t.call("run_command", &json!({"command": {"type": "undo"}}))
            .unwrap();
        assert_eq!(text(&t), "hello world");
        assert!(ui.log().iter().all(|l| l == "changed:1"));
    }

    #[test]
    fn typing_like_a_person_runs_as_a_playback_and_waits() {
        let (t, ui) = tools("");
        let r = t
            .call(
                "type_text",
                &json!({"text": "abc", "tempo": "even", "ms": 5}),
            )
            .unwrap();
        assert_eq!(r["reason"], "finished");
        assert_eq!(text(&t), "abc");
        assert!(ui.log().contains(&"playback:1:finished".to_string()));
        assert!(t.playing.lock().unwrap().is_empty());
        // a running playback blocks edits
        t.playing.lock().unwrap().insert(1, Arc::default());
        let e = t
            .call("replace_selection", &json!({"text": "x"}))
            .unwrap_err();
        assert_eq!(e.code, FsErrorCode::PlaybackRunning);
    }

    #[test]
    fn read_range_and_find_are_bounded() {
        let long = "x".repeat(50_000);
        let (t, _) = tools(&format!("one\ntwo\n{long}\nfour two"));
        let r = t
            .call("read_range", &json!({"from_line": 1, "to_line": 2}))
            .unwrap();
        assert_eq!(
            (r["text"].as_str().unwrap(), r["truncated"].as_bool()),
            ("one\ntwo", Some(false))
        );
        let r = t
            .call("read_range", &json!({"from_line": 2, "max_chars": 100}))
            .unwrap();
        assert_eq!(r["truncated"], true);
        assert_eq!(r["text"].as_str().unwrap().chars().count(), 100);
        assert_eq!(r["to_line"], 3);
        assert!(t.call("read_range", &json!({"from_line": 9})).is_err());
        let r = t
            .call("find", &json!({"pattern": "two", "max_hits": 1}))
            .unwrap();
        assert_eq!(
            r["hits"][0],
            json!({"line": 2, "column": 1, "length": 3, "preview": "two"})
        );
        assert_eq!(r["more"], true);
        let r = t
            .call("find", &json!({"pattern": "x+", "regex": true}))
            .unwrap();
        assert!(r["hits"][0]["preview"].as_str().unwrap().len() <= PREVIEW);
        assert_eq!(
            t.call("find", &json!({"pattern": "(", "regex": true}))
                .unwrap_err()
                .code,
            FsErrorCode::InvalidSearch
        );
    }

    #[test]
    fn documents_actions_scripts_and_recordings() {
        let (t, ui) = tools("b\na");
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("x.txt");
        std::fs::write(&f, "file text").unwrap();
        let info = t
            .call("open_document", &json!({"path": f.to_string_lossy()}))
            .unwrap();
        assert_eq!(info["name"], "x.txt");
        let again = t
            .call("open_document", &json!({"path": f.to_string_lossy()}))
            .unwrap();
        assert_eq!(again["id"], info["id"]);
        let docs = t.call("list_documents", &json!({})).unwrap();
        assert_eq!(docs["documents"].as_array().unwrap().len(), 2);
        assert!(
            t.call("open_document", &json!({"path": "relative.txt"}))
                .is_err()
        );
        t.call("run_action", &json!({"action": "edit.sortAsc"}))
            .unwrap();
        assert!(
            t.call("run_action", &json!({"action": "app.quit"}))
                .is_err()
        );
        t.call("start_recording", &json!({})).unwrap();
        t.call("play_script", &json!({"script": "move docEnd\ntype \"!\""}))
            .unwrap();
        assert_eq!(text(&t), "b\na!");
        assert!(
            t.call("play_script", &json!({"script": "type @/etc/passwd"}))
                .is_err()
        );
        let out = dir.path().join("r.edscript");
        let r = t
            .call("stop_recording", &json!({"path": out.to_string_lossy()}))
            .unwrap();
        assert!(r["steps"].as_u64().unwrap() >= 2);
        assert!(
            std::fs::read_to_string(&out)
                .unwrap()
                .contains("type \"!\"")
        );
        let id2 = info["id"].as_u64().unwrap();
        t.call("save_document", &json!({"id": id2})).unwrap();
        let log = ui.log();
        assert!(log.contains(&"opened:x.txt".to_string()));
        assert!(log.contains(&format!("activate:{id2}")));
        assert!(log.contains(&"action:1:edit.sortAsc".to_string()));
        assert!(log.contains(&"recording:1:true".to_string()));
        assert_eq!(
            t.call("nope", &json!({})).unwrap_err().code,
            FsErrorCode::InvalidRequest
        );
    }

    /// The real sidecar client against the real bridge server.
    #[test]
    fn the_sidecar_client_drives_the_app_over_the_bridge() {
        let (t, ui) = tools("x");
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mcp.json");
        let server = Server::start(Arc::new(t), &file).unwrap();
        let mut client = BridgeClient::new(Some(file.clone()));
        let st = client.call("get_state", json!({})).unwrap();
        assert_eq!(st["lines"], 1);
        client.call("type_text", json!({"text": "yz"})).unwrap();
        let e = client
            .call("find", json!({"pattern": "(", "regex": true}))
            .unwrap_err();
        assert_eq!(e.code, "invalid_search");
        assert_eq!(server.connections(), 1);
        // a wrong token is turned away
        let mut ep = bridge::read_endpoint(&file).unwrap();
        ep.token = bridge::new_token();
        let other = dir.path().join("other.json");
        bridge::write_endpoint(&other, &ep).unwrap();
        assert!(
            BridgeClient::new(Some(other))
                .call("get_state", json!({}))
                .is_err()
        );
        server.disconnect();
        drop(server);
        assert!(!file.exists());
        assert!(ui.log().contains(&"status:1".to_string()));
    }
}
