# Prompt 004: Dateien und Tabs

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 003. Legacy: §3 (FileManager), §5 (Optionen).

## Umfang

- Mehrere offene Dokumente, Tabs (legacy: sichtbar / versteckt / auto = nur bei > 1 Datei), Tabs ziehen zum Umsortieren,
  nächste/vorige Datei (Strg+Tab, F10), Ungespeichert-Punkt.
- Neu, Öffnen (nativer Dialog über Rust), Speichern, Alle speichern, Speichern unter, Schließen, Alle schließen;
  Abfrage bei ungespeicherten Änderungen (Speichern / Verwerfen / Abbrechen), auch beim Beenden.
- Sitzung: zuletzt offene Dateien mit Caret/Scrollposition wiederherstellen (legacy `loadLastFiles`), Liste zuletzt
  geöffneter Dateien.
- Dateien per Drag & Drop öffnen; Datei als Argument beim Start (Dateizuordnung vorbereiten).
- Änderungen von außen erkennen (mtime/Größe beim Fokus) → neu laden anbieten bzw. automatisch, wenn unverändert
  (legacy `autoReloadExternalFileChanges`).
