# Prompt 020: Intelligente Tooltips und Rechen-Vorschläge (Math-Parser)

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 005, 009, 010. Legacy: alte Webseite
> („Intelligente Tooltips zeigen zum Beispiel auch Ergebnisse arithmetischer Funktionen“, Bild: Zeile `255/8`,
> Tooltip `(255.0 / 8.0) = 31.875`). Vorlage für den Parser: Marcs Java-Projekt
> `~/IdeaProjects/java-k-editor/MathParser` (in Remote-Sitzungen per `device_bash` lesen). Unabhängig von 015–019.

## Ziel

Steht am Caret ein Rechenausdruck, zeigt editlicious das Ergebnis dezent als Tooltip **und** bietet es als
Autovervollständigung an: `255/8` → Tooltip `255 / 8 = 31.875`; `255/8=` → Vorschlag `31.875`, Tab/Enter fügt ein.
Dabei entstehen zwei allgemeine Bausteine, die spätere Funktionen nutzen: **Hinweise am Caret** (Tooltip-Anbieter)
und **Vervollständigung** (Vorschlags-Anbieter). Rechnen ist jeweils der erste Anbieter.

## Vorlage MathParser (Java, Stand der Analyse)

- `Parser.compile(String)` → Baum aus `Calculation` (`getValue(): double`, `toString()` liefert die
  geklammerte Form wie im Tooltip). Operatoren `+ - * / ^`, Klammern, führendes Minus, Zahlen mit Punkt.
- Einwertige Funktionen (`OneValueCalculationFactory`): `sin cos tan asin acos atan sinh cosh tanh radians rad
  degrees exp expm1 log log10 log1p sqrt cbrt ceil floor rint round abs ulp signum fac`; dreiwertig
  `convert(a, b, c)` = `a / b * c` (Dreisatz). Konstanten `pi`/`PI`/`Pi`, `e`/`E`; Variablen über `VariableScope`.
- `Parser.getFixedExpression` / `Arranger.arrange`: Ausdruck aufräumen – fehlendes `*` ergänzen (`3x`, `(1+2)(3)`,
  `2 sin(x)`), offene Klammern schließen, Dezimalkomma `1,5` → `1.5`, Operatoren am Anfang/Ende entschärfen.
- Bekannte Schwächen (im Code als TODO bzw. aus der Analyse): `fac` läuft bei großen Werten über (`1/fac(33)`
  falsch), `^` ist links- statt rechtsassoziativ (`2^3^2` = 64 statt 512), Rangfolge über Gewichte mit `TreeMap`
  statt Grammatik, Funktionsnamen per Regex auf 3–7 Kleinbuchstaben begrenzt. Der Testfall
  `"2 + 32/8^(2+3)" → 22.0` in `MathParserTest` ist mathematisch falsch (richtig ≈ 2.000977) – prüfen.
- Numerische Integration (`algo/numerical`) gehört **nicht** zu diesem Prompt (Folge-Idee).

## Umfang

1. **Parser in Rust neu** (`ed-core`, z. B. `src/math/` – rein, ohne Tauri, WASM-fähig, damit auch die
   Webseiten-Demo rechnet): fachlich wie die Vorlage, technisch als kleiner Pratt-/Rekursiver-Abstieg-Parser mit
   sauberer Rangfolge (`^` rechtsassoziativ, unäres Minus, `%` optional), alle Funktionen und Konstanten der
   Vorlage, `convert`, `fac` als Gamma bzw. mit Überlaufprüfung. Eigene Implementierung statt Crate
   (`meval`/`evalexpr`/`fasteval` im Design kurz bewerten; Begründung in `docs/architecture.md`).
   Zahlenformate: Dezimalpunkt, Dezimalkomma (nur wenn eindeutig, z. B. kein `f(1,2)`), Exponent `1e-3`,
   `0x1f`, `0b101`, Tausendertrenner `_`. Ergebnis als `f64`; Ausgabe mit höchstens 15 signifikanten Stellen,
   ohne `.0` bei ganzen Zahlen; Ganzzahlen zusätzlich in Hex/Binär im Tooltip (optional, Design).
2. **Ausdruck finden** (Rust, nur aktuelle Zeile, Länge begrenzt, z. B. 1 000 Zeichen; lange Zeilen → nichts):
   - Auswahl (einzeilig) ist ein Ausdruck, **oder**
   - die ganze (getrimmte) Zeile ist ein Ausdruck, **oder**
   - der längste Ausdruck, der direkt am Caret endet (mit oder ohne abschließendes `=`).
   - **Fehltreffer vermeiden** (datengetriebene Gegenbeispiele): einzelne Zahl, Datum `2026-10-26`, Uhrzeit,
     Version `1.2.3`, IP, Telefonnummer, Pfad `a/b`, `i++`, Kommentarstriche `---`, Markdown-Listen `- 1`, CSS-
     Werte wie `#2f98e1`. Mindestens ein Operator oder eine Funktion und ein endliches Ergebnis.
   - Optional (Design entscheiden): Variablen aus Zuweisungszeilen `x = 3` oberhalb (höchstens N Zeilen).
