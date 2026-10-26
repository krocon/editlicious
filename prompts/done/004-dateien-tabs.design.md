# Design 004: Dateien und Tabs

Status: umgesetzt · Datum: 26.10.2026

## Entscheidungen

- **E1 Aktionssystem schon hier** (aus comic-commander 008 übernommen: `action-catalog`, `ActionService`,
  `KeyboardDispatcher`, `shortcut`, `keymap.*.json`, `platform`): Datei-Kürzel brauchen es, und so landen auch die
  Editor-Tasten gleich als Aktionen mit Engine-Befehl (`ActionDef.cmd`) im Katalog – Kontexte `global` und
  `editor`. Das versteckte Eingabefeld (`data-editor-input`) gilt für den Dispatcher nicht als Textfeld: alle Tasten
  dort sind Aktionen, nur reines Tippen geht als `input` an die Engine. `key-commands.ts` aus 003 entfällt. Menü,
  Befehlspalette und `keymap.json` folgen in 005.
- **E2 Tabs** (legacy: auto/an/aus, Standard **auto** = erst ab der zweiten Datei): ruhig, oben, Punkt = ungespeichert
  (× beim Überfahren), Mittelklick schließt, Ziehen sortiert um, Tastatur nach APG (Pfeile, Pos1/Ende, Entf).
  Nächste/vorige Datei: Strg+Tab / Strg+Umschalt+Tab (alle OS), F10 / Umschalt+F10 (legacy).
- **E3 Fragen als Promise** (`AskService` + `AskDialog`, `alertdialog`): Ungespeichert (Speichern / Nicht speichern /
  Abbrechen), nicht darstellbares Zeichen (→ als UTF-8 speichern), von außen geändert (Neu laden / Meine behalten).
- **E4 Schließen des Fensters:** Rust verhindert das Schließen, sobald das Frontend `take_over_close` gemeldet hat,
  und sendet `close-requested`; das Frontend fragt und ruft `quit_app`. Ohne Frontend schließt das Fenster normal.
- **E5 Sitzung** `session.json` (App-Config): offene Dateien mit Caret und Scrollstand, aktive Datei, zuletzt
  benutzte (30), Tab-Modus; nachsichtig geladen, gebündelt geschrieben (1 s). Start: Kommandozeile → sonst Sitzung →
  sonst leeres Dokument. Ein unberührtes leeres Dokument macht beim Öffnen einer Datei Platz.
- **E6 Änderungen von außen:** beim Fokus des Fensters `disk_state` (Größe/mtime) je Datei; unverändert → still neu
  laden (Caret und Scroll bleiben, `Session::replace_document`), bearbeitet → fragen; gelöscht → Hinweis.
- **E7 Strg/⌘+S speichert die aktuelle Datei** (heutige Konvention, legacy §9); Alle speichern Mod+Alt+S; neue
  Dokumente gehen über „Speichern unter“. Neue Dokumente: LF (Windows: CRLF).
- **E8 Dialoge nur über Rust** (`pick_open_files`, `pick_save_path`, Start im Ordner der aktiven Datei über einen
  Dateipfad `near` – Pfade bleiben im Frontend opak). Fenstertitel über `set_title`
  („• name – editlicious“).
- **E9 Drag & Drop** von Dateien über `onDragDropEvent` der WebView.

## Verträge

Neue Commands: `reload_document`, `disk_state`, `pick_open_files(near)`, `pick_save_path(name, near)`,
`load_session`, `save_session`, `set_title`, `take_over_close`, `quit_app`; Ereignis `close-requested`.
Engine-Befehl `goTo { line, ch, select }`.

## Tests

- Rust: `session_file` (Rundreise, Bereinigen, kaputte Datei), `disk_state`.
- Frontend: `keymap.spec` (je OS, Overrides, jede Editor-Aktion hat einen Befehl), `keyboard-dispatcher.spec`
  (Editor-Eingabe, Tippen, andere Textfelder, IME), `documents.store.spec` (Start, Öffnen + Duplikate + leeres
  Dokument, Sitzung, Ungespeichert-Frage, Speichern unter, UTF-8-Angebot, Beenden, Änderungen von außen, Tabs),
  `editor-view.spec` über den Dispatcher.
- Ende-zu-Ende (Xvfb): zwei Dateien von der Kommandozeile (Umlaut im Namen) → Tabs, Strg+Tab, Tippen, Strg+S
  (Datei geschrieben), Strg+W mit Frage, Escape, Neustart ohne Argumente stellt Dateien und Position wieder her.
  Nicht im Container prüfbar: native Dateidialoge, Fenster-Fokus-Ereignisse, Drag & Drop aus dem Dateimanager.

## Bekannt

- macOS: das Standard-App-Menü von Tauri beendet mit ⌘Q ohne Frage – das eigene Menü (005) behebt das; bis dahin
  ist ⌘Q als Aktion `app.quit` belegt.
