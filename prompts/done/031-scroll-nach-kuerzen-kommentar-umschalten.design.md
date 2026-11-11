# Design 031: Ansicht nach dem Kürzen der längsten Zeile; Zeilenkommentar umschalten

> Prompt: [031-scroll-nach-kuerzen-kommentar-umschalten.md](031-scroll-nach-kuerzen-kommentar-umschalten.md)
> (schema-commander 023). Stand 11.11.2026.

## Teil A – waagerechte Ansicht

### Befund (geprüft, nicht nur gelesen)

Die Vermutung aus dem Prompt stimmt, und es gab eine zweite Ursache. Rote Tests in `editor/tests.rs`:

1. `Session::max_width` wuchs ohne Umbruch nur (`max(fl.width)` beim Zeichnen) und wurde nur beim Neuladen und beim
   Umschalten des Umbruchs geleert. Nach dem Kürzen der breitesten Zeile meldete der Frame weiter die alte Breite:
   der Rollbalken blieb, `clamp_scroll` ließ `left` bis zur alten Breite zu.
2. `clamp_scroll` erlaubte `left` bis zur **Zelle des Cursors** („der Cursor darf rechts von allem stehen“). Ein
   Cursor in Spalte 10 hielt die Ansicht damit bis zu 10 Zellen rechts fest, obwohl alles in die Breite passte
   (Test `the_view_is_never_scrolled_past_a_short_text_by_its_caret`: `left` war 6 statt 0).

### Entscheidungen

- **E1 Breite neu bestimmen aus den Zeilen im Blick, sobald sich der Text geändert hat.** `clamp_scroll` merkt sich
  die Revision des Dokuments (`width_revision`); weicht sie ab, wird `max_width` aus den sichtbaren Zeilen neu
  gebildet (`width_in_view`, dieselben Regeln wie im Frame: lange Zeilen zählen ihre Zeichen, mit Umbruch höchstens
  die Umbruchbreite). Das greift bei jeder Änderung – Tippen, `replaceRange`, Zeilen löschen, Undo/Redo –, weil
  alle durch `Document::apply_raw` laufen und die Revision erhöhen.
  *Verworfen: die breiteste Zeile mitführen.* Dafür müsste jede Änderung wissen, ob sie die breiteste Zeile trifft,
  und nach deren Kürzen die zweitbreiteste finden – das wächst mit der Dokumentlänge oder braucht einen Index über
  alle Zeilenbreiten (5 Mio. Einträge, bei jeder Änderung zu pflegen).
- **Kosten:** höchstens die Zeilen im Blick (≤ 2000 Zeilen, normale Zeilen ≤ 10 000 Zeichen, lange Zeilen ohne
  Layout), einmal je Textänderung, nie beim reinen Rollen oder Bewegen. Nichts wächst mit der Dokumentlänge.
  Messungen (Release, M-Mac, 11.11.2026) unverändert im Rahmen: 100 Tastendrücke am Ende einer 10-Mio.-Zeichen-Zeile
  0,19 ms; Tippen in 1 Mio. Zeilen 214 µs/Taste, Rollen 123 µs/Frame; 1-GB-Datei: 100 Tastendrücke ohne
  Hervorhebung 27 ms.
- **E2 Der Cursor hält die Ansicht nur so weit rechts, wie es braucht, ihn zu zeigen:**
  `left ≤ max(max_width − Spalten + 2, Cursorzelle − Spalten + 1 + Rand)`. Die Regel „steht er rechts von allem,
  was im Blick ist, bleibt die Ansicht bei ihm“ gilt weiter (Ende einer langen Zeile, auch mit Umbruch) – nur nicht
  mehr weiter rechts als nötig. `reveal` läuft wie bisher danach.
- **E3 Folge für die Breite ohne Umbruch:** `max_width` ist nach einer Änderung „breiteste Zeile im Blick“ und
  wächst beim Rollen wieder mit dem, was gesehen wird (wie vorher). War die Ansicht von Hand hinter das Ende aller
  sichtbaren Zeilen gerollt, holt die nächste Textänderung sie an deren Ende zurück – dort stand ohnehin nichts.
  Eine Änderung anderswo verschiebt eine Ansicht, die Text zeigt, nicht (Test
  `a_change_elsewhere_does_not_move_the_view`).
- **E4 Rollbalken und `ed-caret`** brauchen nichts Neues: beide lesen `Frame.max_width`/`left` bzw. den Cursor.

## Teil B – Zeilenkommentar umschalten

- **E5 Kommentarzeichen aus einer eigenen Tabelle** (`editor/comment.rs`, `line_comments(sprache)`), nicht aus
  `Comments.tmPreferences`. Geprüft: syntect liefert die Metadaten nur mit dem Feature `metadata` als eigenes
  Paket (`default_metadata.packdump`), das die Engine nicht lädt; es einzuschalten kostet Größe im WebAssembly
  (und die eigenen Grammatiken Log und INI hätten dort ohnehin keinen Eintrag). Der Grammatiksatz ist fest
  (76 Namen), die Tabelle deckt 46 davon ab und ist per Test an die Namen gebunden.
  Mehrere Zeichen: das erste wird gesetzt, alle werden erkannt (PHP `//` und `#`, INI `#` und `;`).
  SQL kennt nur `--` (`#` ist in PostgreSQL ein Operator) – MySQL gibt die Seite vor (E7).
