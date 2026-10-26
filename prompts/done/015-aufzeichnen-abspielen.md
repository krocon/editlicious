# Prompt 015: Aufzeichnen und Abspielen (Recorder/Player, Skripte)

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 005, 007, 013.

## Ziel

Bearbeitungen lassen sich als Befehlsfolge mit Zeitstempeln aufzeichnen und reproduzierbar wieder abspielen –
im Originaltempo oder mit „menschlichem“ Tipp-Rhythmus. Hauptzweck: identische, saubere Live-Coding-Sequenzen für
Lernvideos (DE und EN); Nebeneffekte: Makros, reproduzierbare Demos, Testfälle.

## Umfang

- **Aufzeichnung in `ed-core`** (`src/editor/recording.rs` o. ä.): jeder Engine-Befehl einer Sitzung wird mit
  relativem Zeitstempel (ms) protokolliert, solange die Aufnahme läuft. Gespeichert werden Befehle, nicht
  Tastendrücke und nicht Frames. IME-Kompositionen nur mit ihrem Ergebnis.
- **Format** `*.edrec.jsonl`: erste Zeile Kopf (Format-Version, App-Version, Ausgangszustand: Dateiname, Hash
  des Inhalts, Cursor, Einstellungen wie Tabweite/Umbruch), danach je Zeile `{ t, cmd, args }`. Versioniert, mit
  nachsichtigem Laden (unbekannte Befehle → klarer Fehler mit Zeilennummer, kein Absturz).
- **Abspielen** mit Tempo-Modi: `original`, `faktor` (0,25×–8×), `gleichmäßig` (feste ms/Zeichen), `menschlich`
  (Jitter je Zeichen, längere Pausen nach Satzzeichen/Zeilenenden, reproduzierbar über Seed). Pause, Weiter, Stopp,
  Einzelschritt. Abspielen läuft als lange Arbeit mit Fortschritt über `tauri::ipc::Channel` und ist abbrechbar.
  Eine Abspielung ist **eine** Undo-Gruppe (optional: je Skript-Schritt).
- **Tipp-Skript** `*.edscript` (für Videos von Hand geschrieben, Format im Design festlegen – YAML oder einfache
  Zeilen-DSL): `type "…"` (mehrzeilig), `pause 800`, `goto 12:5`, `select …`, `action <Aktions-Id>`, `find "…"`,
  `tempo menschlich seed=42`, optionale `typo`-Anweisung (Tippfehler + Korrektur). Das Skript wird in Rust zu
  einer Befehlsfolge übersetzt und wie eine Aufzeichnung abgespielt. Text aus Datei referenzierbar
  (`type @snippets/beispiel.ts`), damit DE/EN-Varianten dieselbe Choreografie teilen.
- **Aufzeichnung → Skript** exportieren (Zeichenfolgen zu `type`-Blöcken zusammenfassen, Pausen > Schwelle als
  `pause`), damit eine eingetippte Sequenz nachbearbeitet werden kann.
- **Aktionen** (Katalog, Befehlspalette, Menü „Bearbeiten → Aufzeichnung“): Aufnahme starten/stoppen, Aufnahme
  speichern, Aufzeichnung/Skript abspielen, Abspielen pausieren/stoppen, letzte Aufnahme als Makro wiederholen.
  Kürzel nur für Start/Stopp und Wiederholen (Vorschlag im Design, mit legacy abgleichen).
- **Sichtbarkeit**: nur ein dezenter Hinweis in der Statusleiste („● Aufnahme“, „▶ Wiedergabe 37 %“), sonst keine
  neuen Bedienelemente. Optional: Statusleiste während der Wiedergabe ausblendbar (für saubere Videos).
- **Vorbereitung für 016/017**: die Abspiel-API ist eine reine Rust-Schnittstelle (Befehlsquelle → Sitzung), die
  später auch der MCP-Server und der Video-Renderer benutzen.

## Regeln und Randfälle

- Abspielen vergleicht den Ausgangszustand (Hash) und warnt bei Abweichung; „trotzdem abspielen“ möglich.
- Während der Wiedergabe sind Nutzereingaben im Editor gesperrt (Esc bricht ab); andere Tabs bleiben bedienbar.
- Große Dateien: Aufzeichnung wächst mit der Zahl der Befehle, nicht mit der Datei; eingefügte Zwischenablage-
  Inhalte > Grenzwert werden nicht inline, sondern als Verweis/Hash gespeichert und beim Abspielen abgelehnt
  (klarer Fehler). Nichts Dateigroßes über IPC.
- Dateiinhalte nie loggen; Aufzeichnungen enthalten Text – das im Speichern-Dialog erwähnen.

## Tests

- Rust: Aufnahme → Abspielen ergibt byte-identischen Text und gleiche Carets (datengetrieben, inkl. Emoji, CJK,
  kombinierender Zeichen, Blockauswahl, mehrere Zeilenenden); Seed macht `menschlich` deterministisch; Skript-
  Parser mit Fehlerfällen; Export Aufnahme → Skript → Abspielen ist äquivalent; Abbruch mitten in der Wiedergabe
  lässt ein konsistentes Dokument und eine Undo-Gruppe zurück.
- Angular: Aktionen, Statusanzeige, Eingabesperre, Abbruch per Esc (Fakes).
- Ende-zu-Ende: Skript mit ~30 s Tippen in der echten App abspielen, Screenshots vor/nach.

## Akzeptanzkriterien

- Ein Skript für ein 60-Sekunden-Video läuft zweimal hintereinander pixelgleich ab (gleiches Ergebnis, gleicher
  Seed → gleiches Timing).
- Eine von Hand getippte Aufnahme lässt sich als Skript exportieren, editieren und wieder abspielen.
