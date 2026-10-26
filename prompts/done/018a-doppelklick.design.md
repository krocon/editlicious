# Design 018a: Fortschreitende Auswahl per Doppelklick

> Prompt: [018a-doppelklick.md](018a-doppelklick.md) (von Marc). Stand 26.10.2026.

## Bestand (Analyse)

- Editor-Komponente: eigene Ansicht (`EditorView`), die Engine (`ed-core`, `editor/mod.rs`) wählt aus;
  Maus: `mousedown` → `Command::Click { line, cell, count = event.detail (1–3), extend, add }`, `Drag`.
- Bisher: `count 2` = Wort (`word_at`, `_` gehört zum Wort), `count 3` = Zeile, Ziehen nach Doppelklick wortweise.
- Auswahl-API: `Session.cursors` / `Command::Select` (aus 015); Syntaxinformationen (Sprache) gibt es, werden
  für diese Regel nicht gebraucht – eine einheitliche Gewichtsregel deckt Text, Code, Pfade, URLs, Konfiguration
  und Markdown ab.

## Entscheidungen

- **E1 Gekapselt in `editor/expand.rs`** (rein, ohne Session): `line_steps(text, ch, weights)` liefert alle
  Schritte einer Zeile, `steps(doc, pos)` hängt Zeile und Absatz an (Absatz höchstens 10 000 Zeilen).
- **E2 Eine Regel statt Sonderfällen:** Gewichte (×10 als ganze Zahlen) `_` 1, `-` 2, `.` 3, Pfad-/URL-/Schlüssel-
  Trenner 3,5, übrige Satzzeichen 3,8, Leerzeichen 4, Tab 5. Ein Schritt überquert die **leichteste** Grenze neben
  der Auswahl und nimmt das Stück dahinter bis zur nächsten mindestens gleich schweren Grenze; bei Gleichstand
  das kleinere Stück, dann links. Klammer- und Anführungspaare (Apostroph im Wort zählt nicht) sind eigene
  Kandidaten (Inhalt, dann mit Zeichen) und begrenzen das Wachsen auf ihren Inhalt; Paare daneben zählen als
  Ganzes. Nächster Schritt = kleinster Kandidat, der die Auswahl echt enthält → nie kleiner, nie Sprünge.
- **E3 Folge und Unterbrechung:** die Sitzung merkt sich Schritte, aktuellen Index und Text-Revision. Ein
  Doppelklick innerhalb der aktuellen Auswahl bei gleicher Revision → nächster Schritt; nach dem größten wieder
  das Wort. Der Einzelklick, den der Browser vor jedem Doppelklick schickt, behält die Folge, wenn er innerhalb der
  Auswahl landet – so funktionieren auch langsame Wiederholungen ohne eigenes Zeitfenster (die Doppelklick-
  Erkennung bleibt die des Systems). Jeder andere Befehl (Bewegen, Tippen, Ziehen, Undo …) beendet die Folge.
- **E4 Klickzahlen:** `detail` 1 = Caret, 2 = Wort/weiter, 3 = Zeile (wie bisher), ab 4 = weitere Doppelklicks.
- **E5 Erster Schritt** ist jetzt das Teilwort (`profile` statt `user_profile_name`, wie der Prompt verlangt);
  Ziehen nach einem Doppelklick bleibt wortweise.
- **E6 Konfigurierbar** über `settings.json` → `expandWeights` (Zeichen oder `space`/`tab`/`punctuation`,
  0,1–9), geprüft beim Laden, an die Engine per `configure`. Doku: `docs/selection.md`.

## Tests

- `expand.rs`: alle 15 Fälle des Prompts Schritt für Schritt (Wort, `_`, `-`, `.`, Leerzeichen, gemischt, Pfad, URL,
  Klammern, Anführungszeichen, Apostroph, Markdown-Link, Umlaute, Emoji/kombinierende Zeichen, eigene Gewichte),
  jede Folge wächst echt und endet bei der Zeile, Absatz.
- Sitzung: ganzer Zyklus inkl. Neustart, Anfang/Mitte/Ende eines Tokens, langsame Wiederholung, andere Stelle,
  Unterbrechung durch Bewegung, Tippen, Undo, Dreifachklick, breite Zeichen, Einstellungen.
- Angular: Abbildung von `detail` auf die Klickzahl.
- Ende-zu-Ende: fünf Doppelklicks in der echten App auf einen Pfad im Import (Screenshots je Schritt).
