# 024 – Pfad öffnen

**Auftrag (Marc, 31.10.2026):** „Ich möchte einen weiteren Öffnen-Dialog, bei dem man direkt ein File (z. B.
`~/Library/Application Support/Claude/claude_desktop_config.json`) angeben kann.“

## Entscheidungen

- **E1 Aktion statt Knopf:** `file.openPath` „Open path…“ / „Pfad öffnen …“, Kürzel **Mod+Shift+O**, im Menü
  „Datei“ direkt unter „Datei öffnen …“, in der Befehlspalette. Kein neues sichtbares Bedienelement.
- **E2 Rust deutet den Pfad** (`src-tauri/src/path_input.rs`, rein, ohne Tauri; Pfade bleiben im Frontend opak):
  Leerraum und umschließende Anführungszeichen weg (Finder „Als Pfadname kopieren“, Windows „Als Pfad kopieren“),
  `file://`-URLs mit `%20` usw., `~` und `~/…` → Home-Ordner (`~bob` nicht), relative Pfade gegen den Ordner der
  aktiven Datei (sonst Home), `.`/`..` rein textuell auflösen (Symlinks bleiben wie getippt – wie beim nativen
  Dialog, damit eine schon offene Datei wiedererkannt wird). Fehler mit vorhandenen Codes (`not_found`,
  `is_a_directory`, `not_a_file`, `invalid_path`); der Dialog bleibt offen und zeigt den Grund.
- **E3 Startwert:** Ordner der aktiven Datei (unter Home als `~/…`), sonst `~/`; markiert, sodass Tippen oder
  Einfügen ihn ersetzt, → bzw. Ende setzt ihn fort.
- **E4 Ergänzen wie in der Shell:** zu jeder Eingabe die Einträge des Ordners, deren Name mit dem getippten Rest
  beginnt (Groß/klein egal, versteckte nur nach `.`), höchstens 200 (`MAX_ENTRIES`, dann Hinweis „weitertippen“).
  Tab übernimmt den gewählten Eintrag oder das gemeinsame Anfangsstück (ein einzelner Ordner bekommt den Trenner),
  ↑/↓ wählen, Enter auf einem Ordner setzt ihn ins Feld, auf einer Datei öffnet sie; ohne Auswahl öffnet Enter den
  getippten Pfad. Antworten zu älteren Eingaben werden verworfen (Sequenz). Fehler beim Lesen eines Ordners geben
  einfach keine Einträge.
- **E5 Barrierefreiheit:** APG-Combobox mit Listbox wie `PickDialog`, `aria-activedescendant`, Fehler als
  `role="alert"` + `aria-invalid`, Hinweiszeile per `aria-describedby`.

## Tests

- Rust `path_input::tests`: `~`, Anführungszeichen, `file://` mit Umlauten, relativ und `..`, Fehlercodes,
  Ergänzung (Groß/klein, einzelner Ordner, versteckte, Grenze, fehlender Ordner), Zeichengrenzen, Startwert.
- Angular `open-path-dialog.spec.ts`: Startwert markiert, Öffnen per Enter, Tab/↓/Enter/Klick, Fehlermeldung,
  veraltete Antworten, Tab ohne Ergänzung.
