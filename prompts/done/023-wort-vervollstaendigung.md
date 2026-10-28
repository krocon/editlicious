# Prompt 023: Wort-Vervollständigung und Sprach-Lexikon

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 008, 009, 020. Legacy: `AutoCompleteAction`
> „Auto Completion“ (Ctrl+Space, legacy §2; Todo „erst ab 2 Zeichen“, §4), alte Webseite: „Autovervollständigung
> für diverse Grammatiken“ (§10). Folge-Idee aus `docs/parity.md` Nr. 2.

## Ziel

Strg+Leertaste vervollständigt das Wort am Caret – aus den Wörtern des Dokuments rund um den Caret und aus einem
kleinen Lexikon typischer Wörter der Sprache (Java, SQL, Python, JavaScript/TypeScript, Rust, C/C++, C#, Go, PHP,
Ruby, Shell, CSS, HTML). Grundlage ist die Vorschlagsliste aus 020 (Engine hält die Liste, UI zeichnet nur).

## Umfang

1. **Wortanbieter in Rust** (`ed-core`, rein, WASM-fähig): Präfix = Wortzeichen links vom Caret; Kandidaten aus
   den Zeilen um den Caret (nach Nähe, Budget an Zeichen – nie proportional zur Dateigröße), dann Lexikon der
   Sprache. Exakte Groß/klein-Treffer vor anderen, Wort am Caret selbst nicht, höchstens 12 Einträge.
2. **Lexikon**: Wortlisten je Sprache als Textdateien im Crate (`include_str!`), Zuordnung über den Sprachnamen
   der Syntax-Hervorhebung. SQL u. ä. ohne Groß/klein-Unterschied: Schreibweise folgt dem Präfix (`sel` →
   `select`, `SEL` → `SELECT`).
3. **Bedienung**: Strg+Leertaste (bzw. ⌥Esc) – Rechenausdruck am Caret hat Vorrang (020), sonst Wörter. Ein
   einziger Kandidat wird direkt eingesetzt. Bei offener Liste filtert Weitertippen/Rücktaste die Liste.
   Übernehmen ersetzt das Präfix, ein Undo-Schritt.
4. **Einstellungen**: „Wörter beim Tippen vorschlagen“ (ab 2 Zeichen, Standard aus – wenig Ablenkung),
   „Schlüsselwörter der Sprache vorschlagen“ (Standard an).
5. **UI**: Art-Bezeichnung „Wort“ / „Schlüsselwort“ in der Liste, Hinweis „Keine Vorschläge“.

## Tests

- Rust: Präfix, Rangfolge, Groß/klein, Unicode-Wörter, Wort am Caret ausgeschlossen, Lexikon je Sprache
  (Listen sauber: keine Dubletten, nur Wortzeichen), Budget bei großen Dateien und langen Zeilen, Sitzung:
  Öffnen, Filtern beim Tippen, Einzelkandidat, Auto-Modus, Undo, Aufnahme.
- Frontend: Art-Bezeichnungen, Einstellungen → `configure.words`, Texte vollständig.
- Ende-zu-Ende: Java-Datei, `pub` + Strg+Leertaste → Liste mit `public`; SQL `sel` → `select`.
