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
018 (Farbfelder), 019 (Marker/HTML) und 020 (Rechnen, Tooltips, Vorschläge) sind unabhängig. Aufzeichnen/KI/Video: 015 → 016 → 017 (017 braucht 016 nicht).

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
| 008 | [008-syntax-farbschemata.md](done/008-syntax-farbschemata.md) | Syntax-Hervorhebung nur für den Viewport (inkrementelle Zustände), Sprachen nach Endung, Farbschemata hell/dunkel umschaltbar (F12 wie legacy) | 003 | M–L | ✅ 26.10.2026 |
| 009 | [009-einstellungen-i18n.md](done/009-einstellungen-i18n.md) | Einstellungen (Schrift, Größe/Zoom, Tabweite, Zeilennummern-Stil, Einrückungslinien, Umbruch, Autosave-Fragen, letzte Dateien) + Dialog, Deutsch/Englisch zur Laufzeit | 005 | M | ✅ 26.10.2026 |
| 010 | [010-statusleiste-extras.md](done/010-statusleiste-extras.md) | Statusleiste (Zeile/Spalte, Auswahl, Encoding, Zeilenende, Größe) mit Umschalten von Encoding/Zeilenende, Kopieren als HTML, Übersichtsleiste (Mini-Map light), Weicher Umbruch für normale Zeilen | 004, 008 | M | ✅ 26.10.2026 |
| 011 | [011-webseite.md](done/011-webseite.md) | Werbe-Webseite `/site` (statisch, light/dark, Screenshots, Features, Tastenkürzel, Download, Live-Demo mit `ed-core` als WebAssembly) | 004 | M | ✅ 26.10.2026 |
| 012 | [012-release-distribution.md](done/012-release-distribution.md) | Version, Release-Notes, Lizenzhinweise, Pakete je OS, CI- und Release-Workflows | 001 | M | ✅ 26.10.2026 |
| 013 | [013-performance-haertung.md](done/013-performance-haertung.md) | Messungen mit 1 GB / 10 Mio. Zeilen / 100-MB-Zeile, Engpässe beheben, Review durch separaten Agenten, Feature-Parität gegen legacy | 002–010 | M | ✅ 26.10.2026 |
| 014 | [014-release-pipeline.md](done/014-release-pipeline.md) | Release-Pipeline wie Comic Commander / Files and Folders: Probelauf ohne Release, Vorabprüfung, automatische Release Notes, Paketprüfung, Prüfsummen, Lizenzbericht | 012 | M | ✅ 26.10.2026 |
| 015 | [015-aufzeichnen-abspielen.md](done/015-aufzeichnen-abspielen.md) | Befehle mit Zeitstempeln aufzeichnen, abspielen (Original/Faktor/menschlich mit Seed), Tipp-Skripte `*.edscript` für Videos, Export Aufnahme → Skript, Makro wiederholen | 005, 007, 013 | M–L | ✅ 26.10.2026 |
| 016 | [016-mcp-server.md](016-mcp-server.md) | MCP-Server (Sidecar `editlicious-mcp`, stdio ↔ lokaler Socket): KI öffnet, liest bereichsweise, tippt im Video-Tempo, führt Aktionen aus, nimmt auf; standardmäßig aus, Token, Revisionen | 015 | M–L | offen |
| 017 | [017-video-export.md](017-video-export.md) | Aufzeichnung/Skript ohne Bildschirmaufnahme als Video rendern (virtuelle Uhr, PNG-Sequenz, MP4 über externes ffmpeg), CLI `editlicious render` für die Video-Pipeline | 015 | L | offen |
| 018 | [018-farbfelder-im-rand.md](018-farbfelder-im-rand.md) | Farbwerte (CSS `#…`/`rgb()`/`hsl()`, `0xAARRGGBB`, `"ffRRGGBB"`) als Farbfelder im linken Rand (legacy), Erkennung in Rust nur für den Frame, Farbwert umrechnen | 008, 009, 010 | S–M | offen |
| 019 | [019-marker-ausbau-html.md](019-marker-ausbau-html.md) | Wort-Marker im „Als HTML kopieren“ (deckende Farben für E-Mail), Auswahl markieren, feste Farbe 1…0, nächstes/voriges Vorkommen, Anzahl in der Statusleiste, Marker in der Sitzung | 007, 010 | S–M | offen |
| 020 | [020-rechnen-tooltips-autocomplete.md](020-rechnen-tooltips-autocomplete.md) | Math-Parser in Rust (Vorlage `java-k-editor/MathParser`), Ergebnis als Hinweis am Caret und als Vorschlag (`255/8=` → `31.875`); allgemeine Bausteine Tooltip- und Vorschlags-Anbieter | 005, 009, 010 | M–L | offen |

## Folge-Ideen (noch ohne Prompt)

- Mehrere Cursor, Code-Faltung (legacy: Fold Selection / Unfold All), Zen Coding / Emmet, Autovervollständigung (Wörter; Grundlage aus 020).
- Log-Modus (legacy `Todos.java`): Filter nach Level, Auto-Reload, „nur zeigen/ignorieren“ mit Regex.
- Textvergleich (Diff) zweier Dateien.
- Hex-Ansicht für Binärdateien.
- Numerische Integration aus der MathParser-Vorlage (`algo/numerical`), Farbwähler für Farbfelder (018).
