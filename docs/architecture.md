# Architektur – editlicious

> Entscheidungen, Verträge und Abhängigkeiten. Details je Feature in den Design-Dokumenten unter `prompts/done/`.

## Überblick

```
┌──────────── WebView (Angular 22, zoneless) ────────────┐ Befehle ┌──────────── Rust (Tauri 2) ─────────────┐
│ Eingaben → Befehle (Keymap, Maus in Zellen, IME)       │ ──────► │ commands.rs (dünn, async)                │
│ zeichnet Frames: sichtbare Zeilen/Spaltenfenster,      │ ◄────── │ ed-core: Sitzung (Cursor, Auswahl,       │
│ Carets, Auswahl; core/*Api = einzige Brücke (call.ts)  │  Frames │ Scroll), document (Rope, Encoding, Undo) │
└────────────────────────────────────────────────────────┘         └──────────────────────────────────────────┘
```

## Entscheidungen

| Nr. | Entscheidung | Begründung |
|---|---|---|
| A1 | Tauri 2 + Angular 22 (zoneless, Signals, OnPush) + Rust wie comic-commander/files-and-folders | Auftrag (000-start), gemeinsame Bausteine |
| A2 | Rust ist die Editor-Engine (Crate `ed-core`), die UI rendert nur Frames | Marc 26.10.2026; große Dateien: IPC-Last unabhängig von der Dateigröße; eine Quelle der Wahrheit, alles in Rust testbar (002 E13) |
| A3 | Eigene Editor-Ansicht statt CodeMirror/Monaco | beide halten den ganzen Text im JS-Heap und kommen mit Zeilen von Millionen Zeichen nicht zurecht (Prompt 003) |
| A5 | Angular 22 für die UI, Engine nativ – WebAssembly nur für die Webseiten-Demo | WASM im WebView müsste Dateien über die Brücke kopieren, 4-GB-Limit, keine Threads; Rust-UI-Frameworks verlören die Wiederverwendung aus den Schwesterprojekten (26.10.2026) |
| A6 | Aufnahmen speichern Engine-Befehle (Klicks als Auswahl), Abspielen läuft in Rust auf einer deterministischen Zeitachse und schickt Frames über einen Channel | fensterunabhängig, wiederholbar (Seed), dieselbe Zeitachse für MCP (016) und Video (017); eine Abspielung = ein Undo-Schritt (015) |
| A7 | KI-Steuerung über ein eigenes MCP-Sidecar (`ed-mcp`, stdio) mit Brücke zur laufenden App über 127.0.0.1 + Zufalls-Token; eigene MCP-Schicht statt `rmcp` | keine Tokio-/Makro-Abhängigkeit, ~800 KB Programm, auf allen drei Systemen nur Standardbibliothek; die App bleibt die eine Wahrheit (016) |
| A8 | Video-Export kopflos in Rust (`ed-video`: Engine-Frames + eigener Rasterizer) statt Aufnahme der WebView | CLI/CI-fähig, deterministisch (Hash-Test), beliebige Größe ohne Fenster; die Farben kommen aus `src/styles.css` (zur Bauzeit gelesen), das Layout folgt der Editor-Ansicht; Kodieren über ein externes `ffmpeg` (Lizenz/Größe), sonst PNG-Folge (017) |
| A9 | Rechnen am Caret mit eigenem Parser in `ed-core` (`math.rs`, `editor/hints.rs`) statt `meval`/`evalexpr`/`fasteval` | die Crates kennen weder Dezimalkomma, `0x`/`0b`/`_`, implizites Mal, `×÷−`, `fac`/Γ und `convert` der Vorlage noch Längen-/Tiefengrenzen, und sie liefern keinen aufgeräumten Ausdruck für den Hinweis; ~600 Zeilen, WASM-fähig, Fuzz-Test ohne `proptest` (eigener Zufall aus `playback::Rng`) (020) |
| A4 | Dev-Server-Port 1440 | comic-commander nutzt 1430, files-and-folders 1420 – alle drei parallel startbar |

## Commands

