# Design 030: Textbausteine mit Platzhaltern

> Prompt: [030-snippets.md](030-snippets.md) (schema-commander E5). Stand 10.11.2026. Vor 028 umgesetzt, weil
> dessen `insertText` Snippets braucht; die Kürzel-Liste (`el.snippets`) nutzt die Vorschlagsliste aus 028.

## Entscheidungen

- **E1 Der Platzhalter-Modus lebt in der Engine** (`ed-core`, `editor/snippet.rs`): Parser, Einfügen, Tab-Sprünge
  und das Nachführen der Platzhalter – wie alle Textlogik (002 E13). Die UI schickt nur Befehle; die Desktop-App
  bekommt den Befehl mit (kein UI dafür, Verhalten unverändert).
- **E2 Positionen wandern über ein Änderungsprotokoll**: `Document::track_edits(on)` / `take_edits()` liefern je
  Bearbeitung die sortierten Änderungen `(from, to, eingefügte Zeichen)` bzw. bei „Zeilenenden umwandeln“ den Text
  davor (`EditBatch::Rewrite`, Zeile/Spalte bleiben). Protokolliert wird **nur**, solange jemand folgt (Snippet
  aktiv, später Markierungen 029) – sonst kostet es nichts. `editor/track.rs`: `EditMap::map(p, Bias)` (binäre Suche
  über Präfixsummen) und `remap_lines`. Undo/Redo laufen durch dieselbe Stelle (`apply_raw`) und werden mitverfolgt.
- **E3 Syntax** (TextMate/LSP-Teilmenge): `$1`, `${1}`, `${1:Vorgabe}` (auch verschachtelt), `${1|a,b|}` (erste
  Option), `$0`, `$NAME`, `${NAME:Vorgabe}`; `\$`, `\}`, `\\` sind die Zeichen selbst, jeder andere Backslash bleibt.
  Unvollständiges (`${1:abc`, `100$`) ist Text. Gleiche Nummer = ein Platzhalter an mehreren Stellen; die erste
  Stelle mit Vorgabe bestimmt den Text aller. Reihenfolge 1, 2, … und `$0` zuletzt; ohne `$0` das Ende.
- **E4 Einfügen** (`Command::InsertSnippet { template, from?, to?, vars }`, `is_edit`): ersetzt den Bereich
  (Vorgabe: Hauptauswahl) als **eine** Änderung = ein Undo-Schritt. Zeilenumbrüche der Vorlage bekommen das
  Zeilenende des Dokuments plus die Einrückung der Zeile, Tabs die Einrückungseinheit; Werte von Variablen behalten
  ihre Form (nur Zeilenenden angeglichen). Ohne Platzhalter: Text, Caret dahinter, kein Modus.
- **E5 Modus**: der aktuelle Platzhalter ist als Auswahl gesetzt – mehrere Stellen als Mehrfach-Cursor, also wird
  gemeinsam getippt. `Indent`/`Outdent` (Tab/Shift+Tab) springen, solange der Modus läuft; der letzte Halt (`$0`)
  beendet ihn. Beim Nachführen wächst der aktuelle Platzhalter (und was ihn enthält) mit Text an seinen Rändern, die
  anderen werden geschoben – `${1:a}${2:b}` bleibt getrennt, `${1:f(${2:x})}` wächst mit. Der Modus endet, wenn der
  Hauptcursor den Platzhalter verlässt, bei `SingleCursor` (Esc; `Frame.snippet` schaltet die Aktion frei) und wenn
  Undo den Textzustand vor dem Einfügen erreicht (`Document::state()`). Undo/Redo **innerhalb** lassen ihn bestehen:
  die Platzhalter sind mitgewandert, die Auswahl kommt aus dem Undo-Schritt.
- **E6 Variablen**: von der Seite (`vars`), in der Engine `SELECTION`/`TM_SELECTED_TEXT`; Datum und Zeit setzt die
  Fassade (`dateVars()`: `DATE`, `TIME`, `CURRENT_YEAR|MONTH|DATE|HOUR|MINUTE|SECOND`) – `ed-core` hat in
  WebAssembly keine Uhr. Unbekannte Variable: ihre Vorgabe, sonst nichts.
- **E7 Kürzel** (`el.snippets = [{ prefix, body, description? }]`, nur Property; `body` auch als Zeilen-Array):
  erscheinen in der Vorschlagsliste (Art „snippet“, 028 E5) und **Tab direkt hinter dem Kürzel** expandiert: die
  Ansicht fragt bei `indent` – nur wenn es Kürzel gibt und kein Snippet läuft – `tabCommand()` in der Reihenfolge
  der Befehle; kein Treffer → normales Tab. Ist die Vorschlagsliste offen, gehört Tab ihr (wie bisher).

## Verträge

```ts
insertSnippet(template: string, options?: { range?: { start, end }; vars?: Record<string, string> } | { start, end })
snippets: { prefix: string; body: string | string[]; description?: string }[] | null
```

Rust `Command::InsertSnippet` ↔ TS `{ type: 'insertSnippet'; template; from?; to?; vars? }`; `Frame.snippet`.

## Tests

- `snippet.rs`: Platzhalter/Vorgaben/`$0`, gleiche Nummer, maskierte `$`/`}`/`\`, Unvollständiges, Verschachtelung,
  Auswahl-Listen, Variablen, Textfilter (Einrückung, CRLF, Emoji).
- `track.rs`: Einfügen/Löschen/Ersetzen, Bias, Berühren, Zeilenenden-Umwandlung.
- `editor/tests.rs`: Akzeptanzfall nur mit Tab und Tippen; drei Stellen gemeinsam; ein Undo-Schritt, Modus übersteht
  Undo/Redo und endet vor dem Einfügen; Esc/Verlassen; Einrückung, Variablen, Auswahl, Bereich; benachbarte und
  verschachtelte Platzhalter.
- Vitest: `insertSnippet` (Variablen, Bereich), Tab hinter Kürzel → `insertSnippet`, sonst `indent`.
- Ende-zu-Ende (10.11.2026, eingebauter Browser, `site/embed/` Editor C): `sel` + Tab, `id`, Tab, `t`, Tab, `x>1`,
  Tab → `SELECT id FROM t WHERE x>1;`; drei Undo nehmen das Getippte zurück, das vierte den Baustein (ein Schritt).

## Offen / bewusst nicht

- Die übrigen Platzhalter werden nicht eigens gezeichnet (nur der aktuelle ist ausgewählt).
- Auswahl-Listen `${1|a,b|}` setzen die erste Option ein (keine Liste).
