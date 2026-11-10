# Design 027: Auswahl und Bereiche abfragen

> Prompt: [027-auswahl-abfragen.md](027-auswahl-abfragen.md) (schema-commander E2). Stand 10.11.2026.

## Entscheidungen

- **E1 Abfragen sind keine Befehle**: `Session::selections()` (alle Auswahlen in Dokumentreihenfolge + Index der
  Hauptauswahl) und `Session::range_text(from, to)` lesen nur. `ed-wasm`: `Editor.selections()` (JSON, 0-basiert)
  und `Editor.rangeText(l1, c1, l2, c2)`. Der Engine-Host beantwortet `selections` in **einem** Schritt mit
  Positionen **und** Texten (ein leeres Caret kostet keinen Aufruf) – zwischen Position und Text kann sich nichts
  ändern. Die Fassade wartet vorher auf `view.idle()` (Tippen direkt davor ist angekommen).
- **E2 Kosten**: Rope-Slice, O(log n + Länge des Bereichs). Messung `measure_027…` (Release, M-Serie): kleine
  Auswahl in 5 Mio. Zeilen **0,67 µs** je Abfrage. Obergrenze wie `Document::text`: 64 MB, darüber
  `selection_too_large` – im Typ dokumentiert.
- **E3 `replaceRange`** ist ein Engine-Befehl `ReplaceRange { from, to, text, caret }` (`is_edit`, also bei
  Schreibschutz `read_only`; wird wie jeder Befehl aufgezeichnet): eine Änderung = **ein** Undo-Schritt.
  Gleicher Text (nach Angleichen der Zeilenenden ans Dokument) → nichts, kein Undo-Schritt, keine neue Revision.
  **Cursor bleiben im Text**: davor unverändert, dahinter verschoben, im Bereich → ans neue Ende; das ist, was
  „Auswahl formatieren“ von außen braucht. `caret: true` setzt stattdessen ein Caret hinter den neuen Text
  (Vervollständigung, 028). Damit Redo dieselben Cursor zurückbringt, setzt `Document::set_sel_after` die
  Auswahl des frischen Undo-Schritts. `replaceSelection(text)` ist der vorhandene Befehl `insert` (ersetzt jede
  Auswahl, ein Schritt).
- **E4 `ed-caret`** bekommt `selStart`/`selEnd` (Hauptauswahl in Dokumentreihenfolge, ohne Auswahl beide = Caret);
  die alten Felder bleiben. Dafür trägt der Frame `anchorLine`/`anchorCh` (wo die Hauptauswahl begann).
- **E5 Zählung**: Positionen der Seite sind `{ line, ch }`, **1-basiert** wie `goTo()` und `ed-caret` bisher;
  `ch` zählt **Unicode-Codepunkte** in der Zeile (so zählt die Engine: Rope-`char`). Ein Emoji ist 1 (in JS
  `length` 2), ein kombinierendes Zeichen zählt einzeln, ein Zeilenende (auch CRLF) gehört zu keiner Spalte.
  Nicht UTF-16, weil `goTo`/`ed-caret` seit 025 so zählen und die Engine keine UTF-16-Indizes führt; die README
  nennt die Umrechnung (`[...lineText].slice(0, ch - 1).join('').length`). Texte kommen mit den Zeilenenden des
  Dokuments zurück. Positionen werden ins Dokument geklemmt, die Reihenfolge von Start/Ende ist egal.

## Verträge

```ts
interface EdPos { line: number; ch: number }            // 1-basiert, ch in Codepunkten
interface EdSelection { text: string; start: EdPos; end: EdPos }
getSelection(): Promise<EdSelection>                    // Hauptauswahl; leer: text '', start = end = Caret
getSelections(): Promise<EdSelection[]>                 // alle Cursor/Blockauswahl, Dokumentreihenfolge
getRange(start: EdPos, end: EdPos): Promise<string>
replaceRange(start: EdPos, end: EdPos, text: string): Promise<void>
replaceSelection(text: string): Promise<void>
// ed-caret: { line, ch, selChars, selLines, carets, selStart, selEnd }
```

Rust `Command::ReplaceRange` ↔ TS `{ type: 'replaceRange'; from; to; text; caret? }`; `Frame.anchor_line/_ch` ↔
`anchorLine/anchorCh`. Engine-Host-Ops `selections`, `rangeText`; `EmbedEngineApi.selections/rangeText`.

## Tests

- Rust `editor/tests.rs`: Bereichstext mit Emoji, kombinierenden Zeichen, CRLF exakt; Auswahlen in
  Dokumentreihenfolge + Anker im Frame; `replaceRange` ein Undo-Schritt, gleicher Text ändert nichts (Revision,
  `can_undo`); Cursor bleiben im Text; Schreibschutz; Messung 5 Mio. Zeilen (ignored).
- `ed-wasm`: `selections`, `rangeText`, `replaceRange` über JSON.
- Vitest: Engine-Host (`selections` mit Texten in einem Schritt), Fassade (1-basiert, Dokumentreihenfolge bei
  Rückwärtsauswahl, Befehle), `ed-caret` mit `selStart`/`selEnd`.
- Ende-zu-Ende (10.11.2026, eingebauter Browser, `site/embed/`): `a👍b\r\ne◌́x\r\nlast`, Auswahl 1:2–2:3 →
  `"👍b\r\né"` exakt; `replaceRange` mit demselben Text lässt den Text unverändert; ersetzen + Undo = ein Schritt.
  Dieselben Schritte stehen in `scripts/embed-e2e.mjs`.
