# Prompt 019: Wort-Marker ausbauen und beim „Als HTML kopieren“ übernehmen

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 007 (Marker, Design E6), 010 (Als HTML
> kopieren, Design E5). Legacy: „Mark Text“ Strg+⇧+F7, „Clear All Markers“ Strg+⇧+F8; alte Webseite: „bis zu
> 10 Wörter farblich unterschiedlich markieren … werden beim Als-HTML-kopieren übernommen und können so leicht in
> E-Mails übertragen werden“. Unabhängig von 015–018.

## Ausgangslage (Stand nach 007/010)

- Vorhanden: `edit.toggleMarker` / `edit.clearMarkers`, `MAX_MARKERS = 10` (`core/src/editor/ops.rs`), Farben als
  Tokens `--ed-marker-0…9` (halbtransparent), Frame-Feld `markers`, Marker-Striche in der Übersicht (010 E3).
- **Lücke:** `copy_html` (010 E5) schreibt nur Syntax-Stile – Marker fehlen im HTML. Genau das ist das Kern-
  versprechen der alten Version („leicht in E-Mail übertragen“).

## Umfang

1. **Marker im HTML** (Kern):
   - `copy_html` legt Marker-Bereiche über die Syntax-Spans (Bereiche schneiden, Reihenfolge: Syntaxfarbe für die
     Schrift, Markerfarbe als `background-color`). Überlappende Marker: der zuletzt gesetzte gewinnt (wie in der
     Ansicht – im Design prüfen und angleichen). Auch bei Zeilennummern-Variante und bei langen Zeilen
     (`is_long`) – dort bisher ohne Spans, Marker trotzdem übernehmen.
   - **Deckende Farben für E-Mail**: viele Mail-Programme (Outlook, Gmail-Verlauf) stellen `rgb(… / 30%)` falsch
     oder gar nicht dar. Die UI schickt die Markerfarben wie die Syntaxfarben mit (Proben-Spans →
     `getComputedStyle`), die Engine mischt sie mit dem Hintergrund des aktuellen Schemas zu deckendem `#rrggbb`
     (reine Funktion, getestet). Zusätzlich `<pre>` mit `background-color` des Schemas, damit das Ergebnis in
     hellen und dunklen Mail-Programmen gleich aussieht.
   - Auch ohne Auswahl (aktuelle Zeile) und für Blockauswahl (im Design entscheiden: Rechteck oder ganze Zeilen,
     wie beim Kopieren aus 007).
2. **Marker-Bedienung abrunden** (klein, ohne neue sichtbare Elemente):
   - Markieren der **Auswahl** (nicht nur ganzes Wort), wenn eine einzeilige Auswahl besteht; Option im Design:
     „ganzes Wort“ / „Groß-/Kleinschreibung beachten“ (Standard wie legacy).
   - **Zum nächsten/vorigen Vorkommen** des Markers unter dem Caret springen (Aktionen, Kürzel im Design vorschlagen,
     Konflikte mit `keymap.*.json` prüfen); Suche in Rust in Zeilenblöcken wie 006, abbrechbar.
   - **Gezielt eine Farbe wählen**: Aktionen `edit.marker.1…0` (Strg+⇧+Alt+1…0 o. ä., im Design prüfen), damit
     dasselbe Wort in Videos/E-Mails immer dieselbe Farbe bekommt.
   - Marker-Anzahl je Farbe in der Statusleiste, wenn der Caret auf einem markierten Wort steht („row · 7×“),
     gezählt auf dem Schnappschuss wie bei der Suche, mit Obergrenze und „> 9 999“.
   - Marker **je Dokument in der Sitzung** (`session.json`) merken und beim Wiederherstellen setzen.
3. **Kontraste**: die zehn Marker-Farben in hell und dunkel mit allen Syntax-Schemata aus 008 prüfen (Text auf
   Marker ≥ 4,5 : 1 bzw. dokumentierte Ausnahme); ggf. Tokens nachschärfen.

## Regeln und Randfälle

- Nichts Dateigroßes über IPC: Marker-Bereiche nur für den Frame bzw. für den kopierten Bereich (≤ 20 000 Zeilen,
  bestehendes `html_too_large`).
- Marker sind Anzeige, kein Dokumentinhalt: kein Undo-Eintrag, `modified` bleibt unverändert.
- Unicode: Wortgrenzen und Spalten in Graphemen (Emoji, CJK, kombinierende Zeichen), Regex-Sonderzeichen im
  markierten Text werden escaped.
- HTML-Escaping bleibt vollständig; keine Klassen, nur Inline-Stile (Mail-Programme entfernen `<style>`).

## Tests

- Rust: `copy_html` mit Markern (über Syntax-Spans geschnitten, überlappend, über Zeilengrenzen der Auswahl, lange
  Zeile, mit/ohne Zeilennummern) als Snapshot-HTML; Farbmischung RGBA + Hintergrund → `#rrggbb`; Marker aus
  Auswahl, feste Farbe, Springen vor/zurück mit Umlauf, Zählen mit Obergrenze; Sitzung speichern/laden.
- Angular: Aktionen, Statusanzeige, mitgeschickte Markerfarben, Wiederherstellung.
- Ende-zu-Ende: Java-Datei wie im Bild, `row` und `col` markieren, als HTML kopieren, Zwischenablage per
  `xclip -o -t text/html` prüfen und das HTML als Screenshot rendern (Chromium/Playwright, hell und dunkel).

## Akzeptanzkriterien

- Wie im Bild: `row` gelb, `col` blau markiert, Strg+⇧+C, in eine E-Mail eingefügt → Syntaxfarben **und** Marker
  sind sichtbar, auch in einem Mail-Programm ohne RGBA-Unterstützung.
- Nach Neustart sind die Marker der wiederhergestellten Dokumente wieder da.
