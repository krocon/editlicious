# Design 002: Dokumentmodell

Status: umgesetzt · Datum: 26.10.2026

## Entscheidungen

- **E1 Rope = `ropey` 1.6** (`default-features = false`, Features `simd`, `cr_lines`): Zeilenumbrüche sind nur LF,
  CR und CRLF (keine Unicode-Zeilentrenner wie U+2028 – die bleiben Zeichen in der Zeile, wie in jedem Editor).
  Der Text liegt **unverändert** im Rope, auch CRLF; Zeilentexte werden ohne Umbruch ausgeliefert.
- **E2 Positionen** sind `{ line, ch }` mit `ch` = Unicode-Skalarwerte (Rust `char`) innerhalb der Zeile ohne
  Umbruch. Das Frontend rechnet innerhalb eines gelieferten Textes selbst zwischen Skalarwerten und UTF-16 um.
  Ungültige Positionen werden geklemmt (Zeile ≤ letzte, `ch` ≤ Zeilenlänge).
- **E3 Viewport in zwei Modi:** Zeilen bis `FULL_LINE_LIMIT = 10 000` Zeichen kommen ganz; längere Zeilen
  (Langzeilen-Modus) nur als Fenster `[window.from, window.from + window.count)` in Zeichen. Jede Zeile meldet
  `{ text, from, len }`. Im Langzeilen-Modus ist jede Spalte genau ein Zeichen (auch Tab und breite Zeichen) – damit
  ist Spalte ↔ Zeichen O(log n) und Tippen am Ende einer 100-MB-Zeile kostet nicht jedes Mal einen Durchlauf über die
  Zeile. Die Ansicht (003) zeigt Tabs dort als schmalen Pfeil.
- **E4 Öffnen gestreamt:** Blöcke à 4 MiB; Encoding: BOM (UTF-8, UTF-16LE/BE) → sonst UTF-8 streng prüfen → bei
  Fehler Neustart mit `chardetng` (Probe der ersten 1 MiB) + `encoding_rs`. Binärdateien (NUL im ersten Block, außer
  UTF-16) → Fehler `binary_file`. Fortschritt über `Channel<LoadEvent>` höchstens alle 50 ms, Abbruch über eine Id.
  Grenze `MAX_FILE_BYTES = 4 GiB` (`file_too_large`).
- **E5 Zeilenende** wird beim Laden gezählt (LF/CRLF/CR); `lineEnding` = häufigstes, `mixedLineEndings` = mehrere.
  Enter fügt das Dokument-Zeilenende ein (neue Datei: LF; Windows-Standard folgt in den Einstellungen 009).
- **E6 Bearbeiten:** `apply_edits` nimmt nicht überlappende Änderungen in **Koordinaten vor der Änderung**,
  wendet sie von hinten nach vorne an und liefert die neue Revision und die Auswahl. Eingefügter Text wird auf das
  Dokument-Zeilenende normalisiert (Einfügen aus Windows in eine LF-Datei ergibt LF).
- **E7 Undo/Redo in Rust:** Schritt = Liste inverser Änderungen + Auswahl davor/danach. Zusammenfassen: gleiche
  Art (`type`/`delete`), direkt anschließend, < 1 s Abstand, kein Zeilenumbruch eingegeben. Verlauf begrenzt auf
  10 000 Schritte und 256 MiB gelöschten Text (älteste fallen weg). `dirty` = Revision ≠ gespeicherte Revision
  (Undo zurück zum gespeicherten Stand ist wieder „sauber“).
- **E8 Speichern:** Kodieren im Dokument-Encoding blockweise; nicht darstellbare Zeichen → Fehler
  `unmappable_character` mit Position (kein stilles `&#…;`). UTF-16 von Hand (encoding_rs kodiert nicht nach
  UTF-16). BOM wie geladen. Schreiben: Symlink auflösen, Temp-Datei im Zielordner, Rechte des Originals übernehmen,
  `fsync`, `rename`. Danach mtime/Größe merken (Änderungen von außen, 004).
- **E9 Registry** offener Dokumente mit `u64`-Ids, je Dokument ein `Mutex`; Lesen (View) und Schreiben laufen in
  `spawn_blocking`.
- **E10 ~~`MemoryDocumentApi`~~ entfällt** (Marc, 26.10.2026: „alle Aktionen in Rust, die UI rendert nur“). Die
  Fälle in `core/testdata/document.cases.json` sind datengetriebene Rust-Tests. Browser-Modus und Webseiten-Demo
  bekommen die Engine später als WebAssembly-Build von `ed-core` (Prompt 011).
