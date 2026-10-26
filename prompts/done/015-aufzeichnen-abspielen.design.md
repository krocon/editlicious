# Design 015: Aufzeichnen und Abspielen

> Prompt: [015-aufzeichnen-abspielen.md](015-aufzeichnen-abspielen.md). Stand 26.10.2026.

## Entscheidungen

- **E1 Befehle, nicht Tasten:** aufgezeichnet werden Engine-Befehle (`editor::model::Command`) mit Zeitstempel in ms
  seit Beginn (`ed-core/src/editor/recording.rs`). Befehle, die nur die Ansicht betreffen (`frame`, `resize`,
  `scroll`, `scrollTo`, `configure`, `copyHtml`), werden nicht aufgezeichnet.
- **E2 Ansichtsabhängiges als Auswahl:** Klick, Ziehen, Bild auf/ab und Auf/Ab bei Umbruch hängen von Fenstergröße
  und Scrollstand ab. Statt des Befehls wird die **Auswahl danach** als neuer Befehl `select { sels }` (Anker/Kopf
  in Zeile/Zeichen, mehrere Cursor möglich) gespeichert – Abspielen ist so unabhängig von der Fenstergröße.
  Eine Blockauswahl wird dabei zu mehreren Cursorn (spätere `blockMove` wirken dann wie ohne Block).
- **E3 Zwischenablage:** `paste` speichert den eingefügten Text mit (`clip`); beim Abspielen benutzt die Engine eine
  eigene **Speicher-Zwischenablage** – Abspielen liest und schreibt nie die System-Zwischenablage. Texte über
  1 MiB werden nicht eingebettet (`clipRef` mit Länge) – Abspielen bricht dort mit `recording_unsupported` ab.
- **E4 Format `*.edrec.jsonl`:** Kopfzeile `{ "format": "edrec", "version": 1, "app", "doc": { name, lines, hash },
  "start": { sels }, "settings": { tabWidth, insertSpaces, wrap } }`, danach je Zeile `{ "t": ms, "cmd": {…},
  "clip"?: "…" }`. Laden nachsichtig: unbekannte Felder egal, unbekannte Befehle → `recording_invalid` mit
  Zeilennummer. Höchstens 1 Mio. Schritte. `hash` = FNV-1a 64 über den Text beim Start (Abweichung → Warnung).
- **E5 Skript `*.edscript`:** eine einfache Zeilen-Sprache statt YAML (keine Abhängigkeit, gut von Hand zu
  schreiben, Fehler mit Zeilennummer). Befehle:
  `tempo original|factor X|even MS|human [cpm=N] [seed=N] [typos=P]`, `type "…"` (Escapes `\n \t \" \\`),
  `type <<END … END` (Textblock), `type @datei` (relativ zum Skript), `pause MS`, `goto Z:S`, `select Z:S Z:S`
  (1-basiert), `move <motion> [select]`, `newline|backspace|delete|undo|redo|indent|outdent|selectAll|copy|cut|paste
  [N]`, `find "…" [regex] [case] [word] [back]`, `do {json}` (jeder Engine-Befehl), Kommentare mit `#`.
- **E6 Tippen im Skript:** `type` erzeugt je Graphem einen `type`-Befehl; ein Zeilenumbruch wird als `type "\n"`
  eingefügt (ohne automatisches Einrücken) und die führenden Leerzeichen der nächsten Zeile in **einem** Schritt
  (wirkt wie Auto-Einrücken, Ergebnis ist genau der Skripttext).
- **E7 Tempo:** reine Funktion `timeline(steps, tempo) → [(at_ms, schritt)]`, deterministisch:
  `original` = aufgezeichnete Zeiten; `factor` = geteilt durch X (0,25–8); `even` = feste ms je Schritt;
  `human` = je Zeichen 60 000/cpm ms (Standard 330 cpm) × Streuung (0,55–1,6) aus einem SplitMix64-Zufall mit
  `seed`, länger nach Satzzeichen (×3) und Zeilenende (×4), kurze Wortpausen; `typos` = Wahrscheinlichkeit je
  Buchstabe für einen Tippfehler (Nachbartaste auf QWERTY, Pause, Rückschritt, richtige Taste). Pausen aus dem
  Skript bleiben in allen Modi unverändert (außer `factor`).
- **E8 Eine Undo-Gruppe:** `History::begin_group()/end_group()` fasst alle Schritte einer Abspielung zu einem
  Gruppen-Schritt zusammen (`Step.group`); Rückgängig nimmt die ganze Abspielung zurück. Auch bei Abbruch.
- **E9 Abspielen läuft in Rust:** ein Thread der App (`playback.rs` im App-Crate) wartet bis zum nächsten Zeitpunkt,
  führt den Schritt unter der Sitzungssperre aus und schickt den **Frame** über einen `tauri::ipc::Channel`
  (`PlaybackEvent::Frame | Progress | Done`). Steuerung über `playback_control(id, pause|resume|stop|step)`.
  Die Logik (Zeitachse, Schritte anwenden) liegt rein in `ed-core::editor::playback` – 016 (MCP) und 017 (Video)
  benutzen sie ebenso.
- **E10 Sperre:** während der Wiedergabe lehnt `editor_command` bearbeitende Befehle mit `playback_running` ab
  (Ansicht, Scrollen, Größe gehen weiter); die UI ignoriert Eingaben im Editor, Esc stoppt.
- **E11 Bedienung:** Aktionen `record.toggle` (Strg/⌘+⇧+R), `record.replay` (Strg/⌘+Alt+R, letzte Aufnahme
  sofort ohne Tempo – Makro), `record.save`, `record.saveScript`, `record.play` (Datei wählen, Tempo wählen),
  `record.pause`, `record.stop`; Menü Bearbeiten → Aufzeichnung. Sichtbar nur in der Statusleiste: „● REC“ bzw.
  „▶ 37 %“ / „❚❚“. Die Aufnahme lebt in der Sitzung (je Dokument), die letzte bleibt bis zur nächsten.
- **E12 Export:** Aufnahme → Skript fasst aufeinanderfolgende `type` zu `type "…"` zusammen, Pausen > 400 ms
  werden `pause`, `select`/`goTo` → `select`/`goto`, Bewegungen → `move`, Rest → `do {json}`.

## Verträge

- Rust `Command::Select { sels: Vec<Sel> }` (neu, auch für 016).
- Tauri: `start_recording(id)`, `stop_recording(id) → RecordingInfo { steps, durationMs }`,
  `save_recording(id, path, script: bool)`, `play(id, source: { path } | { last }, tempo?, onEvent)`,
  `playback_control(id, action)`; `pick_open_files(near, filters?)`.
- Fehlercodes: `recording_invalid`, `recording_unsupported`, `playback_running`, `no_recording`.

## Tests

- Rust: Aufnahme → Abspielen ergibt gleichen Text und gleiche Cursor (ASCII, Emoji, CJK, kombinierende Zeichen,
  CRLF, mehrere Cursor, Klick → `select`, Einfügen); JSONL hin und zurück; Skript-Parser inkl. Fehlerzeilen;
  Zeitachse deterministisch (Seed), Tippfehler korrigieren sich; Export → Skript → Abspielen gleich; Undo-Gruppe
  inkl. Abbruch; Sperre.
- Angular: Aktionen, Statusanzeige, Eingabesperre, Esc stoppt (Fakes).
- Ende-zu-Ende: Skript in der echten App abspielen, Screenshot.
