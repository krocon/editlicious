# Fortschritt 025 (über mehrere Sitzungen)

> Zwischenstand für die nächste Sitzung. Jede Etappe endet mit Commit + Push auf `main`.
> Design: [025-web-component.design.md](025-web-component.design.md). Diese Datei wird beim Abschluss gelöscht.

| Etappe | Inhalt | Stand |
|---|---|---|
| 1 | Design-Dokument, Prompt 024 → 025 umbenannt | ✅ 31.10.2026 |
| 2 | Rust: `readOnly` (E1), Zählen/Übersicht nach `ed-core` (E2), `ed-wasm` ausbauen (E2, E3) | ✅ 31.10.2026 |
| 3 | Frontend-Entkopplung (E7): `ViewSettings`, Dispatcher am Host, Tokens, ARIA-IDs, Clipboard-Brücke | ✅ 31.10.2026 |
| 4 | `WasmEditorApi`, Worker, Fassade, `element.ts`, `build:element`, Host-Theme (E4–E11) | ✅ 31.10.2026 |
| 5 | Demo `site/embed`, E2E (`scripts/embed-e2e.mjs`, alle Punkte grün), Doku, Lizenzen | ✅ 31.10.2026 – nur `done/` fehlt |

## Offen (Montag)

- [ ] `playScript(edscript, tempo?)` (optional im Prompt): Idee – `Player` aus `ed-wasm` im Worker, Frames per
      Zeitachse in die Ansicht schieben (wie `EDITOR_PLAYBACK.pushed`), am Ende `setText(finalText)`.
- [ ] Debug-Build der Desktop-App + kurzer Ende-zu-Ende-Lauf unter Xvfb (keine Regression; Unit-Tests und
      `pnpm build` sind grün).
- [ ] Manuell im echten Browser (Safari/Firefox): Zwischenablage (ClipboardItem mit Promise), IME im Shadow DOM.
- [ ] `uiLanguage` gilt seitenweit für alle Editoren (Texte hängen an einem globalen Sprach-Signal) – bewusst so,
      `<html lang>` bleibt unberührt; ggf. später je Instanz.
- [ ] Optional: npm-Paket aus `dist/element/editlicious-editor/` (package.json, Typen) – nicht verlangt.
- [ ] Abschluss: Prompt + Design nach `done/` (`git mv`), Status in `prompts/README.md`, diese Datei löschen.

## Hinweise für die nächste Sitzung

- Im Container fehlt das Ziel `wasm32-unknown-unknown` (Download gesperrt); das WASM-Paket `site/demo/pkg` baut der
  Workflow „Site demo“ nach jedem Push auf `src-tauri/core/**` bzw. `src-tauri/wasm/**` und checkt es ein →
  vor Etappe 4/5 `git pull`.
