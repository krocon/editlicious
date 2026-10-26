# Prompt 000: Gemeinsame Arbeitsweise für alle Feature-Prompts

> Gilt für alle Prompts in `prompts/` (Index und Reihenfolge: `prompts/README.md`). Jeder Feature-Prompt verweist
> hierher und beschreibt nur, was für sein Feature dazukommt. **Zuerst diese Datei lesen, dann den Feature-Prompt.**
> Übernommen und angepasst aus comic-commander-tauri (`prompts/000-arbeitsweise.md`).

---

## 1. Vor dem Start

- `CLAUDE.md` lesen, dazu `docs/architecture.md` und die im Prompt genannten Abschnitte aus
  `docs/legacy-analysis.md` („legacy §N“). Der Quellcode der Vorlage liegt auf Marcs Mac unter
  `~/IdeaProjects/java-k-editor/Editor` (in Remote-Sitzungen per `device_bash` lesen).
- Code aus **comic-commander-tauri** oder **files-and-folders-tauri** (beide öffentlich auf GitHub, MIT, gleicher
  Autor) darf übernommen werden – mit Herkunftsvermerk im Modulkopf und den zugehörigen Tests (CLAUDE.md §8).
- Abhängigkeiten prüfen: jeder Prompt nennt, welche Prompts vorher fertig sein müssen (`prompts/README.md`).
- Toolchain: `node -v` ≥ 22.22.3, `pnpm -v`, `cargo -V`; `pnpm install`; einmal `pnpm check`.

## 2. Phasen

| Phase | Ergebnis |
|---|---|
| 1 | Design-Dokument `prompts/NNN-<slug>.design.md` (Entscheidungen E1…, Verträge, Tests) |
| 2 | Rust: Tests zuerst, dann Implementierung |
| 3 | Frontend: Tests zuerst, dann Implementierung |
| 4 | Ende-zu-Ende in der echten App (im Container: Debug-Build + Xvfb, CLAUDE.md §9) |
| 5 | Doku: `docs/architecture.md`, README, CLAUDE.md, Prompt-Status |
| 6 | `pnpm check` grün, Commit + Push auf `main`, Prompt nach `prompts/done/` |

**Abweichung von den Schwesterprojekten:** keine Freigabe nach Phase 1 und keine Feature-Branches – Marc hat
ausdrücklich erlaubt, selbständig auf `main` zu committen und zu pushen. Offene Entscheidungsfragen werden im
Design-Dokument mit Empfehlung entschieden und im Abschlussbericht genannt.

### Regeln je Phase

- **Rust:** Logik in reinen Modulen ohne Tauri (testbar), Command-Schicht dünn, Commands `async` (Test
  `command_rules`), Blockierendes in `spawn_blocking`. Lange Arbeit über `tauri::ipc::Channel` mit Abbruch.
  `cargo fmt`, `cargo clippy --all-targets -D warnings`, `cargo test`.
- **Frontend:** Backend nur über typisierte Fassaden (`app/core/...Api`), Fakes in `src/testing/`. Signals, `OnPush`,
  zoneless. Tastatur nur über das Aktionssystem (ab 005). Texte zentral in `*-texts.ts`. Styling nur über Tokens aus
  `src/styles.css`. Barrierefreiheit: sichtbarer Fokus, alles per Tastatur, Dialoge mit Fokusfalle.
- **Große Dateien sind der Normalfall:** jede neue Funktion wird mit einer Datei von ≥ 100 MB, ≥ 1 Mio. Zeilen und
  einer Zeile von ≥ 10 Mio. Zeichen bedacht. Nichts darf proportional zur Dateigröße über die IPC-Grenze gehen.
- Nie in echten Nutzerdaten schreiben; Testdateien per Generator erzeugen.

## 3. Definition of Done

- [ ] Design-Dokument geschrieben (Entscheidungen, Verträge, Tests)
- [ ] Rust- und Frontend-Tests grün, `pnpm check` grün, Debug-Build gebaut
- [ ] Akzeptanzkriterien Ende-zu-Ende geprüft (soweit im Container möglich; Rest im Bericht)
- [ ] Doku aktuell, Prompt-Status abgehakt, Prompt + Design nach `done/` verschoben (`git mv`)
- [ ] Commit(s) auf `main`, gepusht