| Command | Zweck | Prompt |
|---|---|---|
| `get_app_info` | Name, Version, OS, Architektur | 001 |
| `open_document(path, loadId, onEvent)` / `cancel_load(loadId)` | Datei laden in eine neue Editor-Sitzung (gestreamt, Fortschritt, Abbruch) | 002/003 |
| `new_document(lineEnding?)` / `close_document(id)` / `document_info(id)` | Sitzungen verwalten | 002/003 |
| `editor_command(id, cmd) -> Frame` | **jede** Editor-Aktion (Bewegen, Tippen, Maus, Scrollen, Undo, Zwischenablage); Antwort = zu zeichnender Frame | 003 |
| `save_document(id, req)` | atomar speichern, Speichern unter, mit Encoding | 002 |
| `startup_paths` | Dateien der Kommandozeile | 003 |
| `reload_document(id, encoding?)` / `disk_state(id)` | Änderungen von außen erkennen und neu laden | 004 |
| `pick_open_files(near)` / `pick_save_path(name, near)` | native Dateidialoge (nur Rust-API) | 004 |
| `load_session` / `save_session` | `session.json` | 004 |
| `set_title` / `take_over_close` / `quit_app`, Ereignis `close-requested` | Fenster, Beenden mit Frage | 004 |
| `count_matches(id, query)` | Treffer zählen auf einem Schnappschuss (abbrechbar) | 006 |
| `list_languages` | Sprachen der Hervorhebung | 008 |
| `overview(id, buckets)` | Treffer/Marker je Abschnitt für die Scrollleiste (Schnappschuss, abbrechbar) | 010 |
| `list_encodings` | wählbare Zeichenkodierungen | 010 |
| `toggle_fullscreen` | Vollbild | 005 |
| `set_app_menu(spec)`, Ereignis `menu-action` | natives Menü (macOS) | 005 |
| `load_keymap_overrides` | `keymap.json` | 005 |
| `load_settings` / `save_settings(settings)` | `settings.json` (nachsichtig laden, Werte begrenzen, atomar schreiben) | 009 |
| `start_recording(id)` / `stop_recording(id)` / `save_recording(id, path, script)` | Befehle aufzeichnen, als `*.edrec.jsonl` oder `*.edscript` speichern | 015 |
| `mcp_enable(enabled)` / `mcp_info` / `mcp_set_actions(actions)` / `mcp_set_active(id)` / `mcp_disconnect`, Ereignisse `mcp-opened`, `mcp-activate`, `mcp-changed`, `mcp-action`, `mcp-recording`, `mcp-playback`, `mcp-status` | KI-Steuerung: Brücke an/aus, Werkzeuge kennen Aktionen und aktives Dokument, die UI führt aus, was die KI verlangt | 016 |
| `export_video(id, source, options, onEvent)` / `cancel_export` | Aufnahme/Skript als Video (ffmpeg) oder PNG-Folge rendern, Fortschritt über Channel, abbrechbar; dasselbe ohne App: `editlicious render …` | 017 |
| `count_marker(id)` | wie oft das markierte Wort am Cursor vorkommt (Schnappschuss, abbrechbar) | 019 |
| `check_playback(id, source)` / `play(id, source, tempo, onEvent)` / `playback_control(id, action)` | Aufnahme oder Skript prüfen und abspielen (Frames/Fortschritt über Channel, anhalten, Schritt, Stopp); während der Wiedergabe lehnt `editor_command` Bearbeitungen ab (`playback_running`) | 015 |

## Abhängigkeiten

| Paket | Wofür | Prompt |
|---|---|---|
| `tauri`, `tauri-plugin-dialog` | App-Rahmen; native Datei-Dialoge nur über die Rust-API | 001 |
| `serde`, `serde_json` | DTOs, Konfiguration | 001 |
| `tempfile` (dev) | Dateisystem-Tests | 001 |
| `ropey` 1.6 (`cr_lines`, `simd`) | Rope für den Text, O(log n) für Zeilen/Zeichen | 002 |
| `encoding_rs` | Dekodieren/Kodieren aller Encodings des Encoding Standard | 002 |
| `chardetng` | Encoding-Erkennung für Nicht-UTF-8-Dateien (Nachfolger von universalchardet der alten Version) | 002 |
| `memchr` | schnelles Zählen der Zeilenenden, NUL-Erkennung | 002 |
| `unicode-width`, `unicode-segmentation` | Zellen je Zeichen (breit/kombinierend), Graphem-Grenzen für Cursor-Schritte | 003 |
| `regex` | Suchen/Ersetzen mit linearer Laufzeit | 006 |
| `syntect` (Oniguruma; `fancy` für WASM) | Syntax-Hervorhebung mit Sublime/TextMate-Grammatiken | 008 |
| `arboard` (nur Text, Wayland) | System-Zwischenablage aus Rust – große Auswahlen gehen nicht über die IPC | 003 |
| `fontdue` 0.9 | Glyphen in reinem Rust rastern – deterministisch, ohne System-Schriftbibliothek (Video-Export) | 017 |
| `png` 0.18 | PNG-Einzelbilder und Golden-Images des Video-Exports | 017 |
| JetBrains Mono 2.304 (OFL-1.1, Schrift im Programm) | feste Schrift des Video-Exports (Ersatzschriften nur ausdrücklich) | 017 |
| `ed-mcp` (eigener Crate, nur `serde`/`serde_json`) | MCP-Sidecar `editlicious-mcp` und Brücken-Nachrichten | 016 |
| `@angular/*`, `@tauri-apps/api`, `rxjs`, `tslib` | Frontend | 001 |

