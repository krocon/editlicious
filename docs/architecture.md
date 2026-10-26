# Architektur – editlicious

> Entscheidungen, Verträge und Abhängigkeiten. Details je Feature in den Design-Dokumenten unter `prompts/done/`.

## Überblick

```
┌──────────── WebView (Angular 22, zoneless) ────────────┐        ┌──────────── Rust (Tauri 2) ────────────┐
│ Editor-Ansicht: rendert nur sichtbare Zeilen und das   │ invoke │ commands.rs (dünn, async)               │
│ sichtbare Spaltenfenster; Caret/Auswahl als Signals    │ ─────► │ document/ (Rope, Encoding, Undo, Save)  │
│ core/*Api = einzige Brücke (call.ts)                   │ ◄───── │ Channels für Fortschritt (Laden, Suche) │
└────────────────────────────────────────────────────────┘        └─────────────────────────────────────────┘
```

## Entscheidungen

| Nr. | Entscheidung | Begründung |
|---|---|---|
| A1 | Tauri 2 + Angular 22 (zoneless, Signals, OnPush) + Rust wie comic-commander/files-and-folders | Auftrag (000-start), gemeinsame Bausteine |
| A2 | Text im Rust-Backend, Frontend fragt Ausschnitte ab | große Dateien: IPC-Last unabhängig von der Dateigröße (Prompt 002) |
| A3 | Eigene Editor-Ansicht statt CodeMirror/Monaco | beide halten den ganzen Text im JS-Heap und kommen mit Zeilen von Millionen Zeichen nicht zurecht (Prompt 003) |
| A4 | Dev-Server-Port 1440 | comic-commander nutzt 1430, files-and-folders 1420 – alle drei parallel startbar |

## Commands

| Command | Zweck | Prompt |
|---|---|---|
| `get_app_info` | Name, Version, OS, Architektur | 001 |

## Abhängigkeiten

| Paket | Wofür | Prompt |
|---|---|---|
| `tauri`, `tauri-plugin-dialog` | App-Rahmen; native Datei-Dialoge nur über die Rust-API | 001 |
| `serde`, `serde_json` | DTOs, Konfiguration | 001 |
| `tempfile` (dev) | Dateisystem-Tests | 001 |
| `@angular/*`, `@tauri-apps/api`, `rxjs`, `tslib` | Frontend | 001 |
