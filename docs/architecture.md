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
| A4 | Dev-Server-Port 1440 | comic-commander nutzt 1430, files-and-folders 1420 – alle drei parallel startbar |

## Commands

| Command | Zweck | Prompt |
|---|---|---|
| `get_app_info` | Name, Version, OS, Architektur | 001 |
| `open_document(path, loadId, onEvent)` / `cancel_load(loadId)` | Datei laden in eine neue Editor-Sitzung (gestreamt, Fortschritt, Abbruch) | 002/003 |
| `new_document` / `close_document(id)` / `document_info(id)` | Sitzungen verwalten | 002/003 |
| `editor_command(id, cmd) -> Frame` | **jede** Editor-Aktion (Bewegen, Tippen, Maus, Scrollen, Undo, Zwischenablage); Antwort = zu zeichnender Frame | 003 |
| `save_document(id, req)` | atomar speichern, Speichern unter, mit Encoding | 002 |
| `startup_paths` | Dateien der Kommandozeile | 003 |

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
| `arboard` (nur Text, Wayland) | System-Zwischenablage aus Rust – große Auswahlen gehen nicht über die IPC | 003 |
| `@angular/*`, `@tauri-apps/api`, `rxjs`, `tslib` | Frontend | 001 |

## Messwerte

Release-Build im Linux-Container (26.10.2026), Details in `prompts/done/002-dokumentmodell.design.md`:
100-MB-Datei mit 1,3 Mio. Zeilen öffnen 191 ms · Ansicht am Ende 0,07 ms · Fenster in 10-Mio.-Zeichen-Zeile
0,007 ms · 100 Tastenanschläge am Ende dieser Zeile 0,37 ms.
