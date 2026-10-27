# editlicious – Texte für Einträge

> Quelle aller Aussagen: https://krocon.de/products/editlicious/, `README.md` und Release v1.0.0 (`facts.json`).
> Keine Aussage hinzufügen, die dort nicht belegt ist. Englisch für internationale Portale, Deutsch für deutsche
> (CHIP, heise download, Softpedia DE …). Der Name wird **immer klein** geschrieben: editlicious.

## Name

editlicious

## Tagline (≤ 60 Zeichen)

- EN: `A calm, keyboard-first text editor for huge files` (49)
- DE: `Ruhiger Texteditor für riesige Dateien – per Tastatur` (53)

## Kurzbeschreibung (≤ 160 Zeichen)

- EN: `Free, open-source text editor for macOS, Windows and Linux that opens and edits huge files – millions of lines, lines with millions of characters.` (146)
- DE: `Kostenloser Open-Source-Texteditor für macOS, Windows und Linux, der riesige Dateien öffnet und bearbeitet – Millionen Zeilen, Zeilen mit Millionen Zeichen.` (156)

## Paketmanager-Beschreibung (eine Zeile, ohne Produktname am Anfang, ≤ 80 Zeichen)

`Calm, keyboard-first text editor for huge files` (47)

## Lange Beschreibung – Englisch

editlicious is a calm, keyboard-first text editor for macOS, Windows and Linux that opens and edits huge files –
millions of lines, lines with millions of characters, files up to 4 GB. It is free and open source (MIT).

- **Huge files first** – the text lives in a rope inside a Rust engine; the window only ever renders what you see,
  vertically and horizontally. Log files, data dumps and minified JSON open without waiting for the whole file to be
  drawn.
- **Distraction-free** – no toolbars, no side panels. The menu bar appears on Alt, tabs only with a second file, F1
  hides everything but the text. Every function is a command in the command palette with a shortcut.
- **Editing** – several carets, column selection, selections that grow with every double click, find and replace
  with regular expressions, sort lines, ten word markers that jump, count and stay in “Copy as HTML”.
- **Your encodings, your line endings** – detected on load, kept on save; saving is atomic and keeps permissions and
  symlinks.
- **Looks** – syntax colours for more than 70 languages (Sublime/TextMate grammars, log files and INI included),
  thirteen colour schemes, any monospaced font, soft wrap, colour values as swatches in the gutter.
- **More** – record and play back macros and typing scripts and render them to video (`editlicious render`), and an
  opt-in AI control via the Model Context Protocol (off by default, local only).

No telemetry, no accounts, no update pings. It is not an IDE: no project view, no language server, no plugin store.
English and German user interface. Try the real editing engine in the browser: https://krocon.de/products/editlicious/demo/

Note: the packages are not code-signed yet. macOS: right-click → Open on first start. Windows: SmartScreen → More info
→ Run anyway.

## Lange Beschreibung – Deutsch

editlicious ist ein ruhiger, tastaturgesteuerter Texteditor für macOS, Windows und Linux, der riesige Dateien öffnet
und bearbeitet – Millionen Zeilen, Zeilen mit Millionen Zeichen, Dateien bis 4 GB. Kostenlos und Open Source (MIT).

- **Große Dateien zuerst** – der Text liegt als Rope in einer Rust-Engine; das Fenster zeichnet nur, was man sieht,
  senkrecht und waagerecht. Logdateien, Datenexporte und minifiziertes JSON öffnen ohne Warten.
- **Ablenkungsfrei** – keine Werkzeugleisten, keine Seitenleisten. Die Menüleiste erscheint mit Alt, Tabs erst ab der
  zweiten Datei, F1 blendet alles außer dem Text aus. Jede Funktion ist ein Befehl in der Befehlspalette mit Kürzel.
- **Bearbeiten** – mehrere Cursor, Blockauswahl, eine Auswahl, die mit jedem Doppelklick wächst, Suchen und Ersetzen
  mit regulären Ausdrücken, Zeilen sortieren, zehn Wort-Marker, die springen, zählen und in „Als HTML kopieren“
  erhalten bleiben.
- **Deine Encodings, deine Zeilenenden** – beim Laden erkannt, beim Speichern beibehalten; Speichern ist atomar und
  erhält Rechte und Symlinks.
- **Aussehen** – Syntaxfarben für mehr als 70 Sprachen (Sublime/TextMate-Grammatiken, auch Logdateien und INI),
  dreizehn Farbschemata, jede Monospace-Schrift, weicher Umbruch, Farbwerte als Farbfelder im Rand.
