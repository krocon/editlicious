# Fortschritt 025 (über mehrere Sitzungen)

> Zwischenstand für die nächste Sitzung. Jede Etappe endet mit Commit + Push auf `main`.
> Design: [025-web-component.design.md](025-web-component.design.md). Diese Datei wird beim Abschluss gelöscht.

| Etappe | Inhalt | Stand |
|---|---|---|
| 1 | Design-Dokument, Prompt 024 → 025 umbenannt | ✅ 31.10.2026 |
| 2 | Rust: `readOnly` (E1), Zählen/Übersicht nach `ed-core` (E2), `ed-wasm` ausbauen (E2, E3) | ✅ 31.10.2026 |
| 3 | Frontend-Entkopplung (E7): `ViewSettings`, Dispatcher am Host, Tokens, ARIA-IDs, Clipboard-Brücke | ✅ 31.10.2026 |
| 4 | `WasmEditorApi`, Worker, Fassade, `element.ts`, `build:element`, Host-Theme (E4–E11) | offen |
| 5 | Demo `site/embed`, E2E, Doku, Lizenzen, `done/` (E12) | offen |

## Hinweise für die nächste Sitzung

- Im Container fehlt das Ziel `wasm32-unknown-unknown` (Download gesperrt); das WASM-Paket `site/demo/pkg` baut der
  Workflow „Site demo“ nach jedem Push auf `src-tauri/core/**` bzw. `src-tauri/wasm/**` und checkt es ein →
  vor Etappe 4/5 `git pull`.
