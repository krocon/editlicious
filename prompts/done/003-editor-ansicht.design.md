# Design 003: Editor-Engine in Rust und Render-Ansicht

Status: umgesetzt · Datum: 26.10.2026

## Entscheidungen

- **E1 Sitzung + Frames:** `ed_core::editor::Session` = Dokument + Cursors (Anker, Kopf, Ziel-Zelle) + Scrollstand
  (`top` in Zeilen, `left` in Zellen, beide gebrochen für weiches Scrollen) + Viewport + Tabweite/Leerzeichen +
  Überschreibmodus. Jeder Befehl (`editor_command(id, cmd)`) liefert einen **Frame**: sichtbare Zeilen als Läufe
  mit Zellposition, Auswahl je Zeile in Zellen (+ „über das Zeilenende“), Carets, Status (Zeile/Zeichen,
  Auswahlgröße), `DocInfo`. Die UI hält keinen Text und rechnet keine Positionen.
- **E2 Layout in Zellen:** Tab → nächster Tabstopp, East-Asian-Wide = 2 Zellen (`unicode-width`), kombinierende
  Zeichen = 0 (hängen am Lauf davor), Steuerzeichen = 1 Zelle als Steuerzeichen-Bild (U+2400…). Klick-Positionen:
  Hälfte einer Zelle entscheidet, nie zwischen Basiszeichen und Akzent. Cursor-Schritte nach Graphem
  (`unicode-segmentation`). Langzeilen (> 10 000 Zeichen): 1 Zelle je Zeichen, Schritte je Zeichen (002 E3).
- **E3 Fenster in beide Richtungen:** Frame enthält nur Zeilen `floor(top) … +ceil(viewLines)+1` und von jeder
  Zeile nur die Zeichen, die die Zellen `left−16 … left+cols+16` berühren. `maxWidth` = breiteste bisher gesehene
  Zeile (horizontaler Scrollbereich; wächst nur).
- **E4 Eigene, dezente Scrollleisten** (6 px Daumen, halbtransparent): Browser-Scrollflächen scheitern ab ~33 Mio.
  Pixel (10 Mio. Zeilen × 22 px). Mausrad/Trackpad → `scroll` (gebrochene Zeilen/Zellen, je Animationsframe
  zusammengefasst), Daumen ziehen/Spur klicken → `scrollTo`.
- **E5 Zwischenablage in Rust** (`arboard`, ein langlebiger Handle wegen X11): Kopieren/Ausschneiden/Einfügen
  bewegen keinen Text über die IPC; ohne Auswahl wird die ganze Zeile kopiert/ausgeschnitten. Einfügen aus dem
  Kontextmenü (`paste`-Ereignis) schickt den Text als `insert`.
- **E6 Tastatur:** reine Funktion `keyToCommand` (je OS: Wortsprung Alt ⌥ auf macOS, Strg sonst; ⌘←/→ Zeile,
  ⌘↑/↓ Dokument; Pos1/Ende; Bild↑/↓; Strg+↑/↓ scrollt; Einfg = Überschreiben; Mod+A/Z/Y/C/X/V). Bis Prompt 005
  der einzige `keydown`-Handler am versteckten Textfeld.
- **E7 Eingabe über verstecktes `textarea`** am primären Caret: `input` → `type`; IME-Komposition bleibt sichtbar
  im Textfeld, erst `compositionend` schickt den Text. Befehle laufen seriell (Promise-Kette).
- **E8 Smart Home**, Enter übernimmt die Einrückung, Tab mit Leerzeichen bis zum nächsten Tabstopp, Backspace
  entfernt weiche Tabs, Einrücken/Ausrücken mehrerer Zeilen (leere Zeilen und eine Auswahl, die in Spalte 0 endet,
  bleiben unberührt), Einrückung der Datei wird erkannt (Tabs/2/4 Leerzeichen).
- **E9 Mehrere Cursor im Modell** (Vec, zusammenführen bei Überlappung) – Bedienung (Alt+Klick, Blockauswahl)
  folgt in 007.
- **E10 Start:** die erste Datei der Kommandozeile (`startup_paths`) oder ein leeres Dokument; mehrere Dateien und
  Tabs in 004. Ohne Backend (Browser) bleibt der Startbildschirm (`DESKTOP`-Token).

## Verträge

Commands: `open_document`, `cancel_load`, `new_document`, `close_document`, `document_info`,
`editor_command(id, cmd) -> Frame`, `save_document`, `startup_paths`. `get_view`/`apply_edits`/`undo`/`redo`/
`get_text` aus 002 sind keine Commands mehr (die Sitzung nutzt die Funktionen direkt).

## Tests

- Rust `editor/tests.rs` (23): Layout (Tabs, breit, kombinierend), Grapheme, Bewegungen inkl. Ziel-Zelle,
  Wortsprünge, Smart Home, Seiten, Auswahl, Tippen + Undo, Enter-Einrückung, Backspace/Delete/Wort, weiche Tabs,
  Überschreiben, Ein-/Ausrücken, Zwischenablage, Klick/Doppel/Dreifach/Ziehen/Shift, Läufe je Art, Fenster,
  Langzeile mit 3 Mio. Zeichen (Tippen am Ende, Wortsprung), Scroll-Grenzen, mehrere Cursor, Einrückungserkennung.
- Frontend: `key-commands.spec.ts`, `editor-view.spec.ts` (Größe → resize, Zeilen, Tasten, Tippen, IME, Klick,
  Auswahl/Langzeile), `app.spec.ts` (Start mit Datei/leer/Browser).
- Ende-zu-Ende (Xvfb, Debug-Build): Unicode-Datei (Tabs, CJK, Emoji, Steuerzeichen), Klicken, Tippen, Enter,
  Shift-Auswahl; 140 MB / 2 Mio. Zeilen: Strg+Ende, Tippen; 3-Mio.-Zeichen-Zeile: Ende, Tippen, Wortsprünge.

## Offen / Folge

- Emoji mit Hautton werden im Container-Font als zwei Glyphen gezeigt (Font-Frage, Zellen stimmen).
- Weicher Umbruch (010), Einrückungslinien (009), Syntaxfarben (008).