- **Außerdem** – Makros und Tipp-Skripte aufzeichnen, abspielen und als Video rendern (`editlicious render`) sowie
  eine abschaltbare KI-Steuerung über das Model Context Protocol (standardmäßig aus, nur lokal).

Keine Telemetrie, keine Konten, keine Update-Abfragen. Keine IDE: keine Projektansicht, kein Language Server, kein
Plugin-Store. Oberfläche Deutsch und Englisch. Die echte Engine im Browser ausprobieren:
https://krocon.de/products/editlicious/demo/

Hinweis: Die Pakete sind noch nicht signiert (macOS: Rechtsklick → Öffnen; Windows: SmartScreen → Weitere
Informationen → Trotzdem ausführen).

## Leistung (nur mit Rahmen zitieren)

Gemessen mit Release-Builds der Engine auf einer kleinen Linux-Maschine mit zwei Kernen (`docs/architecture.md`):
1 GB / 10 Mio. Zeilen öffnen 1,6 s · erste → letzte Zeile 5 ms · 100-MB-Zeile öffnen 0,1 s · 10 000 Treffer in der
1-GB-Datei ersetzen 0,4 s. Nicht als „schnellster Editor“ o. ä. zuspitzen.

## Kategorien

- Allgemein: Developer Tools / Text Editors · Utilities
- AlternativeTo: die Kategorie, unter der vergleichbare Texteditoren stehen (dort nachsehen, nicht raten)
- Linux (Desktop-Datei im .deb v1.0.0): `Development;` – `Development;TextEditor;Utility;` wäre passender
  (`src-tauri/linux/editlicious.desktop`, nächstes Release)
- AppStream: `Development`, `TextEditor`, `Utility`

## Tags

`text-editor`, `editor`, `large-files`, `huge-files`, `log-viewer`, `logs`, `keyboard-first`, `distraction-free`,
`multiple-cursors`, `syntax-highlighting`, `encoding`, `rope`, `mcp`, `tauri`, `rust`, `webassembly`, `open-source`,
`cross-platform`, `macos`, `windows`, `linux`

## Vergleichbare Programme (für „Alternative to“-Angaben)

Nur verwenden, wenn die Plattform selbst diese Programme als Texteditor bzw. Viewer für große Dateien führt:
EmEditor, UltraEdit, Large Text File Viewer, klogg, glogg, BBEdit, CotEditor, Notepad++, Sublime Text, TextEdit/
Notepad. Keine Aussagen über Vorteile gegenüber diesen Programmen, keine Vergleichszahlen.

## Screenshots

1. https://krocon.de/products/editlicious/img/main.png – A Rust file with syntax colours, the find bar and marks in the scroll bar
2. https://krocon.de/products/editlicious/img/big-log.png – A log file with 1,000,001 lines, scrolled to line 500,000
3. https://krocon.de/products/editlicious/img/wrap.png – Soft wrap
4. https://krocon.de/products/editlicious/img/palette.png – Command palette
5. https://krocon.de/products/editlicious/img/settings.png – Settings

Ersatz, falls krocon.de nicht lädt: `https://raw.githubusercontent.com/krocon/editlicious/main/site/img/<name>.png`.
Icon: https://raw.githubusercontent.com/krocon/editlicious/main/src-tauri/icons/icon.png

## Show HN / Product Hunt – Entwurf (nur nach Freigabe posten)

**Show HN: editlicious – a calm text editor for huge files (Rust engine, Tauri)**

> I rebuilt my old Java/Swing editor from 2012 as a desktop app with Tauri 2, Rust and Angular. It opens and edits
> huge files – millions of lines, lines with millions of characters, up to 4 GB. The text lives in a rope in a Rust
> engine without any UI code; the window only gets the visible lines and columns, so nothing proportional to the file
> size crosses the bridge. Because the engine has no UI, it also runs as WebAssembly – there is a live demo that loads
> a million lines or a five-million-character line in your browser tab:
> https://krocon.de/products/editlicious/demo/
> Keyboard-first and quiet (menu on Alt, F1 hides everything), encodings and line endings kept on save, MIT licensed,
> builds for macOS, Windows and Linux (not code-signed yet). Feedback welcome.
> https://github.com/krocon/editlicious

(Der Text ist in Marcs Namen geschrieben: vor dem Posten von Marc prüfen lassen.)

## This Week in Rust – Entwurf (nur nach Freigabe)

Abschnitt „Project/Tooling Updates“ verlinkt Artikel, keine Repos. Ohne Blogartikel/Release-Ankündigung von Marc →
`WARTET_AUF_MARC`. Zeilenvorschlag, sobald es einen gibt: `[editlicious 1.0: a text editor for huge files with a Rust rope engine](<Artikel-URL>)`.
