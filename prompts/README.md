# Feature-Prompts – editlicious (Tauri)

Arbeitsaufträge für den Aufbau von **editlicious**, einem ablenkungsarmen Texteditor für sehr große Dateien
(viele Zeilen, extrem lange Zeilen). Auftrag und Rahmen: [000-start.md](000-start.md). Gemeinsame Regeln:
[000-arbeitsweise.md](000-arbeitsweise.md).

**Architektur-Grundsatz (Marc, 26.10.2026):** alle Aktionen laufen in Rust (`ed-core`), die Angular-UI rendert nur
Frames und übersetzt Eingaben in Befehle. WebAssembly nur für die Demo auf der Webseite.

**Arbeitsweise in diesem Repo (Marc, 26.10.2026):** Claude arbeitet die Prompts **selbständig nacheinander** ab
(keine Freigabe nach dem Design), committet und pusht nach jedem Prompt auf **`main`** und verschiebt den
fertigen Prompt samt Design-Dokument nach [`done/`](done/).

Herkunft: fachliche Vorlage ist der alte Java/Swing-Editor `java-k-editor/Editor` (Analyse in
`docs/legacy-analysis.md`, „legacy §N“); Architektur und übernehmbarer Code aus **comic-commander-tauri** und
**files-and-folders-tauri** (gleicher Autor, MIT).

## Reihenfolge

Kritischer Pfad zum ersten nutzbaren Editor: **001 → 002 → 003 → 004**. Danach 005–010 in dieser Reihenfolge,
011 (Webseite) und 012 (Release) dürfen jederzeit danach kommen, 013 schließt ab.

| Nr. | Prompt | Inhalt | Braucht | Umfang | Status |
|---|---|---|---|---|---|
| 000 | [000-arbeitsweise.md](000-arbeitsweise.md) | gemeinsamer Ablauf, Phasen, Regeln | – | – | – |
| 001 | [001-grundgeruest.md](done/001-grundgeruest.md) | Walking Skeleton: Tauri 2 + Angular 22 zoneless + Rust, Fassade `call.ts`, Fehler, About, Icon, Checks, CI, Doku, Legacy-Analyse | – | M | ✅ 26.10.2026 |
| 002 | [002-dokumentmodell.md](done/002-dokumentmodell.md) | Rust-Crate `ed-core`: Dokument als Rope, Öffnen mit Encoding-/BOM-/Zeilenende-Erkennung, Viewport-Abfrage mit Spaltenfenster, Bearbeiten, Undo/Redo, atomares Speichern | 001 | L | ✅ 26.10.2026 |
| 003 | [003-editor-ansicht.md](done/003-editor-ansicht.md) | Editor-Engine in Rust (Sitzung: Cursor, Auswahl, Befehle, Layout in Zellen, Zwischenablage, Frames) + Angular-Render-Ansicht in beide Richtungen virtualisiert, IME | 002 | L | ✅ 26.10.2026 |
| 004 | [004-dateien-tabs.md](done/004-dateien-tabs.md) | Mehrere Dateien: Tabs (auto/aus), Neu/Öffnen/Speichern/Speichern unter/Schließen, Ungespeichert-Abfrage, Sitzung wiederherstellen, Drag & Drop, Änderungen von außen | 003 | M | ✅ 26.10.2026 |
| 005 | [005-aktionen-menue-tastatur.md](done/005-aktionen-menue-tastatur.md) | Aktionskatalog, Keymaps je OS, natives/HTML-Menü, Befehlspalette, Ablenkungsfrei-Modus (Menü/Tabs/Status ein-/ausblenden), Vollbild, About, Tastenkürzel-Übersicht | 004 | M | ✅ 26.10.2026 |
| 006 | [006-suchen-ersetzen.md](done/006-suchen-ersetzen.md) | Suchleiste, Regex/Groß-klein/Ganzes Wort, Suche in Rust (gestreamt, abbrechbar), Treffer markieren, Ersetzen/Alle ersetzen, Verlauf, Gehe zu Zeile | 003, 005 | M | ✅ 26.10.2026 |
| 007 | [007-bearbeiten-befehle.md](done/007-bearbeiten-befehle.md) | Zeilen sortieren (auf/ab, natürlich/numerisch, Duplikate entfernen), Groß/klein, Zeile duplizieren/verschieben/löschen, Ein-/Ausrücken, Blockauswahl, Markierungen (Marker) | 003, 005 | M | ✅ 26.10.2026 |
| 008 | [008-syntax-farbschemata.md](008-syntax-farbschemata.md) | Syntax-Hervorhebung nur für den Viewport (inkrementelle Zustände), Sprachen nach Endung, Farbschemata hell/dunkel umschaltbar (F12 wie legacy) | 003 | M–L | offen |
| 009 | [009-einstellungen-i18n.md](009-einstellungen-i18n.md) | Einstellungen (Schrift, Größe/Zoom, Tabweite, Zeilennummern-Stil, Einrückungslinien, Umbruch, Autosave-Fragen, letzte Dateien) + Dialog, Deutsch/Englisch zur Laufzeit | 005 | M | offen |
| 010 | [010-statusleiste-extras.md](010-statusleiste-extras.md) | Statusleiste (Zeile/Spalte, Auswahl, Encoding, Zeilenende, Größe) mit Umschalten von Encoding/Zeilenende, Kopieren als HTML, Übersichtsleiste (Mini-Map light), Weicher Umbruch für normale Zeilen | 004, 008 | M | offen |
| 011 | [011-webseite.md](011-webseite.md) | Werbe-Webseite `/site` (statisch, light/dark, Screenshots, Features, Tastenkürzel, Download, Live-Demo mit `ed-core` als WebAssembly) | 004 | M | offen |
| 012 | [012-release-distribution.md](012-release-distribution.md) | Version, Release-Notes, Lizenzhinweise, Pakete je OS, CI- und Release-Workflows | 001 | M | offen |
| 013 | [013-performance-haertung.md](013-performance-haertung.md) | Messungen mit 1 GB / 10 Mio. Zeilen / 100-MB-Zeile, Engpässe beheben, Review durch separaten Agenten, Feature-Parität gegen legacy | 002–010 | M | offen |

## Folge-Ideen (noch ohne Prompt)

- Mehrere Cursor, Code-Faltung (legacy: Fold Selection / Unfold All), Zen Coding / Emmet, Autovervollständigung.
- Log-Modus (legacy `Todos.java`): Filter nach Level, Auto-Reload, „nur zeigen/ignorieren“ mit Regex.
- Textvergleich (Diff) zweier Dateien.
- Hex-Ansicht für Binärdateien.
