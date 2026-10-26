# Design 008: Syntax-Hervorhebung und Farbschemata

Status: umgesetzt · Datum: 26.10.2026

## Entscheidungen

- **E1 `syntect` mit den Sublime/TextMate-Grammatiken** (legacy-Wunsch „TextMate-Grammatiken“): rund 45 Sprachen
  der Standardsammlung, dazu eigene Grammatiken **Log** (Level farbig, Datum, Stacktrace – legacy Log-Modus) und
  **INI/Properties/TOML**. Erkennung nach Endung (TypeScript → JavaScript), Dateiname (Makefile, Dockerfile) und
  erster Zeile (Shebang); „Sprache ändern…“ (Palette, Ansicht-Menü) mit filterbarer Liste.
- **E2 Nur der Viewport wird hervorgehoben:** Parse-Zustände an Checkpoints alle 128 Zeilen; ein Frame parst ab
  dem nächsten Checkpoint oberhalb; eine Bearbeitung verwirft die Checkpoints ab der geänderten Zeile
  (`Document::take_changed_from`). Zeilen über 4 000 Zeichen und Langzeilen bleiben ungefärbt.
- **E3 Weite Sprünge in großen Dateien:** liegt der letzte exakte Zustand mehr als 1 000 Zeilen über der Ansicht,
  beginnt die Ansicht mit frischem Zustand 300 Zeilen darüber (`approxHighlight`), und ein Hintergrund-Thread parst
  auf einem Rope-Schnappschuss bis dorthin; Zustände oberhalb einer zwischenzeitlichen Bearbeitung bleiben gültig.
  Die UI fragt bei `approxHighlight` nach 250 ms erneut nach einem Frame.
- **E4 Stilklassen statt Farben im Backend:** Scopes (innerste zuerst, `punctuation`/`meta` geben an den Scope
  außen ab) → 25 Klassen (`highlight::CLASSES` = `STYLE_CLASSES`); die Farben kommen aus CSS-Tokens `--syn-*` je
  Schema. Läufe tragen `style`.
- **E5 Oniguruma** (C, ~3,5× schneller als das reine-Rust-Regex) für die App; Feature `fancy` für WebAssembly.
  `ParseState` enthält mit Oniguruma einen Puffer, der nicht `Send` ist: eine kleine `Snapshot`-Hülle mit
  begründetem `unsafe impl Send` (Zustände werden nur geklont oder hinter einem Mutex verwendet).
- **E6 Farbschemata** (legacy: vier eingebaute Stile, F12 schaltet weiter): System folgen, Paper (hell), Ink (dunkel),
  Sand (warm hell), Night (kontrastreich dunkel). F12 und Ansicht › Farbschema; das Schema setzt `data-theme`
  (Chrome) und `data-scheme` (Editor). Gespeichert wird es ab 009 in den Einstellungen.

## Messungen (Release, 100 000 Zeilen Rust)

Erster Frame 0,2 ms · erste Taste 2 000 Zeilen tief (Aufholen) 14 ms · 20 Tasten 100 ms (5 ms je Taste) · Frame
am Dateiende (ungefähr) 0,25 ms.

## Tests

- Rust `highlight` (Erkennung, Rust-Klassen, Blockkommentar über 500 Zeilen + Invalidieren, ungefähr → exakt nach
  Hintergrund-Parse, Log-Level), Sitzung (Läufe mit Stil, Sprache wechseln, Tippen invalidiert).
- Frontend: `chrome.store.spec` (F12-Zyklus, Attribute, Menüleiste), `app.spec` (F12 setzt `data-theme`/`data-scheme`,
  Sprache über die Auswahlliste).
- Ende-zu-Ende: `search.rs` in Paper, Ink und Night.
