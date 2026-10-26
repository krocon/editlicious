# Legacy-Analyse: editlicious (Java/Swing)

> Fachliche Vorlage („legacy §N“) für die Neuauflage. Quelle: `~/IdeaProjects/java-k-editor/Editor` auf Marcs Mac
> (Paket `com.krocon.editlicious`, ca. 10 000 Zeilen eigener Code + Mozilla `universalchardet`). Stand der Analyse:
> 26.10.2026. Die alte Webseite (http://editlicio.us/, online nicht erreichbar) liegt lokal unter
> `~/IdeaProjects/java-k-editor/Web` – Auswertung in §10.
>
> Der eigentliche Text-Kern (`EditComponent`, `TextModel`, `Grammar`, `StyleManager`) lag in einem separaten Modul,
> das nicht im Ordner ist – nur seine Tests (`TextModelTest`, `TextModel2Test`) sind vorhanden.

## §1 Produktidee

- Schlanker, **tastaturgesteuerter** Editor mit sehr wenig sichtbarer Oberfläche: Menü, Icon-Leiste, Tabs und
  Statusleiste lassen sich einzeln und gemeinsam ausblenden (F1) oder ein-/ausfaden.
- Logo: Schreibschrift-Schriftzug „Editlicious“ (Metroscript-artig) und ein Füllfederhalter – die Neuauflage nimmt
  beides auf (Schriftzug aus der OFL-Schrift *Yellowtail* als Pfad, App-Icon: goldene Feder, die eine Linie schreibt).
- Notizen in `Todos.java` zeigen das Vorbild: Sublime Text, TextMate, BBEdit, Notepad++ (Funktionen wie „Save Copy
  As“, Suche/Ersetzen-Verlauf, Zeilenende-Anzeige).

## §2 Aktionen und Tastenkürzel (ActionFactory)

| Aktion (ID) | Name | Kürzel (Windows/Linux-Stil, Swing) |
|---|---|---|
| NewFileAction | New… | Ctrl+N |
| OpenFileAction | Open File… | Ctrl+O, Ctrl+Shift+F12 |
| SaveAllAction | Save all | Ctrl+S (speichert **alle**) |
| SaveAsAction | Save as… | Ctrl+Alt+S |
| CloseFileAction | Close file | Ctrl+F4, Ctrl+W |
| CloseAllFilesAction | Close all files | Ctrl+Shift+F4 |
| ExitAction | Exit | Alt+F4 |
| NextFileAction / PreviousFileAction | Next/Prev File | F10 / Shift+F10, Ctrl+Tab / Ctrl+Shift+Tab |
| CopyAction / CutAction / PasteAction | Copy/Cut/Paste | Ctrl+C / Ctrl+X / Ctrl+V |
| CopyHtmlWithoutRowNumbersAction | Copy as HTML | Ctrl+H |
| CopyHtmlWithRowNumbersAction | Copy as HTML with row numbers | Ctrl+Alt+H |
| UndoAction / RedoAction | Undo/Redo | Ctrl+Z / Ctrl+Shift+Z |
| UpperAndLowerCaseAction | Upper and Lowercase | Ctrl+U |
| SortTextAscendingAction / …Descending | Sort Lines Asc/Desc | Ctrl+Shift+Alt+A / Ctrl+Shift+Alt+D |
| AutoCompleteAction | Auto Completion | Ctrl+Space |
| ZenCodingAction | Zen Coding | Ctrl+Alt+Z |
| ChangeTabLengthAction(2/4/6/8) | Change Tab Length to n | – |
| MarkerAction / MarkerClearAllAction | Mark Text / Clear All Markers | Ctrl+Shift+F7 / Ctrl+Shift+F8 |
| FoldSelectionAction / UnfoldAllAction | Fold Selection / Unfold All | Ctrl+Shift+8 / Ctrl+Shift+Alt+F |
| ToggleWordWrapAction | Toggle Word Wrap | Ctrl+Shift+W |
| ToggleColorStyleAction | Toggle Color Style | F12 (F11) |
| ToggleLineNumbersAction | Toggle Line Numbers | Ctrl+L |
| ToggleIntendationLinesAction | Toggle Indentation Lines | Ctrl+Alt+L |
| ToggleMainMenueAction / ToggleIconBarAction / ToggleStatusBarAction | Menü / Icon-Leiste / Statusleiste | Ctrl+M / Ctrl+F6 / Ctrl+F5 |
| ToggleScrollAreaAction | Toggle Mini Map | Ctrl+Shift+S |
| ToggleMenueAndTabsAction | Toggle Menu, Tabs, Iconbar | **F1** |
| FadeInAndOutMenueAndTabsAction | Fade In&Out: Menu, Tabs, Iconbar | Ctrl+Shift+F1 |
| ToggleWindowModeAction | Toggle Window Mode (Vollbild) | Ctrl+Shift+M |
| Show/Hide/AutoTabComponentAction | File Tabs: zeigen / verstecken / auto | – |
| IncreaseFontSizeAction / Decrease / Reset | Schrift größer/kleiner/zurück | Ctrl+Num+ / Ctrl+Num− / Ctrl+F9 |
| OpenSettingsAction / SaveSettingsAction | Settings… / Save Settings | Ctrl+Shift+O / Ctrl+Alt+S |
| ImportColorStyleAction | Import Webmatrix Color Style | Ctrl+Shift+Alt+I |
| ShowKeystrokesAction | Show Keystrokes… | Ctrl+Shift+K |
| OpenAboutDialogAction / DoWebCheckAction | About… / Update-Prüfung | Ctrl+F1 / Ctrl+F12 |
| DebugAction | Debug… | Ctrl+F2 |

Beobachtungen: Kürzel sind über eine `KeyMapping`-Konfiguration (JAXB) überschreibbar, je Aktion bis zu fünf
Kürzel. Doppelte Belegungen (Ctrl+U, Ctrl+Alt+S) zeigen, dass die Keymap nie aufgeräumt wurde – die Neuauflage
prüft Kollisionen per Test. Auf macOS gilt Cmd statt Ctrl.

## §3 Dateien (FileManager, FileUtil)

- Mehrere Dateien gleichzeitig, eine ist ausgewählt; nächste/vorige, „Switch to…“-Menü mit allen Dateien.
- Encoding-Erkennung beim Laden mit Mozilla `UniversalDetector` (Standard UTF-8), Encoding wird gemerkt und beim
  Speichern verwendet.
- Drag & Drop von Dateien ins Fenster öffnet sie.
- Ungespeicherte Änderungen (`hasDocumentChanges`) → Rückfrage beim Schließen/Beenden; Optionen
  `autoSaveFilesOnClose`, `autoSaveFilesOnApplicationExit` (null = fragen).
- „Projekt“-Datei (JAXB): offene Dateien mit Caret/Auswahl (`OpenedFile`), zuletzt benutzte Dateien,
  Farbschema, Keymap, Optionen. `loadLastFiles` stellt die Sitzung wieder her.

## §4 Editor-Verhalten

- Einfüge-/Überschreibmodus (Statusleiste „Insert“/„Overwrite“).
- Tabweite 2/4/6/8 (Standard 8), Einrückungslinien, Zeilennummern als `LinenumberStyle` (u. a. ABSOLUTE).
- Spaltenmodus (Blockauswahl); Todo: Breite 0 erlauben (zum Einfügen in mehrere Zeilen).
- Falten der Auswahl, alles aufklappen; Todo: Falten nach Ebene, Kommentare, Methoden.
- Autovervollständigung (Todo: erst ab 2 Zeichen), Zen Coding (JS-Plugin `zen_coding.js` über die ScriptEngine).
- Testdatei `docs/test.txt` prüft Tabs gemischt mit Leerzeichen – Tabs werden auf Tabstopps ausgerichtet.

## §5 Optionen (`config/Options.java`)

| Option | Standard | Neuauflage |
|---|---|---|
| autoSaveOptionsOnExit | true | Einstellungen werden sofort gespeichert |
| fullScreen | false | Vollbild-Aktion (005) |
| mainMenuVisible / iconBarVisible / statusBarVisible | true / false / true | Menü aus, keine Icon-Leiste, Statusleiste an (005/010) |
| lineNumbersVisible | ABSOLUTE | aus/absolut/relativ (009) |
| tabComponentVisible | null = auto | Tabs auto/an/aus (004) |
| scrollingAreaVisible | true | Übersichtsleiste (010) |
| wordwrap | null = auto | Umbruch (009/010) |
| autoReloadExternalFileChanges | false | Änderungen von außen (004) |
| autoSaveFilesOnApplicationExit / OnClose | null = fragen | Rückfrage (004) |
| loadLastFiles | true | Sitzung wiederherstellen (004) |
| checkForNewVersionOnStartup | true | Updater später (012) |
| windowPosition | 0,0,1000,750 | Fenstergröße merken |
| tabIndentation | 8 | 4 als neuer Standard, 2/4/6/8 wählbar |
| indentationLinesVisible | true | (009) |
| defaultTypeFace | Monospaced 14 | System-Monospace 14 px (009) |
| proxy | – | entfällt |

## §6 Farbstile und Grammatiken

- `StyleManager` mit vier eingebauten Farbstilen, F12 schaltet weiter; Import von „Webmatrix“-Farbstilen.
- Grammatiken als eigene Klassen; Todos wollen TextMate-Grammatiken (`.tmbundle`) und mehr Stile (studiostyl.es).
- Log-Dateien: eigener Modus mit Farbe je Level (Todo-Liste, s. §8).

## §7 Oberfläche

- Animierte, verschiebbare Tabs (`MovableTabsPanel`), Tabs auto-sichtbar ab der zweiten Datei.
- Statusleiste: Dateiindex, Dateiname, „Line r, Column c“, Encoding, Tabweite, Insert/Overwrite, Speicheranzeige.
- „Sheet“-Dialoge (macOS-Stil, eingeblendet am Fensterkopf) für Rückfragen.
- Menüs: File, Edit, View (Dateien, Marker, Falten, Ansichtsschalter, File Tabs, Font), Options, Help.

## §8 Offene Wünsche aus `Todos.java` (Auswahl)

- Zeilen sortieren: ganzer Text oder Auswahl, auf/ab, alphanumerisch (Groß/klein ignorieren), numerisch
  (1, 2, 5, 10, 20, 100), Duplikate entfernen.
- Tab / Shift+Tab rückt die Auswahl ein/aus.
- Zeilenenden sichtbar machen ([LF], [CR], [CR][LF]).
- „Save Copy As“ (Sicherungskopie `name_datum.suffix`).
- Suchen & Ersetzen mit Verlauf, Paare Suchen/Ersetzen zusammen, Verlauf eindeutig, je Projekt.
- Log-Modus: Filter nach Level, Auto-Reload, Datumsbereich, doppelte Zeilen überspringen, nur zeigen / ignorieren
  mit Regex.
- Textvergleich (Diff), Wörterbücher (Rechtschreibung), Plugin-Mechanismus.
- Farbe als Hintergrund für Farbwerte wie `#7dff333a` (CSS).

## §9 Bewusste Abweichungen der Neuauflage

- **Große Dateien zuerst:** die alte Version hielt den Text im Swing-Modell; die Neuauflage hält ihn als Rope im
  Rust-Backend und rendert nur den sichtbaren Ausschnitt (auch horizontal) – Ziel sind Dateien mit Millionen Zeilen
  und Zeilen mit Millionen Zeichen.
- Strg/Cmd+S speichert die **aktuelle** Datei (heutige Konvention); „Alle speichern“ bekommt Mod+Alt+S.
- Kein Proxy, keine Webprüfung beim Start (Updater ggf. in 012), keine Icon-Leiste (Befehlspalette statt Icons).
- Einstellungen sofort speichern statt „Save Settings“.
- Zen Coding / Autovervollständigung / Falten erst als Folge-Ideen.

## §10 Alte Webseite (`java-k-editor/Web`, 2013)

- Foundation-3-Seite (jQuery, Orbit-Slider mit animierten GIF-Screenshots), Navigation Übersicht · Features ·
  Download · Impressum; Akzentfarbe Rot `#e00034` auf hellem, leicht strukturiertem Grund; Logo = Schriftzug +
  Füllfederhalter (Schriften im Ordner `src/`: Lobster, FFF Tusj).
- Claim: **„Is tastier than chocolate“** – „A new text editor for Windows & Mac“. Name = *Editor* + *Delicious*:
  „Dieser Texteditor macht jedem Anwender Spaß.“
- Versprochen: schlanke Oberfläche, konfigurierbares Aussehen und Handling, Syntax-Hervorhebung und
  Autovervollständigung für diverse Grammatiken (inkl. Variablen im Scope), Performance beim Rendern bei wenig
  Speicher, Erweiterbarkeit per JavaScript (Zen Coding integriert).
- Features mit Screenshots: Minimap (Dokument zehnfach verkleinert, rechts), Farbwerte als Farbfelder im linken
  Rand, helles und eigenes Farbschema, Zeilen- und Blockauswahl (beim Kopieren und Einfügen berücksichtigt),
  Zen Coding, intelligente Tooltips (z. B. Ergebnis arithmetischer Ausdrücke), Code-Faltung, relative/absolute
  Zeilennummern, bis zu **10 Wörter** in verschiedenen Farben markieren, „Als HTML kopieren“ im aktuellen
  Farbschema (z. B. in E-Mails einfügen), Schriftgröße per Mausrad, alle Menüpunkte per Kürzel, Undo mit
  > 200 Schritten.
- Download war nie öffentlich („früher Prototyp“).
- Für die neue Seite (Prompt 011) übernehmen: Claim-Ton (verspielt, „lecker“), Schriftzug, rotes Akzent-Erbe,
  Feature-Liste mit kurzen bewegten Beispielen; neu: große Dateien als Hauptbotschaft, Download über GitHub.
