# Prompt 031: Ansicht nach dem Kürzen der längsten Zeile; Zeilenkommentar umschalten

> **Slug:** `scroll-und-kommentar` · **Gebraucht von:** Schema Commander 023 (SQL formatieren) · **Braucht:**
> 025, 027 · **Umfang:** S–M
> Lies zuerst `prompts/000-arbeitsweise.md`. Gefunden in schema-commander, Design
> `prompts/done/023-sql-formatieren.design.md`, Abschnitt „Gefunden“ (lokal unter `../schema-commander`).

## Ziel

Zwei Lücken, die beim Formatieren von SQL in Schema Commander aufgefallen sind und dort im Moment von außen
ausgeglichen werden:

1. Wird die längste Zeile kürzer, bleibt die Ansicht waagerecht dort stehen, wo die alte Zeile endete – der Text
   ist links abgeschnitten, obwohl jetzt alles in die Breite passt.
2. Der Editor hat keine Aktion, die Zeilen ein- und auskommentiert.

## Teil A – waagerechte Ansicht nach einer Änderung

### Befund

- Nachstellen: eine Zeile, die breiter ist als das Fenster; Cursor an ihr Ende (die Ansicht rollt nach rechts);
  dann `replaceRange(…)` mit einem Text aus mehreren kurzen Zeilen. Ergebnis: alle Zeilen sind kurz, die Ansicht
  bleibt um etliche Spalten verschoben, der waagerechte Rollbalken bleibt. Dasselbe mit Tippen: die lange Zeile
  markieren und durch ein Wort ersetzen. Erst ein `goTo(zeile, 1)` holt die Ansicht zurück.
- Vermutete Ursache (`src-tauri/core/src/editor/mod.rs`): `max_width` wächst beim Zeichnen nur
  (`self.max_width = self.max_width.max(fl.width)`) und wird nur an wenigen Stellen auf 0 gesetzt; `clamp_scroll`
  begrenzt `left` mit diesem alten Wert. Nach einer Änderung, die die breiteste Zeile kürzt oder entfernt, stimmt
  er nicht mehr. **Erst prüfen, dann beheben** – die Vermutung stammt aus dem Lesen des Codes, nicht aus einem
  Test.

### Aufgaben

1. Roter Test in der Engine (`editor/tests.rs`): lange Zeile, Cursor ans Ende, Zeile ersetzen → `left` ist 0 und
   `max_width` die Breite der jetzt breitesten Zeile. Je ein Fall für `replaceRange`, Tippen über eine Auswahl,
   Zeile löschen, Undo/Redo (nach Undo ist die lange Zeile wieder da: die Ansicht darf zum Cursor rollen).
2. Beheben: Die Breite, mit der `left` begrenzt wird, muss nach jeder Änderung des Textes stimmen. Entscheiden
   und im Design festhalten, wie – neu bestimmen aus den Zeilen im Blick, oder die breiteste Zeile mitführen –
   und was das bei 5 Mio. Zeilen kostet (nichts, was mit der Dokumentlänge wächst).
3. Die Regel für den Cursor bleibt: Steht er rechts von allem, was im Blick ist, darf die Ansicht bei ihm bleiben.
   Ohne Umbruch und mit Umbruch (lange Zeilen rollen dort weiter seitwärts) prüfen.
4. Der Rollbalken und `ed-caret` folgen dem neuen Stand ohne weiteres Zutun der Seite.

### Akzeptanzkriterien

- Nach `replaceRange`, das die breiteste Zeile kürzt, zeigt die Ansicht Spalte 1, wenn der Cursor im Blick ist –
  ohne `goTo` der Seite. Kein waagerechter Rollbalken, wenn alles passt.
- Eine Änderung irgendwo anders im Text verschiebt die Ansicht nicht.
- Kein Flackern beim Tippen am Ende einer langen Zeile; Rollen und Tippen in einer Datei mit 5 Mio. Zeilen sind
  nicht langsamer als vorher (die vorhandenen Messungen laufen lassen).

