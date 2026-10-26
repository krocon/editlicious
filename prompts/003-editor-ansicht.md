# Prompt 003: Editor-Engine in Rust und Render-Ansicht

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 002. Legacy: §4 (Editor-Verhalten).
> Grundsatz (Marc, 26.10.2026): **alle Aktionen in Rust, die UI rendert nur** (Design 002 E13).

## Ziel

Eine **Editor-Sitzung** in `ed-core` (Dokument + Carets/Auswahlen + Scrollstand + Viewport + Tabweite), die Befehle
ausführt und **Frames** liefert; eine Angular-Komponente, die Eingaben in Befehle übersetzt und Frames zeichnet –
in beiden Richtungen virtualisiert (nur sichtbare Zeilen, bei Langzeilen nur das sichtbare Spaltenfenster).

## Umfang

- **Rust `ed-core/editor/`**: `Session` mit Befehlen (Bewegen: Zeichen/Graphem, Wort, Zeilenanfang intelligent,
  Zeilenende, Seite, Dokumentanfang/-ende, jeweils mit Auswahl; Klicken/Ziehen/Doppel-/Dreifachklick in Zellen;
  Text einfügen, Enter mit Auto-Einrückung, Backspace/Entf (auch wortweise), Tab/Umschalt+Tab, Alles auswählen,
  Undo/Redo; Scrollen, Größe; Kopieren/Ausschneiden/Einfügen über die System-Zwischenablage im Backend).
- **Layout in Zellen**: Tabs auf Tabstopps, breite Zeichen (East Asian Wide) = 2 Zellen, kombinierende Zeichen = 0;
  Langzeilen (> 10 000 Zeichen) = 1 Zelle je Zeichen (002 E3). Frame = sichtbare Zeilen als Läufe mit Zellposition,
  Carets und Auswahlbereiche je Zeile in Zellen, Scrollstand, Dokumentinfo.
- **Frontend `features/editor/`**: Fassade `EditorApi` (+ Fake), `EditorView` (eigene, dezente Scrollleisten;
  Zeichenbreite gemessen; Läufe absolut nach Zellen positioniert), verstecktes `textarea` für Eingabe und IME
  (Komposition lokal, Abschluss als Befehl), Befehle serialisiert, veraltete Frames verworfen.
- Zeilennummern, aktuelle Zeile, Tabs als dezente Pfeile, sichtbarer Caret (blinkend, im Fokus).

## Akzeptanz

- Datei mit 10 Mio. Zeilen: Springen ans Ende sofort, Scrollen flüssig. Zeile mit 10 Mio. Zeichen: Caret ans Ende,
  Tippen dort ohne merkliche Verzögerung.
- Rust-Tests für alle Befehle (Unicode, Tabs, CRLF, Langzeilen); Komponenten-Tests mit Fake-Frames; Ende-zu-Ende
  unter Xvfb (Tippen, Navigieren, Auswahl mit Maus).
