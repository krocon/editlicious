# Design 021: Weitere Farbschemata und Farbschema-Auswahl

> Prompt: [021-more-themes.md](021-more-themes.md). Stand 26.10.2026.

## Entscheidungen

- **E1 Neun Schemata, „inspiriert von“** – eigene Paletten nach dem Charakter der Vorbilder, Namen ohne fremde
  Marken (die Seiten der Vorbilder waren aus der Sitzung nicht abrufbar; bekannte Paletten wie gruvbox/One Dark/
  Moonlight nach ihren öffentlichen Farbwerten, die übrigen nach Name und Beschreibung):

  | Id | Name (en / de) | Vorbild | Grund |
  |---|---|---|---|
  | `glass` | Glass / Glas | Liquid Glass light | hell |
  | `harvest` | Harvest / Ernte | gruvbox light hard | hell |
  | `chalk` | Chalk / Kreide | Chalkdraw paper | hell |
  | `one` | One | One Dark | dunkel |
  | `moss` | Moss / Moos | gruvbox material dark | dunkel |
  | `mocha` | Mocha / Mokka | Cozy Latte dark | dunkel |
  | `moon` | Moon / Mond | Moonlight | dunkel |
  | `graphite` | Graphite / Graphit | Graphite Pop | dunkel |
  | `amber` | Amber / Bernstein | Russian Hacker amber | dunkel |

  Jedes Schema setzt Chrome (`--color-*`), Editor (`--ed-*`) und Syntax (`--syn-*`) und nennt seine Basis
  `--scheme-base: light|dark` (Video-Export, 017).
- **E2 Reihenfolge hell → dunkel** nach der relativen Leuchtdichte des Editor-Hintergrunds: Papier, Glas, Sand,
  Ernte, Kreide, One, Moos, Mokka, Mond, Nacht, Graphit, Tinte, Bernstein; „wie das System“ steht vorn. Eine Liste
  (`SCHEMES` in `settings.models.ts`, Rust `settings::SCHEMES`, `video/theme.rs ORDER`), der Rust-Test prüft die
  Sortierung gegen `styles.css`, der TS-Test dieselbe Liste. F12 läuft in dieser Reihenfolge.
- **E3 Lesbarkeit** (Test `every_scheme_is_readable`): Text ≥ 7 : 1, gedämpfte Chrome-Schrift ≥ 4 : 1, Caret ≥ 3 : 1;
  Syntax der neuen Schemata ≥ 4,5 : 1 (Kommentar, Satzzeichen, Debug-Zeilen ≥ 3 : 1). Dafür etwas angepasst:
  One-Text `#bbc1cc` statt `#abb2bf`, gruvbox-Gelb/-Grün dunkler, Kreide-Kommentare dunkler. Marker (019 E5) bleiben
  ≥ 4,5 : 1 – in One und Moos mit etwas schwächeren Markern 2/3/5/6. Ältere Paletten behalten ihren Charakter
  (Sand ≈ Solarized liegt um 3 : 1).
- **E4 Vorschaubilder ohne zweite Farbquelle:** `styles.css` bleibt die einzige Stelle für Farben. Die Blöcke
  gelten zusätzlich für `[data-preview]` (helle Basis), `[data-preview][data-dark]` (dunkle Basis) und
  `[data-preview][data-preview='x']` (Schema) – so zeichnet ein Element ein beliebiges Schema, unabhängig vom
  aktiven. Der Video-Parser liest Selektorlisten.
- **E5 Farbschema-Auswahl** (`view.schemePicker`, ⇧F12, Menü Ansicht › Farbschema, Palette): Dialog mit Raster
  aus Vorschaubildern (Mini-Code mit Zeilennummern, Auswahl, aktueller Zeile, Caret), APG-Radiogruppe mit
  wanderndem Tabstopp: Pfeile/Pos1/Ende probieren sofort aus, Enter/Leertaste/OK/Doppelklick behalten, Esc/
  Abbrechen kehren zum vorigen Schema zurück. „Wie das System“ zeigt Papier und Tinte nebeneinander.

## Tests

- Rust (`ed-video`): Reihenfolge und Basis aller Schemata, Lesbarkeit, Marker-Kontrast, Namen aus Selektorlisten;
  Einstellungen begrenzen auf `SCHEMES`.
- Angular: Reihenfolge/Attribute (`chrome.store.spec`), Auswahl-Dialog (Raster, Vorschau-Attribute, Tabstopp,
  Pfeile, Enter, Klick, Esc zurück).
- Ende-zu-Ende: Dialog mit allen Vorschaubildern, Mond, Bernstein, Ernte auf einer Rust-Datei.
