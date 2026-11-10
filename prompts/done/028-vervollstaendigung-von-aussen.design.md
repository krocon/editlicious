# Design 028: Vervollständigung von außen

> Prompt: [028-vervollstaendigung-von-aussen.md](028-vervollstaendigung-von-aussen.md) (schema-commander E3).
> Stand 10.11.2026. Braucht 027 (`replaceRange`) und 030 (Snippets).

## Entscheidungen

- **E1 Die Vorschläge der Seite leben in der UI-Schicht**, nicht in der Engine: sie kommen als JS-Objekte von einer
  JS-Funktion, bis zu Zehntausende – sie in die Engine zu schieben und je Frame zurückzuholen wäre genau der
  Verkehr über die Grenze, den 002 E13 verbietet. `ProviderCompletion` (`src/app/embed/provider-completion.ts`, je
  Editor-Instanz) fragt, merkt sich die Antwort, filtert und fügt ein. Die Engine bleibt für den **Text**
  zuständig: Kontext am Cursor, Wortgrenzen, Einfügen. Die Editor-Ansicht kennt nur die Schnittstelle
  `ExternalCompletion` (Token `EDITOR_COMPLETION`, in der App `null` → Desktop unverändert).
- **E2 Kontext aus der Engine**: `Session::completion_context(words)` → Zeile/Spalte, `single` (ein Caret ohne
  Auswahl), die Zeile bis zum Cursor (höchstens die letzten 2000 Zeichen – auch in einer 10-Mio.-Zeichen-Zeile),
  Beginn des angefangenen Worts (Wortzeichen der Sprache, z. B. `-` in CSS) samt Zelle, Revision und – auf Wunsch –
  die Kandidaten der Wort-Vervollständigung. Der Anbieter bekommt daraus `position`, `lineText`, `word`,
  `wordStart`, `trigger`, `triggerCharacter`, `signal` und `getRange()`/`getSlice()` für mehr Text **auf Anfrage**.
- **E3 Vorschlag**: `label`, `insertText`, `snippet` (Platzhalter, 030), `kind` (frei), `detail`, `documentation`,
  `sortText`, `filterText`, `range`. Antwort: Array = vollständig; `{ items, incomplete: true }` = bei jedem
  Tastendruck neu fragen; `null` = nichts.
- **E4 Eine Liste**: die Ansicht zeichnet die Vorschläge der Seite in **ihrer** Vorschlagsliste (`role="listbox"`,
  `aria-activedescendant` am Eingabefeld, dazu `aria-setsize`/`aria-posinset`), Kontext `completion` der Tastatur
  wie bisher (↑ ↓ Enter Tab Esc). Auf dem Schirm stehen **zehn Zeilen** (`COMPLETION_ROWS`), der Rest rollt mit
  den Pfeilen oder dem Mausrad; „n / gesamt“ darunter. Die Dokumentation des aktiven Eintrags steht unter der Liste
  und ist per `aria-describedby` angebunden. Kein Platz unter der Zeile → Liste darüber. Die eigene Liste der
  Engine (Rechnen `255/8=`) hat Vorrang, solange sie offen ist. `kind` wird zum Buchstaben-Icon; die Seite färbt es
  über `::part(icon-<kind>)` / `::part(item-<kind>)` (kein SQL-Wissen, keine festen Arten im Editor).
- **E5 Auslöser und Quellen**: `Strg+Leertaste` (`edit.complete`; steht ein Rechenausdruck am Caret, bleibt es die
  Engine), Auslösezeichen des Anbieters (`provider.triggerCharacters`), und beim Tippen ab zwei Zeichen eines Worts,
  wenn `word-suggest` an ist – dann fragt der Anbieter statt der Engine (`configure.words.auto = false`).
  `completion-sources`: `both` (Vorgabe: Anbieter, dann Kürzel der Seite, dann Wörter des Texts ohne Doppelte),
  `provider`, `words`. Ohne Anbieter und ohne Kürzel ist `on() = false` – alles wie vorher.
- **E6 Nichts blockiert das Tippen**: Befehle warten nie auf den Anbieter. Jede Änderung von Text oder Caret zählt
  `seq` hoch und bricht das `AbortSignal` ab; eine Antwort mit altem `seq` wird verworfen (kein Flackern, keine
  veraltete Liste). Vollständige Antwort + Weitertippen im selben Wort = nur filtern (ein Durchlauf: Präfix-Treffer
  vor Enthält-Treffern, Reihenfolge bleibt; 50 000 Einträge ≈ 1 ms), keine neue Anfrage. Verlässt das Tippen das
  Wort, schließt die Liste. Fehler des Anbieters: `console.error`, keine Liste.
- **E7 Einfügen**: in der Befehlsreihenfolge der Ansicht (`enqueue`) wird der Kontext frisch gelesen – was vor der
  Wahl noch getippt wurde, gehört zum Wort –, dann `replaceRange { caret: true }` bzw. `insertSnippet`. Ein
  Undo-Schritt.

## Verträge

```ts
el.completionProvider = async (ctx: CompletionContext) => CompletionItem[] | { items, incomplete? } | null
el.completionProvider.triggerCharacters = ['.']        // oder { provideCompletions, triggerCharacters }
el.completionSources = 'both' | 'provider' | 'words'   // Attribut completion-sources
```

Rust `model::CompletionContext` ↔ TS `EngineCompletionContext`; `ed-wasm` `Editor.completionContext(words)`;
Engine-Host-Op `completionContext`.

## Tests

- Rust: Kontext (Wortbeginn in Codepunkten, Kandidaten, Auswahl/mehrere Cursor), riesige Zeile bleibt klein.
- Vitest `provider-completion.spec.ts`: Kontext für den Anbieter; Filtern ohne neue Anfrage; unvollständig → jedes
  Mal fragen; Tippen/Auslösezeichen; verspätete und abgebrochene Antwort, Fehler; Einfügen (Wort bis Caret,
  Snippet, eigener Bereich); Quellen mischen; Tab-Kürzel; 50 000 Einträge (< 60 ms je Taste im Test, Filter
  < 16 ms); Arten-Namen.
- `editlicious-editor.spec.ts`: Liste im Shadow Root, `part`-Namen, ARIA, Tasten, `words.auto` aus bei Anbieter.
- Ende-zu-Ende (10.11.2026, eingebauter Browser, `site/embed/` Editor C, 50 000 Einträge): neun Tasten flüssig,
  Liste zeigt zehn von 400 Treffern, ↓ + Enter setzt `orders_0005` ein; Anbieter mit 2 s: Tippen läuft, nach
  Cursorbewegung erscheint seine Antwort nicht mehr. Dieselben Schritte in `scripts/embed-e2e.mjs`.

## Offen / bewusst nicht

- Kein unscharfes (Teilfolgen-)Filtern, keine Seiten-Tasten in der Liste; Dokumentation ist reiner Text.
- Läuft ein Snippet und die Liste ist offen, nimmt Tab den Vorschlag (wie in anderen Editoren).