3. **Hinweise am Caret (Tooltip-Anbieter)**:
   - Engine liefert im Frame (oder per eigenem Aufruf nach kurzer Ruhe, z. B. 150 ms – im Design entscheiden) einen
     Hinweis `{ kind: 'math', range, text }`, z. B. `255 / 8 = 31.875`. Die UI zeigt ihn als kleines Kästchen unter
     dem Ausdruck (wie im Bild), ohne den Fokus zu nehmen; Mausüber-Hinweise mit Verzögerung.
   - Schließt bei Esc, Tippen außerhalb, Scrollen. Für Screenreader in die Live-Region (gedrosselt).
   - Allgemein gebaut: weitere Anbieter (z. B. Farbwerte aus 018, Zeichencode unter dem Caret) können später
     andocken, ohne die UI zu ändern.
4. **Vervollständigung (Vorschlags-Anbieter)**:
   - Erster allgemeiner Vorschlags-Popup des Editors (Combobox/Listbox nach APG, `aria-activedescendant`), am
     Caret positioniert, Tastatur über Aktionen (`completion.next/prev/accept/close`), Tab/Enter übernimmt, Esc
     schließt. Vorschläge kommen aus Rust (`complete(caret) → items`), damit später Wort-Vervollständigung
     (Folge-Idee „Autovervollständigung“) denselben Weg nutzt.
   - Rechen-Anbieter: nach `=` am Ende eines Ausdrucks automatisch, sonst über Aktion „Vorschläge zeigen“
     (Strg+Leertaste; Konflikt mit IME/Eingabequellen auf macOS im Design prüfen). Einträge: Ergebnis einfügen
     (`31.875`), Ausdruck durch Ergebnis ersetzen, gerundet (Stellen aus Einstellung), bei Ganzzahlen Hex.
   - Übernehmen ist eine Undo-Gruppe; der Cursor steht danach hinter dem Eingefügten.
5. **Aktionen**: „Ausdruck ausrechnen und ersetzen“, „Ergebnis kopieren“, „Vorschläge zeigen“ (Katalog,
   Befehlspalette, Menü Bearbeiten), deutsche Namen in `action-catalog.de.ts`.
6. **Einstellungen** (009): „Rechen-Hinweise anzeigen“ (Standard an), „Ergebnis nach `=` vorschlagen“ (Standard
   an), Nachkommastellen (Standard: automatisch), Dezimaltrenner für die Ausgabe (Punkt/Komma/nach Sprache).

## Regeln und Randfälle

- Nichts Dateigroßes über IPC: nur Ausdruck, Bereich und Ergebnistext.
- Fehler beim Parsen sind kein Fehlerfall für den Nutzer: dann einfach kein Hinweis (keine roten Meldungen).
  Division durch 0, `NaN`, `∞`: Hinweis zeigt „nicht definiert“ bzw. `∞`, kein Vorschlag.
- Laufzeit begrenzen (Tiefe, Länge, `fac` bis 170); kein Hängen bei bösartiger Eingabe (Fuzz-Test).
- Unicode: Operatoren `×`, `÷`, `−` (U+2212) werden akzeptiert; Spaltenangaben in Graphemen.
- Kein neues sichtbares Bedienelement außer Tooltip und Popup, beide nur bei Bedarf.

## Tests

- Rust: Port aller Fälle aus `MathParserTest`, `ArrangerDemo`, `ParserFixedExpressionDemo` als datengetriebene
  Datei `testdata/math.cases.json` (mit korrigiertem Erwartungswert und Kommentar), Rangfolge/Assoziativität,
  Funktionen, Konstanten, Zahlenformate, Formatierung der Ausgabe, Fehlerfälle, Fuzzing (`proptest` o. ä.,
  Begründung bei neuer Abhängigkeit); Ausdruckssuche am Caret mit Positiv- und Gegenbeispiel-Liste;
  Vorschläge und Übernehmen inkl. Undo.
- Angular: Tooltip-Position/-Schließen, Popup-Tastatur (APG), Aktionen, Einstellungen, Live-Region.
- Ende-zu-Ende: `255/8` tippen → Tooltip wie im Bild; `(1+2)*3=` → Vorschlag `9`, Tab fügt ein; Screenshots hell
  und dunkel.

## Akzeptanzkriterien

- Wie im Bild: Caret hinter `255/8` zeigt `255 / 8 = 31.875`.
- `2^3^2=` schlägt `512` vor; `1/fac(33)` liefert ein korrektes, positives Ergebnis.
- In einer CSS-, Java- und Markdown-Datei erscheinen beim normalen Tippen keine störenden Fehl-Hinweise.
