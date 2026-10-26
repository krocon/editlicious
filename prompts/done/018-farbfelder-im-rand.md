# Prompt 018: Farbwerte als Farbfelder im linken Rand

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 008, 009, 010. Legacy: §8 („Farbe als
> Hintergrund für Farbwerte wie `#7dff333a`“), Webseite der alten Version („Farbwerte als Farbfelder im linken
> Rand“). Unabhängig von 015–017, darf vorher umgesetzt werden.

## Ziel

Steht in einer Zeile ein erkannter Farbwert, zeigt der linke Rand (neben den Zeilennummern) ein kleines Farbfeld in
dieser Farbe – wie in der alten Version:

```
■  bg="ffffff00" fontSize=…      (gelb)
■  bg="ff0098db" fontSize=…      (blau)
```

Ruhig und nützlich: keine neuen Bedienelemente, nur ein schmaler Streifen, der bei Bedarf erscheint.

## Umfang

- **Erkennung in Rust** (`ed-core`, z. B. `src/editor/colors.rs`), nur für die Zeilen des aktuellen Frames, als
  Teil der Frame-Daten (`colors: [{ line, rgba, start, end }]`, pro Zeile höchstens wenige Einträge). Nichts
  Dateigroßes über IPC.
- **Formate** (im Design mit Beispielen und Gegenbeispielen festlegen):
  - CSS: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()/rgba()`, `hsl()/hsla()` (Komma- und Leerzeichen-Syntax,
    Prozent), optional benannte CSS-Farben nur in CSS/SCSS/LESS/HTML.
  - Hex-Literale: `0xRRGGBB`, `0xAARRGGBB` (Java/Kotlin/Android, C#).
  - **Nackte Hex-Werte in Anführungszeichen** wie im Bild: `"ffffff00"` = AARRGGBB (8 Stellen), `"0098db"` =
    RRGGBB (6 Stellen). Nur in Anführungszeichen oder als ganzer Attributwert, sonst zu viele Fehltreffer
    (Git-Hashes, IDs).
  - Mehrdeutigkeit 8 Stellen: mit `#` → CSS `RRGGBBAA`, mit `0x` oder nackt → `AARRGGBB`; Abweichungen je Sprache
    im Design entscheiden (Einstellung „8-stellige Werte als ARGB/RGBA“ nur, wenn nötig).
  - Wortgrenzen beachten (`#ffffff` in `#ffffffx` ist keine Farbe), Groß-/Kleinschreibung egal.
- **Darstellung** (Angular, `EditorView`):
  - Farbfeld ca. eine Zeilenhöhe minus Rand, quadratisch, 1-px-Kontur (`--ed-*`-Token), damit Weiß auf Hell und
    Schwarz auf Dunkel sichtbar bleibt; Transparenz über Schachbrettmuster.
  - Mehrere Farben in einer Zeile: erstes Feld voll, bei 2–3 Farben geteiltes Feld; Tooltip/`title` listet alle
    Werte. Die Spalte erscheint nur, wenn im Dokument (bzw. im Viewport, im Design entscheiden – ohne Springen der
    Breite beim Scrollen) Farben vorkommen.
  - Bei weichem Umbruch (010) steht das Feld in der ersten Teilzeile.
- **Optional** (legacy §8): Farbwert im Text dezent hinterlegen oder unterstreichen – als eigene Einstellung,
  standardmäßig aus.
- **Interaktion** (klein halten): Klick auf das Feld wählt den Farbwert im Text aus; Aktion „Farbwert umrechnen“
  (Befehlspalette) wandelt den Wert unter dem Cursor zyklisch um (`#rrggbb` ↔ `rgb()` ↔ `hsl()` ↔ `0xAARRGGBB`),
  in Rust, als eine Undo-Gruppe. Ein Farbwähler ist ausdrücklich **nicht** Teil dieses Prompts (Folge-Idee).
- **Einstellung** (009): „Farbfelder im Rand anzeigen“ (Standard an); Aktion zum Umschalten im Katalog, ohne
  Standard-Kürzel. Texte in `*-texts.ts` (DE/EN), Barrierefreiheit: Felder `aria-hidden`, Farbwert im Tooltip und
  über die Statusleiste/Live-Region abrufbar, wenn der Cursor darauf steht.

## Regeln und Randfälle

- Sehr lange Zeilen: nur innerhalb einer festen Grenze ab Zeilenanfang suchen (z. B. erste 4 096 Zeichen) oder im
  sichtbaren Spaltenfenster – im Design entscheiden und messen; nie die ganze 100-MB-Zeile scannen.
- Ergebnisse je Zeile mit der Dokument-Revision cachen (wie die Syntax-Checkpoints aus 008); Bearbeiten
  invalidiert nur die betroffene Zeile.
- Ungültige Werte (`#ggg`, `rgb(300,0,0)`) → kein Feld (bzw. geklemmt, im Design festlegen und testen).
- Leistung: Erkennung pro Frame ≤ 1 ms bei 100 sichtbaren Zeilen (Messung in den ignored-Tests aus 013).
- Webseiten-Demo (011): Funktion steht automatisch in `ed-wasm` zur Verfügung; Screenshot auf der Webseite ergänzen.

## Tests

- Rust, datengetrieben (`testdata/colors.cases.json`): alle Formate, Alpha, Mehrdeutigkeit 8 Stellen,
  Wortgrenzen, mehrere Farben je Zeile, Fehltreffer-Liste (Git-Hashes, UUIDs, `#include`, Ankerlinks `#top`),
  Unicode vor dem Wert (Spaltenangaben in Graphemen/Zellen korrekt), lange Zeile mit Grenze.
- Umrechnen: Hin- und Rückweg verlustfrei (bis auf Rundung bei HSL, dokumentiert), Undo.
- Angular: Feld-Darstellung (Farbe, Schachbrett bei Alpha, geteiltes Feld), Spalte ein/aus, Einstellung, Klick
  wählt aus.
- Ende-zu-Ende: generierte Datei wie im Bild (`bg="ffRRGGBB"`) und eine CSS-Datei, Screenshots hell und dunkel.

## Akzeptanzkriterien

- Die Zeilen aus dem Beispielbild zeigen gelb, blau, magenta, grün, rot, lila, orange, dunkelblau und hellgrau –
  wie in der alten Version.
- Eine 1-GB-Datei ohne Farben verhält sich unverändert (keine Randspalte, keine messbare Verlangsamung).
