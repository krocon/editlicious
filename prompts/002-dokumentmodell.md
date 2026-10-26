# Prompt 002: Dokumentmodell in Rust

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 001. Legacy: §3 (Dateien), §5 (Optionen).

## Ziel

Das Herz des Editors: Dokumente leben **im Rust-Backend** (Rope), das Frontend fragt nur den sichtbaren Ausschnitt ab
und schickt Bearbeitungen. So bleibt die IPC-Last unabhängig von der Dateigröße.

## Umfang

- `document/`: `Document` auf Basis eines Ropes (z. B. `ropey`), Registry offener Dokumente mit Ids.
- **Öffnen**: Datei streamen, Encoding erkennen (BOM, UTF-8-Prüfung, sonst Erkennung z. B. `chardetng`, Dekodieren
  mit `encoding_rs`), Zeilenende erkennen (LF/CRLF/CR/gemischt), Binärdateien erkennen (NUL-Bytes) und ablehnen.
  Fortschritt bei großen Dateien über einen Channel, abbrechbar.
- **Viewport**: `get_lines(doc, firstLine, count, colStart, colCount)` liefert nur das Fenster der Zeilen (visuelle
  Spalten mit Tabweite und breiten Zeichen); Zeilenlänge (visuell) mitliefern; für sehr lange Zeilen ein Index
  visueller Spalten (Checkpoints), damit Spalte ↔ Zeichen nicht jedes Mal von vorne gezählt wird.
- **Bearbeiten**: Liste von Änderungen (Bereich → Text) atomar anwenden, Undo/Redo mit Gruppierung (Tippen in einem
  Wort = ein Schritt), Revisionszähler, „dirty“-Status.
- **Speichern**: im ursprünglichen Encoding/Zeilenende/BOM, atomar (`fs_atomic`), Speichern unter.
- TS-Fassade `DocumentApi` + DTOs; **`MemoryDocumentApi`** (reines TS, gleiche Semantik) für Tests, den Browser-Modus
  und die Demo auf der Webseite.

## Akzeptanz

- Rust-Tests: Unicode (Emoji, CJK, kombinierende Zeichen), alle Zeilenenden, BOMs, Latin-1/Windows-1252,
  leere Datei, Datei ohne abschließendes Zeilenende, Binärdatei, 100-MB-Datei (ignored-Test mit Messung),
  Zeile mit 10 Mio. Zeichen (Fenster mitten in der Zeile < 5 ms).
- Gleiche Fälle als Vertragstest gegen `MemoryDocumentApi`.
