# Design 018: Farbwerte als Farbfelder im linken Rand

> Prompt: [018-farbfelder-im-rand.md](018-farbfelder-im-rand.md). Stand 26.10.2026.

## Entscheidungen

- **E1 Erkennung in Rust** (`ed-core/src/editor/colors.rs`), eine Regex über die ersten **4096 Zeichen** jeder
  sichtbaren Zeile (auch bei langen Zeilen nie mehr), höchstens **3 Farben je Zeile**. Messung (Release, 100 Zeilen
  mit 300 Farben je Frame): 0,79 ms statt 0,32 ms ohne – Ziel ≤ 1 ms erreicht; ohne Treffer kostet eine Zeile
  nur einen `contains`. Kein Zwischenspeicher nötig.
- **E2 Formate:** `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` (nicht nach Wortzeichen, `&` oder `#`, Wortgrenze danach),
  `rgb()/rgba()` und `hsl()/hsla()` (Kommas oder Leerzeichen, Prozent, `/ alpha`, `deg`; Werte außerhalb des
  Bereichs → keine Farbe), `0xRRGGBB`/`0xAARRGGBB`, nackte Hex-Werte **nur in Anführungszeichen** (8 Stellen =
  AARRGGBB wie im Bild der alten Version, 6 = RRGGBB). Benannte CSS-Farben bewusst nicht (zu viele Fehltreffer in
  normalem Text; Folge-Idee). Mehrdeutigkeit 8 Stellen: `#` = CSS RRGGBBAA, `0x`/nackt = AARRGGBB.
- **E3 Frame:** `FrameLine.colors = [[von, bis, 0xRRGGBBAA]]` (Zeichen), `Frame.colorColumn` (bleibt an, sobald im
  Dokument einmal eine Farbe sichtbar war – die Gutter-Breite springt beim Scrollen nicht),
  `Frame.caretColor` (Wert am Cursor für Statusleiste/Screenreader).
- **E4 Darstellung:** Farbfeld (Zeilenhöhe − 8 px) links im Gutter, 1-px-Kontur in Gutter-Textfarbe, Transparenz über
  Schachbrett, 2–3 Farben nebeneinander, Tooltip mit allen Werten; Klick wählt den (ersten) Farbwert im Text aus.
  Statusleiste: „■ #2f98e1“ mit `aria-label` „Farbe #2f98e1“, wenn der Cursor auf einem Wert steht.
- **E5 Umrechnen** (`convertColor`, Aktion „Farbwert umrechnen“): `#…` → `rgb()` → `hsl()` → `#…`; `0x…` und
  nackte Werte → `#…`. Ein Undo-Schritt, der neue Wert ist danach ausgewählt; ohne Farbwert Hinweis `no_color`.
- **E6 Einstellung** `colorSwatches` (Standard an) + Aktion `view.colorSwatches`; aus = keine Erkennung, keine
  Spalte. Die optionale Hinterlegung im Text (legacy §8) und ein Farbwähler bleiben Folge-Ideen.

## Tests

- Rust: alle Formate, Alpha, Android-Werte aus dem Bild, Fehltreffer-Liste (`#include`, `#top`, Git-Hashes,
  UUIDs, Entities, `#ffffffx`, Werte außer Bereich), Zeichenpositionen nach Umlauten/Emoji, Grenze 3/Zeile und
  4096 Zeichen, Umrechnen hin und zurück; Sitzung: Frame-Felder, Spalte bleibt, Umrechnen + Undo, aus.
- Angular: Farbfelder (Farbe, geteilt, Tooltip), Klick wählt aus, `configure` mit `colors`, Statusleiste.
- Ende-zu-Ende: Datei wie im Bild (9 Farben wie in der alten Version) + CSS, Klick auf das Feld, Umrechnen.
