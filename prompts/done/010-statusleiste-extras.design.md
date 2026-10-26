# Design 010: Statusleiste und Extras

## Entscheidungen

- **E1 Statusleiste** (`features/editor/status-bar.ts`): eine Zeile unter dem Editor, gedämpft. Links Zeile/Spalte
  (Spalte = Zelle + 1, Tabs zählen ihre Breite), Auswahl (Zeichen, Zeilen) bzw. Anzahl Cursor, „ÜB“ im
  Überschreibmodus. Rechts Umbruch-Hinweis, Zeilenzahl, Dateigröße und drei Knöpfe: Sprache, Zeichenkodierung (+ BOM),
  Zeilenende („gemischt“). Die Knöpfe lösen dieselben Aktionen aus wie Palette und Menü. Ein/aus: Strg+F5 (legacy)
  bzw. Einstellung `statusBar`; F1 (alles aus) blendet sie mit aus. Sie lebt in `App`, gespeist vom letzten Frame.
- **E2 Kodierung und Zeilenenden**: Zeichenkodierung … wählt aus `list_encodings` (alle les- und schreibbar,
  `document::ENCODINGS`), dann die Frage „Neu lesen“ (`reload_document(id, encoding)` → `load_file_as`, ohne Raten,
  passendes BOM wird übersprungen) oder „Umgewandelt speichern“ (`save_document` mit `encoding`). Neue Dateien:
  „Speichern unter“ mit der Kodierung. Zeilenenden … wandelt jeden Umbruch um (`SetLineEnding`): ein Durchlauf über
  die Chunks der Rope, ein Undo-Schritt, neue Zeilen danach im neuen Format; 1 Mio. Zeilen ≈ 1,6 s (Release,
  237 MB). Große Edits (> 256 Änderungen) bauen die Rope neu statt in place zu ändern (`REBUILD_FROM`).
- **E3 Übersicht in der Scrollleiste** statt eigener „Mini Map“: Treffer (bit 1) und Marker (bit 2) als Striche
  im senkrechten Balken. `overview(id, buckets)` rechnet auf einem Schnappschuss (Rope-Klon O(1)) im
  Hintergrund, ein neuer Aufruf bricht den alten ab; je Bucket genügt ein Treffer (springt zum nächsten Bucket).
  Die UI fragt 300 ms nach Änderung von Revision oder `Frame.deco` (Hash von Suche und Markern, 0 = nichts).
  Abschaltbar (`overview`, Aktion „Treffer in der Scrollleiste zeigen“).
- **E4 Weicher Umbruch** in der Engine: `Configure { wrap }`; Zeilen umbrechen nach dem letzten Leerzeichen, das
  passt, sonst hart (nie in einem Graphem); Breite = Spalten der Ansicht − 1. Lange Zeilen (> 10 000 Zeichen)
  bleiben unumbrochen und scrollen seitlich (Hinweis in der Statusleiste). Scrollposition bleibt
  `top` = Zeile + Anteil, der Anteil zählt Zeilen-Reihen; Scrollen, Seite auf/ab, Klick und Ziehen rechnen in
  Reihen (`advance`, nie mehr Zeilen anschauen als Reihen bewegt werden), Hoch/Runter wandert reihenweise mit
  Ziel-x innerhalb der Reihe. Frame: `FrameLine.row` (Reihen ab Oberkante), `wraps` (Anfangszellen der
  Folgereihen; Runs werden dort geteilt), `caretXy` (Reihe + x je Cursor), `primary`. Die UI teilt Auswahl-,
  Treffer- und Markerbereiche an `wraps` (reine Geometrie). Scrollleiste bleibt zeilenbasiert (ungefähr).
  Messung (1 Mio. Zeilen à 237 Zeichen): Frame 0,2 ms, Sprung ans Ende 0,3 ms.
- **E5 Als HTML kopieren** (`CopyHtml`): die Engine baut `<pre>` mit Inline-Stilen aus den Farben, die die UI
  mitschickt (Proben-Spans je Stilklasse → `getComputedStyle`), optional mit Zeilennummern; Text als
  Alternative (`Clipboard::set_html`, arboard `set_html`). Höchstens 20 000 Zeilen (`html_too_large`).
  Strg+Umschalt+C; mit Zeilennummern über Palette/Menü (legacy Strg+H ist hier Ersetzen).

## Tests

- Rust: `wrap_points` (Leerzeichen, hart, breite Zeichen, kombinierende Zeichen), Reihen/Runs/Caret im Frame,
  Bewegen/Scrollen/Klicken in Reihen, lange Zeilen, Breite nach Resize; Zeilenenden inkl. CR-Sonderfälle und Undo;
  HTML (Escaping, Stile, Zeilennummern, Endzeile); `overview`; `load_file_as`; `ENCODINGS` schreibbar.
- Angular: Darstellung umbrochener Zeilen, Übersichtsstriche, Zeilenenden/HTML-Aktionen, Statusleiste,
  Kodierungswechsel im DocumentsStore.
- Ende-zu-Ende (Xvfb): Umbruch mit 5 000 Zeilen, Suche mit Strichen in der Scrollleiste, Kodierung neu lesen,
  CRLF umwandeln, HTML in der Zwischenablage (`xclip -t text/html`).