## Teil B – Zeilenkommentar umschalten

### Aufgaben

1. Aktion `edit.toggleComment` (Katalog, Menü „Bearbeiten“, Keymap je OS: `Mod+/`; auf deutschen Tastaturen
   liegt `/` auf Shift+7 – prüfen, wie `shortcut.ts` das liest, sonst ein zweites Kürzel), erreichbar über
   `runAction('edit.toggleComment')`.
2. Verhalten (wie die reine Funktion `toggleComment` in schema-commander
   `src/app/features/format/format-logic.ts`, deren Tests als Vorlage dienen dürfen):
   - betroffen sind die Zeilen der Auswahl bzw. die Zeile des Cursors; eine Auswahl, die am Anfang einer Zeile
     endet, meint diese Zeile nicht; bei mehreren Cursorn jede betroffene Zeile einmal;
   - sind alle Zeilen mit Text schon Kommentare, wird auskommentiert, sonst alle einkommentiert (auch die, die
     schon Kommentare sind);
   - das Zeichen steht an der Einrückung der am wenigsten eingerückten Zeile, gefolgt von einem Leerzeichen;
     beim Entfernen geht dieses eine Leerzeichen mit; leere Zeilen bleiben leer; Zeilenenden bleiben, wie sie sind;
   - **ein** Undo-Schritt; Auswahl und Cursor bleiben auf demselben Text.
3. **Das Kommentarzeichen der Sprache:** prüfen, was die Grammatiken hergeben (syntect: `Comments.tmPreferences`
   mit `TM_COMMENT_START` – sind diese Metadaten im gepackten Satz enthalten?). Empfehlung im Design: daraus
   lesen, mit einer kleinen eigenen Tabelle für die Sprachen, bei denen es fehlt; Sprachen mit mehreren Zeichen
   (MySQL: `--` und `#`) erkennen beim Entfernen alle, setzen das erste. Sprachen ohne Zeilenkommentar (CSS,
   HTML, JSON): Blockkommentar um die Zeilen, oder die Aktion ist dort nicht verfügbar (`runAction` → `false`) –
   entscheiden und begründen.
4. Für die Web Component: Die Seite kann das Zeichen vorgeben, wenn sie es besser weiß
   (Eigenschaft, z. B. `lineComment: string | string[] | null`; `null` = das der Sprache). Typen und README
   ergänzen, bestehende Methoden unverändert.
5. Schreibschutz: die Aktion tut nichts.

### Akzeptanzkriterien

- SQL, Rust, TypeScript, Python, Shell, YAML: zweimal umschalten stellt den Text Zeichen für Zeichen wieder her
  (auch mit CRLF, Tabs, Emoji, leeren Zeilen dazwischen).
- Ein eingerückter Block behält seine Form: das Zeichen steht in einer Spalte.
- Eine Auswahl über 100 000 Zeilen wird ohne spürbare Pause umgeschaltet und ist ein Undo-Schritt.
- In der App, in `<ed-editor>` und in `<editlicious-editor>` dieselbe Aktion.

## Tests

Engine: Scroll-Fälle aus Teil A; Kommentar-Fälle als Katalog (Vorher/Nachher je Sprache, Einrückung, gemischte
Blöcke, mehrere Cursor). Frontend: Aktion im Katalog, im Menü und in allen drei Keymaps (die vorhandenen Tests
verlangen es). Ende-zu-Ende in der echten App und in der Web Component.

## Danach in schema-commander (nicht Teil dieses Prompts, nur festhalten)

Nach der Veröffentlichung von `editlicious-editor` mit beiden Teilen kann schema-commander
`src/app/features/editor/sql-editor.ts` vereinfachen: das `goTo(zeile, 1)` vor dem Setzen des Cursors in
`format()` entfällt, und `toggleComment()` ruft `runAction('edit.toggleComment')` statt der eigenen Funktion.
Im Abschlussbericht die Version nennen, ab der das gilt.