## Messwerte

Release-Build im Linux-Container (26.10.2026), Details in `prompts/done/002-dokumentmodell.design.md`:
100-MB-Datei mit 1,3 Mio. Zeilen öffnen 191 ms · Ansicht am Ende 0,07 ms · Fenster in 10-Mio.-Zeichen-Zeile
0,007 ms · 100 Tastenanschläge am Ende dieser Zeile 0,37 ms.
Suche (006): Treffer am Ende von 100 MB 30 ms · 1,4 Mio. Treffer zählen 0,44 s.
Hervorhebung (008): Taste mit Hervorhebung ~5 ms in 100 000 Zeilen Rust, weite Sprünge sofort (ungefähr) und
nach dem Hintergrund-Aufholen exakt.

### Große Dateien (013)

`cargo test -p ed-core --release -- --ignored --nocapture --test-threads 1 measure_013` (`src-tauri/core/src/measure.rs`,
Größe über `ED_MEASURE_MB`), Linux-Container mit 2 Kernen, 7 GB RAM, über die Befehle der Sitzung wie in der App:

| Schritt | 1 GB, 10 Mio. Zeilen |
|---|---|
| Öffnen (lesen, dekodieren, Rope) | 1,6 s · Speicher 0,95 GB |
| erster Frame / Sprung ans Ende / Gehe zu Zeile 5 Mio. | 0,3 ms / 5 ms / 6 ms |
| Tippen in der Mitte (mit Log-Hervorhebung / ohne) | 4,5 ms / 0,2 ms je Taste |
| Weitersuchen / 10 000 Treffer zählen | 0,8 ms / 0,3 s |
| Alle ersetzen (10 000) / rückgängig | 0,4 s / 17 ms |
| Speichern (atomar) | 1,5 s |
| Zeilenenden → CRLF / rückgängig | 3,5 s / 2,5 s (Spitze 2× Dateigröße) |
| 2 Mio. Zeilen (200 MB) sortieren / rückgängig | 1,3 s / 0,4 s |

| Schritt | eine 100-MB-Zeile | 1 Mio. Zeilen LF/CRLF/CR gemischt |
|---|---|---|
| Öffnen | 90 ms | 41 ms |
| Ende der Zeile / 100 Tasten dort | < 0,1 ms / 1,2 ms | – |
| Rückwärts suchen über 100 MB | 110 ms | – |
| alle Zeilenenden → LF | – | 105 ms |
| Speichern | 134 ms | 45 ms |

**Grenzen** (013): Dateien bis 4 GB (`MAX_FILE_BYTES`), Speicherbedarf ≈ Dateigröße (Rope) + Undo; keine
Zwangswarnung, die App bleibt bei 1 GB flüssig. Sortieren höchstens 512 MB an Zeilen (`MAX_SORT_BYTES`, braucht
etwa das Vierfache), „Als HTML kopieren“ 20 000 Zeilen, Kopieren/Ausschneiden 64 MB (`MAX_TEXT_BYTES`), Undo
10 000 Schritte bzw. 256 MB (ein einzelner größerer Schritt bleibt). Ansicht höchstens 2 000 Reihen × 4 000 Zellen,
ein Scrollschritt höchstens 100 000 Reihen. Keine Geräte/Pipes. Behobene Engpässe: Zeilenenden-Undo als 1 Byte je
Umbruch statt Millionen Änderungen; Edits ab 50 000 Änderungen bauen die Rope neu, darunter in place (Speicher
flach); Sortieren ohne String je Zeile, natürlicher Vergleich ohne Allokation; Suche mit `find_iter` statt
`find_at` je Zeichen (Muster mit leeren Treffern); Einfügen normalisiert byteweise.

## WebAssembly-Demo (011)

`src-tauri/wasm` (Crate `ed-wasm`, nicht im Workspace, eigene `Cargo.lock`) bindet `ed-core` mit Feature `fancy`
per `wasm-bindgen` an (`Editor.command(json) → frame json`). Abhängigkeiten: `wasm-bindgen` (Bindung),
`web-time` in `ed-core` (`Instant`, das es auf wasm32 sonst nicht gibt; nativ = std). Gebaut vom Workflow
„Site demo“, der `site/demo/pkg` zurück auf `main` committet (im Container fehlt das wasm32-Ziel).

