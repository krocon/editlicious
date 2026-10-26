# Parität mit dem Java-Editor (Prompt 013)

Jede Aktion der Vorlage (legacy §2, `ActionFactory`) mit ihrem Gegenstück in editlicious und dem Test, der es
belegt. ✅ = vorhanden, ◐ = anders gelöst, ✗ = fehlt (→ Folge-Ideen unten). Kürzel: `Ctrl` = `⌘` auf macOS.

| Vorlage | Kürzel alt | editlicious | Kürzel neu | Stand | Beleg (Test) |
|---|---|---|---|---|---|
| New… | Ctrl+N | `file.new` | Ctrl+N | ✅ | documents.store.spec „starts with an empty document“ |
| Open File… | Ctrl+O | `file.open` (nativer Dialog) | Ctrl+O | ✅ | documents.store.spec „opens command-line files …“, Rust `load` Tests |
| Save all | Ctrl+S | `file.saveAll`; Ctrl+S speichert die **aktuelle** Datei | Ctrl+Alt+S | ◐ | documents.store.spec „saves new documents through Save as …“ |
| Save as… | Ctrl+Alt+S | `file.saveAs` | Ctrl+Shift+S | ✅ | Rust `new_document_save_as`, `save_round_trips_bytes` |
| Close file / all | Ctrl+W, Ctrl+F4 / Ctrl+Shift+F4 | `file.close` / `file.closeAll` | Ctrl+W / Ctrl+Shift+W | ✅ | documents.store.spec „asks before closing unsaved changes“ |
| Exit | Alt+F4 | `app.quit` | Ctrl+Q | ✅ | documents.store.spec „quits after asking, keeping the session“ |
| Next / previous file | F10 / Shift+F10, Ctrl+Tab | `file.next` / `file.prev` | gleich | ✅ | keymap.spec „switches files with Ctrl+Tab … F10 like the old editor“ |
| Copy / Cut / Paste | Ctrl+C/X/V | `edit.copy/cut/paste` (Rust, auch große Auswahlen) | gleich | ✅ | Rust `copy_cut_paste`, `paste_one_line_per_cursor` |
| Copy as HTML (ohne/mit Zeilennummern) | Ctrl+H / Ctrl+Alt+H | `edit.copyHtml` / `edit.copyHtmlNumbers` | Ctrl+Shift+C / Palette | ✅ | Rust `copy_as_html_with_styles_and_line_numbers` |
| Undo / Redo (> 200 Schritte) | Ctrl+Z / Ctrl+Shift+Z | `edit.undo/redo` (10 000 Schritte, 256 MB) | gleich (+Ctrl+Y) | ✅ | Rust `shared_cases`, `save_marks_clean_and_undo_back_is_dirty` |
| Upper and Lowercase | Ctrl+U | `edit.toggleCase` (+ upper/lower/title) | Ctrl+U | ✅ | Rust `case_changes`, `case_of_selection_and_word` |
| Sort Lines Asc/Desc | Ctrl+Shift+Alt+A/D | `edit.sortAsc/Desc` (+ numerisch, ohne Duplikate) | gleich | ✅ | Rust `natural_order`, `numeric_descending_unique`, `sort_selection_or_document` |
| Auto Completion | Ctrl+Space | – | – | ✗ | – |
| Zen Coding | Ctrl+Alt+Z | – (bewusst nicht) | – | ✗ | – |
| Change Tab Length 2/4/6/8 | – | Einstellungen „Tabweite“ (2/3/4/6/8), „wie die Datei“ | Ctrl+, | ✅ | Rust `configure_detect_follows_the_file`; settings-dialog.spec |
| Mark Text / Clear All Markers (10 Farben) | Ctrl+Shift+F7 / F8 | `edit.toggleMarker` / `edit.clearMarkers` (10 Farben, in der Scrollleiste) | gleich | ✅ | Rust `markers_colour_words`; editor-view.spec „draws word markers …“ |
| Fold Selection / Unfold All | Ctrl+Shift+8 / Ctrl+Shift+Alt+F | – | – | ✗ | – |
| Toggle Word Wrap | Ctrl+Shift+W | `view.wordWrap` (Engine, Reihen) | Alt+Z | ◐ Kürzel | Rust `wrap_*`; editor-view.spec „draws wrapped lines …“ |
| Toggle Color Style | F12 | `view.nextScheme` (5 Schemata) | F12 | ✅ | chrome.store.spec „cycles the colour schemes with F12 (legacy)“ |
| Toggle Line Numbers | Ctrl+L | `view.lineNumbers` (absolut/relativ/aus) | Ctrl+Shift+L | ◐ Kürzel | settings.store.spec „line numbers cycle …“ |
| Toggle Indentation Lines | Ctrl+Alt+L | `view.indentGuides` | Ctrl+Alt+L | ✅ | Rust `frame_lines_carry_their_indentation`; editor-view.spec |
| Toggle Main Menu | Ctrl+M | `view.toggleMenuBar` (Alt zeigt sie einmal) | Ctrl+M | ✅ | menu-bar spec „shows the menu bar for one use or for good“ |
| Toggle Icon Bar | Ctrl+F6 | – (keine Icon-Leiste, „wenig Ablenkung“) | – | ◐ | – |
| Toggle Status Bar | Ctrl+F5 | `view.statusBar` | Ctrl+F5 | ✅ | status-bar.spec |
| Toggle Mini Map | Ctrl+Shift+S | Treffer/Marker in der Scrollleiste (`view.overview`) | Palette | ◐ | Rust `overview_marks_buckets_per_layer`; editor-view.spec „asks for the overview …“ |
| Toggle Menu, Tabs, Iconbar | F1 | `view.toggleChrome` | F1 | ✅ | app.spec / chrome store |
| Fade In&Out Menu, Tabs | Ctrl+Shift+F1 | – | – | ✗ | – |
| Toggle Window Mode | Ctrl+Shift+M | `view.fullscreen` | F11 / Ctrl+⌘+F | ✅ | (Rust `toggle_fullscreen`, E2E) |
| File Tabs zeigen/verstecken/auto | – | `view.tabs.show/hide/auto` | Menü | ✅ | documents.store.spec „switches and reorders tabs“ |
| Schrift größer/kleiner/zurück | Ctrl+Num+ / − / Ctrl+F9 | `view.zoomIn/Out/Reset` | Ctrl+Plus/Minus/0 | ✅ | settings.store.spec „zoom stays in range and resets“ |
| Settings… / Save Settings | Ctrl+Shift+O / Ctrl+Alt+S | `app.settings`, speichert selbst | Ctrl+, | ✅ | settings.store.spec „saves changes bundled …“ |
| Import Webmatrix Color Style | Ctrl+Shift+Alt+I | – | – | ✗ | – |
| Show Keystrokes… | Ctrl+Shift+K | `app.shortcuts` (+ Befehlspalette) | Ctrl+Shift+K | ✅ | app.spec „lists the keyboard shortcuts“ |
| About… / Update-Prüfung | Ctrl+F1 / Ctrl+F12 | `app.about`; Update-Prüfung offen (012) | Menü | ◐ | about dialog spec |
| Debug… | Ctrl+F2 | – (Entwicklerwerkzeug) | – | ◐ | – |

Dazu neu (nicht in der Vorlage): Suchen/Ersetzen mit Verlauf und Zählung, Gehe zu Zeile, mehrere Cursor,
Blockauswahl, Zeilen verschieben/duplizieren/verbinden, Leerraum-Befehle, Kodierungen lesen/umwandeln,
Zeilenenden umwandeln, Sitzung wiederherstellen, Änderungen von außen, Deutsch/Englisch, Befehlspalette.

## Folge-Ideen (Lücken)

1. **Code-Faltung** (Fold Selection / Unfold All) – in der Engine als ausgeblendete Zeilenbereiche; Frames
   überspringen sie. Mittel.
2. **Autovervollständigung** aus Wörtern des Dokuments (Ctrl+Space) – Wortindex der sichtbaren Umgebung. Klein.
3. **Ein- und Ausblenden mit Übergang** (Ctrl+Shift+F1) – reine Oberfläche. Klein.
4. **Farbschema importieren** (TextMate `.tmTheme` statt Webmatrix) → `--syn-*`-Tokens. Mittel.
5. **Update-Prüfung** (tauri-plugin-updater) – nach Signierung, s. `docs/release.md`.