- **E6 Sprachen ohne Zeilenkommentar (CSS, HTML, XML, JSON, Markdown, reiner Text): die Aktion ändert nichts** und
  meldet `no_line_comment` („Diese Sprache hat keinen Zeilenkommentar.“). Kein Blockkommentar um die Zeilen: CSS
  verschachtelt `/* */` nicht, zweimal umschalten gäbe den Text nicht sicher Zeichen für Zeichen zurück.
  `runAction` liefert weiter `true` (die Aktion gibt es) – ob eine Sprache ein Zeichen hat, weiß nur die Engine;
  dafür ein Feld im Frame und eine `enabled`-Regel war den Vertrag nicht wert.
- **E7 Zeichen von außen:** Befehl `setLineComment { marks: string[] | null }` (nur Ansicht: nicht aufgezeichnet,
  auch schreibgeschützt), in der Web Component die Eigenschaft `lineComment: string | string[] | null` (Attribut
  `line-comment`, Zeichen durch Leerzeichen getrennt; `null` = die der Sprache). Leere Zeichen, solche mit
  Leerraum und mehr als 8 werden verworfen.
- **E8 Verhalten** (`Command::ToggleComment`, `is_edit`; Vorlage `toggleComment` aus schema-commander):
  betroffen sind die Zeilen aller Cursor (eine Auswahl, die am Zeilenanfang endet, meint diese Zeile nicht – wie
  Einrücken), jede einmal. Sind alle Zeilen mit Text Kommentare, geht das Zeichen samt **einem** Leerzeichen weg;
  sonst bekommt jede Zeile mit Text Zeichen + Leerzeichen an der Einrückung der am wenigsten eingerückten Zeile.
  Zeilen aus Leerraum bleiben, Zeilenenden werden nie berührt. Eine Bearbeitung = ein Undo-Schritt.
- **E9 Cursor bleiben auf demselben Text:** Positionen ab der Einfügestelle wandern mit; der Anfang einer Auswahl
  genau an der Einfügestelle bleibt (die Auswahl umfasst das Zeichen, wiederholtes Umschalten trifft dieselben
  Zeilen). Beim Entfernen landet ein Cursor im Zeichen am Textanfang.
- **E10 Große Auswahlen:** je Zeile zwei kleine Rope-Ausschnitte (Einrückung in Stücken von 1024 Zeichen, dann
  Zeichenlänge + 1), die Änderungen gehen als eine Liste durch `Document::apply`. 100 000 Zeilen: 170 ms
  (Release; Einrücken derselben Zeilen: 110 ms).
- **E11 Kürzel `Mod+/`** in `keymap.base.json`. Deutsche Tastatur: `/` ist Shift+7 – `shortcut.ts` liest
  `KeyboardEvent.key` und zählt Shift bei Satzzeichen nicht, `Strg+Shift+7` ergibt also `Mod+/`; kein zweites
  Kürzel nötig (im Browser mit `key: '/'`, `shiftKey: true` geprüft). `Mod+7` (Datei 7) bleibt unberührt.

## Verträge

- Rust `editor/model.rs` ↔ TS `editor.models.ts`: `{ type: 'toggleComment' }`,
  `{ type: 'setLineComment', marks: string[] | null }`. Notiz `no_line_comment`.
- Aktion `edit.toggleComment` („Toggle line comment“ / „Zeilenkommentar umschalten“), Menü Bearbeiten → Zeilen.
- Skripte: `toggleComment` als einfacher Befehl (`docs/scripts.md`).
- Paket: `lineComment` in `editlicious-editor.d.ts` und README („Line comments“).

## Tests

- Engine `editor/tests.rs`: `replaceRange` (Cursor dahinter / im Text), Tippen über die Auswahl, Zeile löschen,
  Undo/Redo, Änderung anderswo, Tippen und Backspace am Ende einer breiten Zeile (eine Zelle je Taste), Cursor hält
  nicht fest, Umbruch mit langer Zeile.
- Engine `editor/comment.rs`: Katalog je Sprache (SQL, Rust, JavaScript/TypeScript, Python, Shell, YAML, Lua, LaTeX,
  Lisp) mit Hin und Zurück, Einrückung mit Leerzeichen und Tabs, CRLF/CR/leere Zeilen, Emoji und kombinierende
  Zeichen, gemischte Blöcke, Cursorzeile, Auswahl bis Zeilenanfang, mehrere Cursor + Undo/Redo, Zeichen von außen,
  reiner Text, Sprachen ohne Zeichen, Schreibschutz, 100 000 Zeilen.
- Frontend: Katalog/Keymap/Menü (vorhandene Vollständigkeitstests), `editlicious-editor.spec.ts` (Aktion,
  `lineComment` als Array, Attribut-Form, `null`).
- Ende-zu-Ende (11.11.2026, `site/embed/` im Browser, frisch gebautes WASM und Element): Zeile mit 300 Zeichen,
  Cursor ans Ende, `replaceRange` → Spalte 1, Rollbalken weg; `runAction('edit.toggleComment')` hin und zurück,
  `lineComment = ['#', '--']`, CSS → Hinweis in der Statuszeile, Tastendruck `/` mit Strg/Cmd+Shift.
  **Nicht geprüft:** die Desktop-App selbst (kein Debug-Build gestartet) – sie nutzt dieselbe Engine, dieselbe
  Aktion und dasselbe Menümodell.

## Für schema-commander

Mit der nächsten Veröffentlichung von `editlicious-editor` **nach 1.2.0** (noch nicht veröffentlicht): in
`sql-editor.ts` entfällt das `goTo(zeile, 1)` in `format()`, und `toggleComment()` kann
`runAction('edit.toggleComment')` rufen; für MySQL `lineComment = ['--', '#']` setzen.
