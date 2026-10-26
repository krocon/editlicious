# Design 007: Bearbeitungsbefehle

Status: umgesetzt · Datum: 26.10.2026

## Entscheidungen

- **E1 Zeilen sortieren** (legacy Strg+Umschalt+Alt+A/D, Todo-Liste): Auswahl-Zeilen, ohne Auswahl das ganze
  Dokument; natürlich (`a2 < a10`, Zahlen nach Wert, Groß/klein ignorierbar), Text, numerisch (erste Zahl der
  Zeile, Komma oder Punkt, Zeilen ohne Zahl ans Ende), absteigend, Duplikate entfernen; stabil; die leere Zeile nach
  einem abschließenden Zeilenumbruch bleibt am Ende. Ein Undo-Schritt. Kürzel: Mod+⇧+Alt+A/D, sonst Palette/Menü.
- **E2 Groß/klein** (legacy Strg+U = umschalten: groß, außer es ist schon groß), dazu groß, klein, Titel – je Auswahl
  oder Wort am Caret, Unicode-korrekt (ß → SS).
- **E3 Zeilen:** duplizieren (Mod+⇧+D), löschen (Mod+⇧+⌫), verschieben (Alt+↑/↓), verbinden (Mod+J, ein Leerzeichen,
  Einrückung der nächsten Zeile fällt weg), Leerraum am Zeilenende entfernen und Einrückung Tabs ↔ Leerzeichen
  (Palette/Menü).
- **E4 Blockauswahl** (legacy „Column Mode“): Alt+Ziehen bzw. Alt+⇧+Pfeile (macOS ⌘⌥⇧+Pfeile, weil ⌥⇧+Pfeil dort
  Wortauswahl ist); ein Cursor je Zeile zwischen zwei Zellen, kürzere Zeilen bekommen einen Cursor am Zeilenende
  (Breite 0 erlaubt – legacy-Todo), höchstens 10 000 Zeilen. Tippen, Löschen und Einfügen wirken auf jeder Zeile;
  enthält die Zwischenablage genau so viele Zeilen wie Cursor, bekommt jeder Cursor seine Zeile.
- **E5 Mehrere Cursor:** Alt+Klick fügt einen hinzu, Mod+D wählt das Wort und dann das nächste Vorkommen
  (Groß/klein genau, mit Umlauf); der zuletzt hinzugefügte Cursor ist der primäre (Scrollen, Status). Escape:
  erst Suche schließen, dann zurück zu einem Cursor (`edit.escape`).
- **E6 Marker** (legacy „Mark Text“ Strg+⇧+F7 / „Clear All Markers“ Strg+⇧+F8, „bis zu 10 Wörter farblich“):
  Wort am Caret (oder Auswahl) als ganzes Wort in der nächsten freien von zehn Farben; nochmals = entfernen; die
  ältesten fallen heraus, wenn alle belegt sind. Frame-Feld `markers`; Farben als Tokens `--ed-marker-0…9`.

## Tests

- Rust `ops` (natürlich, numerisch + eindeutig, Groß/klein) und Sitzung (Sortieren Auswahl/Dokument + Undo,
  Groß/klein, Duplizieren/Verschieben/Löschen/Verbinden, Leerraum/Einrückung, Blockauswahl tippen + Escape,
  Alt-Ziehen/Alt-Klick, nächstes Vorkommen, verteiltes Einfügen, Marker).
- Frontend: Alt-Klick/-Ziehen sendet `add`/`column`, Escape → `singleCursor`, Marker gezeichnet.
- Ende-zu-Ende (Xvfb): Sortieren per Kürzel, Blockauswahl über drei Zeilen und tippen, Marker.
