# Prompt 003: Virtuelle Editor-Ansicht

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 002. Legacy: §4 (Editor-Verhalten).

## Ziel

Eine eigene Editor-Komponente, die **in beiden Richtungen virtualisiert**: nur sichtbare Zeilen und von jeder Zeile
nur das sichtbare Spaltenfenster werden gerendert. Monospace-Schrift (Zeichenbreite gemessen), eigene Scrollleisten
für Dokumente mit mehr Pixeln als der Browser erlaubt.

## Umfang

- `features/editor/`: `EditorView` (DOM-Zeilen, kein Framework-Overhead je Zeile), `EditorStore` (Caret, Auswahl,
  Scrollposition in Zeilen/Spalten, Viewport-Cache, veraltete Antworten verwerfen).
- Eingabe über verstecktes `textarea` (Tippen, IME-Komposition, Einfügen, Ausschneiden, Kopieren); Kopieren großer
  Auswahlen über das Backend.
- Navigation: Pfeile, Pos1/Ende (intelligent: erst Einrückung), Bild auf/ab, Strg+Pos1/Ende, Wortweise
  (Strg/Alt+Pfeil je OS), Auswahl mit Umschalt, Doppelklick = Wort, Dreifachklick = Zeile, Ziehen mit der Maus inkl.
  Auto-Scroll, Mausrad und Trackpad horizontal.
- Zeilennummern (absolut, legacy-Option relativ vorbereiten), aktuelle Zeile hervorgehoben, Tabs als Pfeile dezent.
- Bearbeiten: Tippen, Enter mit Auto-Einrückung, Backspace/Entf, Tab/Umschalt+Tab (Einrücken), Undo/Redo.

## Akzeptanz

- Datei mit 10 Mio. Zeilen: Springen ans Ende sofort, Scrollen flüssig. Zeile mit 10 Mio. Zeichen: Caret ans Ende,
  Tippen dort ohne merkliche Verzögerung.
- Komponenten-Tests mit `MemoryDocumentApi` (Tastatur, Auswahl, Eingabe); Ende-zu-Ende unter Xvfb.
