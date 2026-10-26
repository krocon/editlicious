# Design 019: Wort-Marker ausbauen und beim „Als HTML kopieren“ übernehmen

> Prompt: [019-marker-ausbau-html.md](019-marker-ausbau-html.md). Stand 26.10.2026.

## Entscheidungen

- **E1 Marker im HTML:** `copyHtml` bekommt die zehn Markerfarben, wie die Ansicht sie zeichnet (Proben-Elemente →
  `getComputedStyle`), und den Editor-Hintergrund. Die Engine mischt sie zu **deckendem `#rrggbb`** (Mail-Programme
  ignorieren Transparenz), setzt sie als `background-color` auf die Spans (Syntaxfarbe bleibt Schrift) und gibt dem
  `<pre>` die Hintergrundfarbe. Überlappend gewinnt der zuletzt gesetzte Marker (wie gezeichnet). Auch lange
  Zeilen und mit Zeilennummern; Wortgrenzen am Rand der Auswahl werden mit Kontext erkannt. Blockauswahl: wie
  bisher die Zeilen der Hauptauswahl (Kopieren aus 007 bleibt unverändert).
- **E2 Markieren der Auswahl** gab es schon (einzeilig); neu: ganze-Wort-Suche nur, wenn der Text mit Wortzeichen
  beginnt und endet (sonst würde `" ro"` nie gefunden), Groß/klein beachtet wie legacy, höchstens 1000 Zeichen,
  nie reiner Leerraum.
- **E3 Feste Farbe:** `setMarker { colour }`, Aktionen „Wort in Farbe 1…10 markieren“ (Palette) – dasselbe Wort
  wandert dorthin. **Keine Standardkürzel:** Strg+Alt(+⇧)+Ziffer ist unter Windows AltGr und tippt auf deutschen
  Tastaturen `{ [ ] }` – eigene Kürzel über `keymap.json`.
- **E4 Springen:** `markerStep { backwards }` – nächstes/voriges Vorkommen des Markers unter dem Cursor, mit
  Umlauf, in Rust (`find_next/find_prev`); **F7 / ⇧F7** (frei, nahe legacy Strg+⇧+F7). Statusleiste: „row · 7×“
  mit Farbfeld, wenn der Cursor auf einem markierten Wort steht – gezählt in Rust auf einem Schnappschuss
  (`count_marker`, eigener Abbruch-Platz, damit die Zählung der Suchleiste nicht stört), ab 10 000 „> 9 999“.
- **E5 Sitzung:** `Frame.markerWords` → `session.json` je Datei (`markers`, geprüft: ≤ 10, ≤ 1000 Zeichen) →
  beim Wiederherstellen `setMarkers`.
- **E6 Kontraste:** Test über alle Schemata (aus `styles.css`): Text auf jeder Markerfarbe ≥ 4,5 : 1. Gefunden:
  „Sand“ lag bei 3,2–4,3 (schon der Text auf dem Grund nur 5,0) → Textfarbe `#45575e` (7,0 : 1), Marker 7 und 9 dort
  etwas heller.

## Tests

- Rust: HTML mit Markern (Farben gemischt, dreimal „row“, Hintergrund am `<pre>`, ohne Marker wie bisher), feste
  Farbe/Wandern, Springen mit Umlauf, kein Marker → Hinweis, Auswahl markieren, Wiederherstellen, Leerraum nie,
  Sitzungsdatei (Rundreise, Grenzen), Kontrasttest.
- Angular: Markerfarben gehen mit, Zählung in der Statusleiste, Sitzung speichern/wiederherstellen.
- Ende-zu-Ende: Java-Datei wie im Bild, zwei Wörter markiert, F7, Strg+⇧+C, HTML aus der Zwischenablage
  (`xclip -t text/html`) gerendert.
