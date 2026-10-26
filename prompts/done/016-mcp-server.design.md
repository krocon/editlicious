# Design 016: MCP-Server – KI steuert den Editor

> Prompt: [016-mcp-server.md](016-mcp-server.md). Stand 26.10.2026.

## Entscheidungen

- **E1 Sidecar + Brücke:** eigenes Binary `editlicious-mcp` (Crate `ed-mcp` im Workspace, nur `serde`/`serde_json`)
  spricht MCP über **stdio** und leitet Tool-Aufrufe über eine lokale Brücke an die laufende App weiter. Die App
  bleibt die eine Quelle der Wahrheit (Sitzungen, Undo, Anzeige); das Sidecar hat keinen Dateizugriff außer der
  Endpunkt-Datei.
- **E2 Eigene MCP-Schicht statt `rmcp`:** gebraucht wird nur JSON-RPC 2.0 über Zeilen auf stdio mit `initialize`,
  `ping`, `tools/list`, `tools/call` (Protokollversionen 2024-11-05, 2025-03-26, 2025-06-18). `rmcp` brächte Tokio,
  Makros und ein großes Binary mit; ~300 Zeilen eigener Code sind vollständig getestet.
- **E3 Brücke über TCP auf 127.0.0.1 statt Unix-Socket/Named Pipe:** mit der Standardbibliothek auf allen drei
  Systemen gleich, keine Abhängigkeit. Schutz: nur Loopback, zufälliger Port, **256-Bit-Token** je App-Start
  (aus dem Betriebssystem-Zufall über `RandomState`), das als erste Zeile gesendet werden muss; Port und Token stehen
  in `<Konfigverzeichnis>/mcp.json` (Unix: Rechte 0600; Windows: Benutzerprofil). Ohne Einstellung „KI-Steuerung
  erlauben“ gibt es weder Port noch Datei.
- **E4 Ablauf:** Sidecar liest `mcp.json`, verbindet, schickt `{"token":…}`, danach je Zeile
  `{"id", "tool", "args"}` → `{"id", "result" | "error": {code, message}}`. Antworten gehen als MCP-Text (JSON)
  zurück, Fehler als `isError: true` mit `code: message`. Läuft die App nicht oder ist die Steuerung aus, meldet das
  Sidecar das klar („editlicious is not running or AI control is off …“) – `tools/list` funktioniert trotzdem.
- **E5 Tools** (Zeilen und Spalten **1-basiert** wie in Skripten; `id` fehlt = aktives Dokument):
  `list_documents`, `open_document(path)`, `new_document`, `activate_document(id)`, `save_document(id, path?)`,
  `get_state(id)`, `read_range(id, from_line, to_line?, max_chars?)` (höchstens 200 000 Zeichen, Standard 20 000),
  `find(id, pattern, regex?, case?, word?, max_hits?)` (höchstens 1000), `goto(id, line, column?)`,
  `select(id, from, to)`, `type_text(id, text, tempo?, cpm?, seed?, typos?, expected_revision?)`,
  `replace_selection(id, text, expected_revision?)`, `run_command(id, command)` (jeder Engine-Befehl als JSON),
  `list_actions`, `run_action(id, action)` (Aktionen der App, die Liste liefert die UI), `play_script(id, script,
  tempo?)` (ohne `type @datei`), `start_recording(id)`, `stop_recording(id, path?)`.
- **E6 Nebenläufigkeit:** schreibende Tools nehmen `expected_revision`; passt sie nicht → `revision_conflict` mit
  dem aktuellen Zustand in der Meldung. Jeder schreibende Aufruf ist **eine Undo-Gruppe**. Während einer Wiedergabe
  lehnt die App Schreibendes ab (`playback_running`).
- **E7 Tippen im Tempo** benutzt die Wiedergabe aus 015 (gleicher Thread-Ablauf, gleiche Sperre); die Frames gehen
  als App-Ereignis `mcp-playback` an die UI, die sie wie eine eigene Wiedergabe zeigt („KI tippt … 37 %“, Esc
  bricht ab). Der Aufruf wartet bis zum Ende und meldet `finished`/`stopped`.
- **E8 UI-Ereignisse:** `mcp-opened` (Tab hinzufügen), `mcp-activate`, `mcp-changed` (neu zeichnen), `mcp-action`
  (die UI führt die Aktion aus), `mcp-status` (Zahl der Verbindungen). Die UI meldet ihre Aktionen
  (`mcp_set_actions`) und das aktive Dokument (`mcp_set_active`).
- **E9 Einstellung** `mcp` (Standard aus) im Einstellungsdialog mit der Zeile für Claude Code
  (`claude mcp add editlicious -- "<Pfad>"`) zum Kopieren; Aktion `mcp.disconnect` trennt alle Verbindungen und
  stoppt KI-Wiedergaben. Statusleiste: „KI“ solange verbunden.
- **E10 Verteilung:** das Sidecar liegt neben dem App-Programm. Der Release-Workflow baut `ed-mcp` vorher und
  nimmt es per `tauri.sidecar.conf.json` (`bundle.externalBin`) in die Pakete; im Alltag (`tauri dev`, Tests) bleibt
  die Konfiguration ohne Sidecar, damit nichts fehlt.

## Tests

- `ed-mcp`: JSON-RPC (initialize/Versionen, tools/list-Schnappschuss, tools/call über eine Fake-Brücke, Fehler,
  Notifications), Endpunkt-Datei, Brücken-Client gegen einen Test-Server.
- App: Tool-Aufrufe gegen echte Sitzungen mit Fake-UI (Grenzen von `read_range`/`find`, Revision, Undo-Gruppe,
  Tippen im Tempo, run_action-Prüfung), Brücken-Server (Token falsch → getrennt, Datei-Rechte), Ende-zu-Ende:
  Sidecar-Prozess über stdio gegen den Brücken-Server.
- Angular: Ereignisse → Tabs/Aktionen/Status, Einstellung.
