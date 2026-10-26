# Prompt 006: Suchen, Ersetzen, Gehe zu

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 003, 005.

## Umfang

- Schlanke Suchleiste oben im Editor (Mod+F, Mod+H/Mod+Alt+F für Ersetzen), Optionen Groß/klein, ganzes Wort, Regex.
- Suche in Rust auf dem Rope (`regex`), vom Caret aus, mit Umlauf; Zählen aller Treffer gestreamt und abbrechbar;
  Treffer im Viewport markiert (nur Viewport anfragen).
- Ersetzen, Alle ersetzen (ein Undo-Schritt), Regex-Gruppen `$1`; Suchverlauf (legacy Todo: Paare Suchen/Ersetzen).
- Gehe zu Zeile[:Spalte] (Mod+G / Mod+L je OS).
