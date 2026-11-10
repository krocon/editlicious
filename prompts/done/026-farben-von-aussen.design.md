# Design 026: Farben von außen

> Prompt: [026-farben-von-aussen.md](026-farben-von-aussen.md) (schema-commander E1). Stand 10.11.2026.

## Bestand (vor 026)

Der Editor zeichnet **nur DOM** (kein Canvas); jede Farbe kommt aus einer CSS-Variable, die `host-theme.css`
(generiert aus `src/styles.css`) auf `:host` setzt. Regeln der Seite, die das Element treffen, gewinnen über `:host`.

| Teil | Variable(n) | von außen (am Element)? |
|---|---|---|
| Textfläche, Text, Gutter, aktuelle Zeile, Auswahl, Caret, Einrückungslinien, Tab-Pfeile | `--ed-bg`, `--ed-text`, `--ed-gutter-*`, `--ed-current-line`, `--ed-selection[-inactive]`, `--ed-caret`, `--ed-indent-guide`, `--ed-whitespace` | ja |
| Treffer, Scrollbalken | `--ed-search-match`, `--ed-search-current`, `--ed-scrollbar[-hover]` | ja |
| Syntaxklassen (23) | `--syn-*` | ja |
| Wort-Marker (10) | `--ed-marker-0…9` | ja |
| Suchleiste, Gehe-zu, Vorschlagsliste, Rechen-Hinweis, Statuszeile, Auswahldialog | `--color-*`, `--shadow-popup` | ja, aber undokumentiert |
| Steuerzeichen | `--color-warning` (geteilt mit Warnungen) | nur zusammen mit Warnungen |
| Striche der Übersichtsleiste | `--ed-search-current`, `--ed-marker-0` (geteilt) | nur zusammen mit Treffer/Marker |
| Schachbrett hinter transparenten Farbfeldern | fest `#c8c8c8`/`#ffffff` | **nein** |
| Rand der Farbfelder | `--ed-gutter-text` (geteilt) | nur zusammen mit Zeilennummern |
| Schrift | `--font-editor`, `--editor-font-size` | **nein**: die Fassade setzt beide als Inline-Style am Host (README behauptete das Gegenteil) |
| Variablen auf `<html>` | – | **nein**: `:host` definiert jede Variable selbst, Vererbung endet dort |
| „Als HTML kopieren“: Farbe der Zeilennummern | las `--ed-gutter-text` von `<html>` | Fehler im eingebetteten Editor (immer grau) |
| Klammerpaar | – | gibt es im Editor nicht (kein Klammer-Hervorheben) |

## Entscheidungen

- **E1 Die Namen bleiben** `--ed-*`, `--syn-*`, `--color-*`, `--shadow-popup` – sie wirken seit 025 am Element und
  sind in der README genannt; ein zweiter, „öffentlicher“ Namensraum mit Umleitung hätte jede der 13 Schema-Regeln
  verdoppelt. **Vertrag:** am Element gesetzt (direkt, per Regel oder als Abbildung `--ed-bg: var(--app-bg)`) gewinnt
  die Seite über das `scheme`; nicht Gesetztes kommt aus dem Schema. Variablen nur auf `<html>` erreichen den Editor
  nicht – die Abbildungsregel löst das und folgt jeder Änderung auf `<html>` sofort (CSS rechnet `var()` am Element
  neu). Schema Commander braucht die Abbildung ohnehin, weil seine Variablen anders heißen.
  `--font-editor` wird nur noch gesetzt, wenn `font-family` angegeben ist – sonst gilt die Variable der Seite.
- **E2 Lückenschluss mit neuen Tokens** in `src/styles.css` (nur im ersten Block, als Verweis auf den bisherigen
  Wert – App und Schemata unverändert): `--ed-control`, `--ed-overview-match`, `--ed-overview-marker`,
  `--ed-swatch-border`, `--ed-checker-light`, `--ed-checker-dark`. Damit steht in keiner Komponenten-CSS mehr eine
  feste Farbe. `ed-video` liest nur seine bekannten Schlüssel – unberührt.
- **E3 `scheme="none"`**: nicht in `SCHEMES` (Einstellungen der App), sondern Typ `EmbedScheme = Scheme | 'none'`
  der Fassade. Host: `data-scheme="none"`; Block `:host([data-scheme='none'][data-scheme])` in
  `editlicious-editor.css` (handgeschrieben, nach `host-theme.css`, doppeltes Attribut = höhere Spezifität als die
  Schema-Regeln) ersetzt **jedes** Token, das ein Schema setzt: Basis `--ed-bg: Canvas`, `--ed-text: CanvasText`,
  alles andere daraus per `color-mix()` abgeleitet (Syntax = Textfarbe, Kommentar gedämpft, Akzent `Highlight`).
  Eine Seite, die nur zwei Variablen setzt, bekommt so einen stimmigen einfarbigen Editor; keine Farbe eines
  eingebauten Schemas scheint durch. Marker, Schrift und Größen bleiben aus dem Grundblock.
  **`color-scheme="light|dark"`** (Property `colorScheme`) setzt `data-theme` und damit CSS `color-scheme`
  (Systemfarben, Formularelemente); ohne Angabe `light dark` (System). Wirkt nur mit `none` – die eingebauten
  Schemata bringen ihr Hell/Dunkel mit.
- **E4 Laufzeit**: nichts zu tun – reine CSS-Variablen, kein Neuaufbau, keine Canvas-Teile. „Als HTML kopieren“ liest
  die Gutter-Farbe jetzt vom Host der Ansicht statt von `<html>`.
- **E5 Vertragstest** `scripts/theme-vars.test.mjs` (läuft in `pnpm scripts:test`): README-Tabelle = Farb-Tokens des
  ersten Blocks von `styles.css`; Komponenten-CSS benutzt nur deklarierte Tokens und keine eigene Farbe; jedes Token
  wird benutzt; der `none`-Block deckt alle Tokens ab, die ein Schema setzt.

## Verträge

- Attribut `scheme`: bisherige Werte + `none`. Attribut `color-scheme` / Property `colorScheme`: `light`, `dark`,
  sonst `null`.
- README „Styling“: Tabelle aller Farbvariablen (Bedeutung, Beispiel), Abschnitt „Your own colour schemes“.
- Beispielseite `site/embed/`: dritter Editor C (`scheme="none"`), gefärbt über Variablen der Seite; zwei Knöpfe
  schalten das Seiten-Thema auf `<html>` um.

## Tests

- `theme-vars.test.mjs` (E5), `host-theme.test.mjs` (generierte Datei aktuell).
- `editlicious-editor.spec.ts`: `none` → `data-scheme="none"`, `colorScheme` → `data-theme`; eingebautes Schema
  ignoriert `colorScheme`; `--font-editor` nicht gesetzt ohne `font-family`.
- `embed-e2e.mjs`: C nimmt Hintergrund und Schlüsselwortfarbe von der Seite, folgt dem Themawechsel auf `<html>`;
  Variable am Element gewinnt über eingebautes Schema.
- Ende-zu-Ende (10.11.2026, eingebauter Browser, `site/embed/`): C `rgb(253, 246, 227)` → nach Themawechsel
  `rgb(0, 43, 54)`, Schlüsselwort `rgb(133, 153, 0)` → `rgb(203, 75, 22)`; A mit `--syn-keyword` am Element
  `rgb(1, 2, 3)`. App/Schemata: bestehende Tests und `ed-video`-Golden-Images unverändert grün.

## Offen / bewusst nicht

- Kein Klammerpaar-Hervorheben (gibt es nicht); Farben der Markierungen von außen kommen mit 029.
