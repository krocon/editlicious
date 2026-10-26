# Design 006: Suchen, Ersetzen, Gehe zu

Status: umgesetzt · Datum: 26.10.2026

## Entscheidungen

- **E1 Suche in Rust in Blöcken ganzer Zeilen** (`ed_core::search`, ≤ 1 MiB, eine längere Zeile ist ein eigener
  Block): große Dateien werden nie ein String; `find_at` mit Zeilenanfang als Kontext gibt `^`, `$`, `\b` ihre
  Bedeutung auch mitten in der Zeile. Treffer sind Zeichenbereiche; Treffer über Blockgrenzen (Muster mit `\n`)
  werden in der Regel gefunden, solange sie innerhalb eines Blocks liegen.
- **E2 `regex`-Crate** (lineare Laufzeit, kein Backtracking), Größenlimit 8 MiB für das kompilierte Muster;
  Optionen Groß/klein, ganzes Wort (`\b` nur an Wortzeichen-Rändern des Literals), Regex. Ungültiges Muster →
  `invalid_search`, Feld rot.
- **E3 Engine-Befehle** `find { query, backwards, incremental }` (inkrementell = ab Auswahlbeginn, beim Tippen),
  `clearSearch`, `replace` (ersetzt die Auswahl nur, wenn sie genau ein Treffer ist, dann nächster Treffer),
  `replaceAll` (ein Undo-Schritt; leere Treffer zählen: `^` → Zeilen voranstellen). Ersetzung im Regex-Modus mit
  `$1`/`${name}` und `\n`, `\t`, `\\`. Treffer in den sichtbaren Zeilen stehen im Frame (`marks`, der ausgewählte
  hervorgehoben); ein Sprung außerhalb der Ansicht setzt die Zeile in die Mitte.
- **E4 Zählen auf einem Schnappschuss** (`count_matches`, Rope-Klon O(1)) außerhalb der Sitzungssperre: Tippen
  bleibt flüssig, ein neues Zählen bricht das alte ab; Anzeige „3 of 13“.
- **E5 Ruhige Suchleiste** oben rechts im Editor (nur sichtbar, wenn gebraucht): Mod+F (Auswahl übernehmen),
  Mod+H bzw. ⌘⌥F Ersetzen, F3/⇧F3 bzw. ⌘G/⌘⇧G weiter/zurück, Enter/⇧Enter im Feld, Alt+C/W/R Optionen, Escape
  schließt und löscht die Markierungen (auch im Editor). Strg/⌘+Enter im Ersetzen-Feld = alle ersetzen.
- **E6 Verlauf** (je 20) für Suchen und Ersetzen in `session.json`, ↑/↓ im Feld (legacy-Todo).
- **E7 Gehe zu Zeile[:Spalte]** (Mod+G bzw. ⌘L) als kleines Feld oben mittig; 1-basiert, Spalte = Zeichen.

## Messungen (Release, 100 MB / 1,4 Mio. Zeilen)

Treffer am Dateiende finden 30 ms · 1,4 Mio. Treffer zählen 0,44 s · 1,4 Mio. Ersetzungen sammeln 0,7 s.

## Tests

- Rust: `search` (Literal/Groß-klein/Wort, Fehler, rückwärts mit Umlauf, Anker mitten in der Zeile, Unicode,
  große Texte über Blockgrenzen + Zählen, Ersetzungsgruppen, leere Treffer, Abbruch), Sitzung (Finden +
  Markierungen, kein Treffer, ungültig, Ersetzen einzeln/alle + ein Undo, Zentrieren).
- Frontend: `find.spec` (Gehe-zu-Parser, Verlauf, Suchleiste: Tippen/Enter/Escape/Optionen),
  `editor-view.spec` (Mod+F übernimmt Auswahl, sucht, zählt, Escape löscht; Gehe zu; Markierungen).
- Ende-zu-Ende (Debug-Build, 140 MB): Mod+F, Tippen sucht inkrementell, Zähler „3 of 13“, Treffer zentriert und
  hervorgehoben, Mod+H öffnet Ersetzen.
