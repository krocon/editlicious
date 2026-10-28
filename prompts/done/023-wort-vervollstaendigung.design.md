# Design 023: Wort-Vervollständigung und Sprach-Lexikon

> Prompt: [023-wort-vervollstaendigung.md](023-wort-vervollstaendigung.md). Stand 28.10.2026.

## Entscheidungen

- **E1 Gleicher Weg wie 020**: `complete`, `applyCompletion { index }`, `closeCompletion` und
  `Frame.completion` bleiben; neue Arten `word` (aus dem Dokument) und `keyword` (Lexikon). Die Liste merkt sich,
  ob sie aus Wörtern stammt. Kein neuer Befehl, kein neues DTO-Feld außer `Configure.words`.
- **E2 Vorrang**: Strg+Leertaste prüft zuerst den Rechenausdruck am Caret (020); nur ohne Rechenvorschläge kommen
  Wörter. `=` am Ende öffnet weiter die Rechenliste.
- **E3 Präfix** (`editor/complete.rs`): Wortzeichen (`CharClass::Word`, also Buchstaben, Ziffern, `_`,
  kombinierende Zeichen) direkt links vom Caret, höchstens 64 Zeichen, nicht mit einer Ziffer beginnend. Nur ein
  Caret ohne Auswahl, keine lange Zeile. Ersetzt wird nur das Präfix (`from..caret`), der Rest des Wortes rechts vom
  Caret bleibt – vorhersagbar wie in den meisten Editoren.
- **E4 Kandidaten aus dem Dokument**: Zeilen nach Nähe (Caret-Zeile, dann −1, +1, −2 …), je Zeile höchstens
  1 000 Zeichen um den Anfang, insgesamt höchstens 400 000 Zeichen bzw. 20 000 Zeilen – konstante Kosten auch bei
  1 GB. Wörter 2–64 Zeichen, nicht nur Ziffern, ungleich dem Präfix; das Wort, in dem der Caret steht, zählt nicht
  (sonst schlägt `pub|` sich selbst vor). Nichts geht über die IPC außer den höchstens 12 Einträgen.
- **E5 Rangfolge**: (1) Dokumentwörter mit exakt passendem Präfix nach Nähe, (2) Lexikonwörter mit exakt
  passendem Präfix, (3) Treffer ohne Groß/klein-Unterschied (Dokument, dann Lexikon). Dubletten fallen weg (das
  Dokumentwort gewinnt, Art `word`). Höchstens 12.
- **E6 Lexikon** (`core/lexicon/*.txt`, `include_str!`): eine Datei je Sprache, ein Wort pro Zeile, `#`-Kommentare;
  Kopfzeile `# case-insensitive` für SQL (Schreibweise folgt dem Präfix: ganz klein → klein, ganz groß → groß,
  sonst wie gelistet; steht das Wort schon im Dokument, gilt dessen Schreibweise – `sel` in einer Datei mit
  `SELECT` ergibt `SELECT`). Zuordnung über den Sprachnamen der Hervorhebung (008): Java, JavaScript (auch für
  TypeScript, das syntect nicht kennt – TS-Wörter sind enthalten), Python, SQL (+ „SQL (Rails)“), Rust, C, C++,
  C#, Go, PHP (+ „PHP Source“), Ruby, Shell (bash, „Shell-Unix-Generic“), CSS, HTML (Elemente und Attribute,
  + Varianten „HTML (…)“). Inhalt: Schlüsselwörter, eingebaute Typen und die häufigsten Bezeichner der
  Standardbibliothek – bewusst kurz (≈ 50–150 Wörter), damit die Liste ruhig bleibt. Handgepflegt statt aus den
  syntect-Grammatiken gelesen: dort stecken Wörter in Regexen. Keine neue Abhängigkeit. Eigene Wortlisten des
  Nutzers: Folge-Idee.
- **E7 Einzelkandidat**: Strg+Leertaste mit genau einem Wort-Kandidaten setzt ihn sofort ein (ein Undo-Schritt),
  ohne Liste. Im Auto-Modus nie (dort würde Tippen sonst überraschend Text einfügen).
- **E8 Weitertippen filtert**: Ist eine Wortliste offen, öffnet `type` mit Wortzeichen bzw. `backspace` sie danach
  neu für das neue Präfix (leer → zu). Alles andere schließt sie wie bisher.
- **E9 Auto-Modus** (Einstellung `wordSuggest`, Standard aus; legacy-Todo „erst ab 2 Zeichen“): nach `type` mit
  Wortzeichen und einem Präfix ab 2 Zeichen öffnet die Liste von selbst (ohne Hinweis, wenn leer).
- **E10 Einstellungen** (009): `wordSuggest` (aus), `keywordSuggest` (an) → `configure.words { auto, keywords }`.
- **E11 Aufnahmen** (015/020 E10): unverändert – `applyCompletion` wird als Auswahl + Einfügen aufgezeichnet; das
  Öffnen/Filtern zeichnet nichts auf.
- **E12 UI**: `kindLabel` kennt `word` („Wort“) und `keyword` („Schlüsselwort“); Hinweis `no_suggestions`
  („Keine Vorschläge.“), wenn Strg+Leertaste weder Ausdruck noch Wort findet (ersetzt dort `no_expression`; die
  Aktionen „Ausrechnen“/„Ergebnis kopieren“ melden weiter `no_expression`). Kein neues Bedienelement.

## Verträge

- Rust `model::Command::Configure { …, words: Option<WordConfig { auto: bool, keywords: bool }> }` ↔ TS
  `{ type: 'configure', …, words?: { auto: boolean; keywords: boolean } }`.
- `CompletionItem.kind`: zusätzlich `word`, `keyword`.
- Einstellungen: Rust `settings.rs` `word_suggest`, `keyword_suggest` ↔ TS `wordSuggest`, `keywordSuggest`.

## Tests

- `complete.rs`: Präfix (Unicode, Ziffer am Anfang, Grenze 64), Rangfolge, Groß/klein, Wort am Caret, Budget
  (lange Zeile, viele Zeilen), Lexikon (jede Datei: nur Wortzeichen, keine Dubletten, Sprachzuordnung, SQL-Schreibweise).
- Sitzung (`editor/tests.rs`): Strg+Leertaste öffnet Wörter, Rechnen hat Vorrang, Einzelkandidat sofort,
  Filtern beim Tippen/Rücktaste, Schließen bei Bewegung, Auto-Modus ab 2 Zeichen, Lexikon an/aus, ein
  Undo-Schritt, Aufnahme.
- Frontend: `kindLabel`, Configure mit `words`, Einstellungen-Dialog, Kataloge.
