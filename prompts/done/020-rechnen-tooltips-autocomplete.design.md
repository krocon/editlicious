# Design 020: Rechen-Hinweise und Vorschläge (Math-Parser)

> Prompt: [020-rechnen-tooltips-autocomplete.md](020-rechnen-tooltips-autocomplete.md). Stand 26.10.2026.

## Entscheidungen

- **E1 Parser selbst** (`ed-core/src/math.rs`, Architektur A9): rekursiver Abstieg mit sauberer Rangfolge
  (`+ -` < `* / % ×÷` und implizites Mal < unäres Minus < `^` rechtsassoziativ < Fakultät `!`), Funktionen der
  Vorlage (`sin … signum`, `fac`, `ln`), `convert(a, b, c) = a / b · c`, Konstanten `pi`/`e`, Variablen. Zahlen:
  Punkt, Dezimalkomma nur außerhalb von Funktionsargumenten (`1,5`, nicht `f(1,2)`), `1e-3`, `0x1f`, `0b101`, `1_000`. Fehlende `)` werden ergänzt (Vorlage `Arranger`).
  Grenzen: 1 000 Zeichen, Tiefe 64; `fac` exakt bis 170, darüber ∞, Nicht-Ganzzahlen über Γ (Lanczos).
  `crate`-Bewertung: `meval` (veraltet, kein Komma/kein implizites Mal), `evalexpr` (eigene Sprache mit Strings
  und Zuweisungen – zu viel), `fasteval` (unsafe, ungepflegt) – keins liefert den aufgeräumten Ausdruck.
- **E2 Der Testfall der Vorlage** `2 + 32/8^(2+3) → 22.0` ist falsch; richtig ist 2.0009765625 (in
  `testdata/math.cases.json` mit Kommentar).
- **E3 Ausdruck finden** (`editor/hints.rs`, nur eine Zeile, ein Caret, keine lange Zeile): Auswahl → längster
  Ausdruck, der am Caret endet (davor `=` erlaubt, nicht `==`/`!=`/`<=`) → ganze Zeile. Startpunkte nie mitten in
  Name/Zahl/Ausdruck; Gegenbeispiele Datum, Bereich, Version, IP, Telefon, `i++`, `---`, Markdown, CSS, Java (Test-
  Liste). Ein Ausdruck braucht einen Operator oder eine Funktion; unbekannte Namen → kein Hinweis.
- **E4 Variablen**: Zuweisungen `name = Ausdruck` (`;` am Ende erlaubt) in den 200 Zeilen darüber, je 200 Zeichen
  gelesen; spätere überschreiben frühere.
- **E5 Hinweis im Frame** (kein eigener Aufruf nach Ruhezeit): `Frame.hint { line, cell, expr, result, hex }` nur
  für den Haupt-Caret – billig (eine Zeile + Fenster darüber) und ohne zweiten Weg über die IPC. `result: null` =
  „nicht definiert“ (0/0, Division durch 0), ∞ wird gezeigt. Hex ab 16 für ganze Zahlen. Die UI zeigt den Hinweis
  nur mit Fokus, Esc und Scrollen blenden ihn aus, bis sich der Ausdruck ändert; Screenreader bekommen ihn nach
  700 ms Ruhe über die Live-Region, jeden Text einmal; das Textfeld verweist per `aria-describedby` darauf.
  Weitere Anbieter (Farbwerte, Zeichencode) können denselben Platz nutzen – die UI zeigt nur Text.
- **E6 Vorschläge** (`Frame.completion { line, items[{ kind, label }] }`): die Engine hält die Liste samt
  Revision; `complete`, `applyCompletion { index }`, `closeCompletion`. Jede andere Änderung schließt sie, `=`
  am Ende öffnet sie (wenn eingeschaltet). Einträge: Ergebnis einfügen, Ausdruck ersetzen, gerundet (2 Stellen,
  wenn keine feste Stellenzahl), Hex. Übernehmen = ein Undo-Schritt, Cursor dahinter. Veraltete Liste (andere
  Revision) tut nichts. Später: Wort-Vervollständigung über dieselben Befehle mit anderem `kind`.
- **E7 Tastatur**: neuer Aktions-Kontext `completion` vor `editor`, aktiv solange `app-editor-view` das Attribut
  `data-completion` trägt: ↓/↑, Enter/Tab übernehmen, Esc schließt. Popup als APG-Listbox; der Fokus bleibt im
  Textfeld (`aria-activedescendant`, `aria-controls`). „Vorschläge zeigen“: Strg+Leertaste; auf macOS ist das oft
  die Eingabequellen-Umschaltung → zusätzlich ⌥Esc.
- **E8 Aktionen**: `edit.complete`, `edit.evaluate` („ausrechnen und ersetzen“), `edit.copyResult` (Menü
  Bearbeiten, Palette). Hinweise `no_expression`, `result_copied`.
- **E9 Einstellungen**: `mathHints`, `mathSuggest` (an), `mathDecimals` (-1 = nach Bedarf, 0–12),
  `decimalSeparator` (`auto` = nach UI-Sprache, `point`, `comma`) → `configure.math`.
- **E10 Aufnahmen** (015): `complete`/`closeCompletion` werden nicht aufgezeichnet, `applyCompletion`/`evaluate`
  als `select` + `insert` des Ergebnisses – das Abspielen hängt so nicht von Einstellungen oder Liste ab.
- **E11 Webseiten-Demo**: zeigt Hinweis und Vorschläge (Enter/Tab = erster Eintrag), Beispieltext nennt `255/8=`.

## Tests

- Rust: `testdata/math.cases.json` (alle Fälle von `MathParserTest`, Grammatik, Gegenbeispiele), Fakultät/Γ,
  Formatierung, Grenzen, Fuzz (20 000 Zufallseingaben, 5 000-fach geschachtelt); Ausdruckssuche mit Positiv- und
  Gegenbeispielen inkl. CSS/Java/Markdown; Sitzung: Hinweis, Variablen, `=`-Vorschlag, Übernehmen + Undo,
  `evaluate`, `copyResult`, Einstellungen, veraltete Liste, Aufnahme.
- Angular: Konfiguration, Hinweis (Fokus, Esc, „not defined“, `aria-describedby`), Listbox-Tastatur über den
  Dispatcher, Klick.
- Ende-zu-Ende (Xvfb): `255/8` → „255 / 8 = 31.875“; `255/8 + width/3 =` mit Variable → 431.875, ↓ Tab ersetzt;
  `(1+2)*3=` Tab → `9`; `2^3^2=` → 512 (dunkel); `1/fac(33)` → 1.1516335620772e-37.