- **E11 Text holen** (`get_text`) für Zwischenablage und Kopieren ist auf 64 MiB begrenzt (`selection_too_large`);
  Größeres geht in 003 über die Zwischenablage im Backend.

- **E12 Eigenes Crate `ed-core`** (`src-tauri/core`, Workspace-Mitglied): Dokumentmodell und – ab 003 – die ganze
  Editor-Engine ohne Tauri; `error.rs` liegt dort, `ed_lib` re-exportiert `document` und `error`. Grund: reine Tests,
  WASM-fähig, klare Grenze zwischen Engine und App-Rahmen.
- **E13 Rust ist die Editor-Engine, die UI rendert nur** (Marc, 26.10.2026): Cursor, Auswahl, Navigation,
  Bearbeitungsbefehle, Scrollstand und Layout (Zellen) entstehen in Rust; die UI schickt Befehle und zeichnet
  Frames (Prompt 003). Die Commands dieses Prompts (`get_view`, `apply_edits` …) bleiben als Bausteine und für Tests;
  das Frontend nutzt ab 003 die Sitzungs-Commands.

## Verträge

```ts
type DocId = number;
interface Pos { line: number; ch: number }
interface Sel { anchor: Pos; head: Pos }
interface DocInfo { id; path: string|null; name: string; encoding: string; bom: boolean; lineEnding: 'lf'|'crlf'|'cr';
                    mixedLineEndings: boolean; lineCount: number; charCount: number; revision: number; dirty: boolean;
                    canUndo; canRedo }
open_document(path, onEvent: Channel<LoadEvent{ kind:'progress', loaded, total }>, loadId) -> DocInfo
cancel_load(loadId)
new_document() -> DocInfo
close_document(id)
document_info(id) -> DocInfo
get_view(id, { firstLine, lineCount, windowFrom, windowCount }) -> { revision, lineCount, lines: ViewLine[] }
  ViewLine { text: string; from: number; len: number }
apply_edits(id, { edits: { from: Pos; to: Pos; text: string }[], kind: 'type'|'delete'|'other', selBefore: Sel[], selAfter?: Sel[] })
  -> EditResult { info: DocInfo; selections: Sel[] }   // selAfter berechnet, wenn nicht gegeben (Caret hinter dem letzten Einfügen)
undo(id) / redo(id) -> EditResult | null
get_text(id, from: Pos, to: Pos) -> string
save_document(id, { path?: string; encoding?: string; lineEnding?: ...; bom?: boolean }) -> DocInfo
```

Neue Fehlercodes: `unknown_document`, `binary_file`, `file_too_large`, `unsupported_encoding`,
`unmappable_character`, `selection_too_large`, `cancelled`.

## Tests

- Rust (`document/tests.rs`): Encodings (UTF-8 ± BOM, UTF-16LE/BE, Windows-1252, Shift_JIS-Probe), Zeilenenden
  (LF/CRLF/CR/gemischt, ohne Abschluss), Binärdatei, leere Datei, Emoji/CJK/kombinierende Zeichen, Langzeilen-Fenster,
  Undo-Gruppen, Dirty nach Undo, Speichern rund (Bytes gleich), Rechte erhalten, Symlink bleibt Symlink,
  unmappable, Abbruch. Gemeinsame Fälle `document.cases.json`. Messung (ignored): 100 MB / 1 Mio. Zeilen öffnen,
  Fenster in 10-Mio.-Zeichen-Zeile.
- TS: entfällt in 002 (E10/E13) – die Fassade entsteht mit der Sitzung in 003.

## Messungen (Release, Linux-Container, 26.10.2026)

| Fall | Zeit |
|---|---|
| 101 MB / 1,3 Mio. Zeilen öffnen (UTF-8, Zeilenenden zählen, Rope bauen) | 191 ms |
| Ansicht 60 Zeilen am Dateiende | 0,07 ms |
| Fenster 400 Zeichen mitten in einer 10-Mio.-Zeichen-Zeile | 0,007 ms |
| 100 Tastenanschläge am Ende dieser Zeile (inkl. Undo-Zusammenfassung) | 0,37 ms gesamt |

## Fortschritt

- [x] Rust: Laden, Ansicht, Bearbeiten, Undo/Redo, Speichern, Registry, Commands; 21 Tests + Messung grün.
