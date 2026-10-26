# Prompt 016: MCP-Server – KI steuert den Editor

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 015.

## Ziel

Eine KI (Claude Code, Claude Desktop o. ä.) kann einen **laufenden** editlicious über das Model Context Protocol
steuern: Dateien öffnen, lesen (bereichsweise), tippen (auch im Video-Tempo aus 015), Aktionen ausführen und
Aufnahmen starten/stoppen. Die KI benutzt dieselben Engine-Befehle wie die UI – keine simulierten Tastendrücke.

## Umfang

- **Architektur** (im Design entscheiden, Empfehlung): eigenes kleines Binary `editlicious-mcp` (Rust, Crate
  `rmcp` oder gleichwertig, Begründung in `docs/architecture.md`), MCP über **stdio**; es verbindet sich über einen
  lokalen Socket (Unix-Socket bzw. Named Pipe unter Windows) mit der laufenden App. Alternative „Streamable HTTP
  auf 127.0.0.1 direkt in der App“ bewerten. Die App startet den Socket nur, wenn die Funktion aktiviert ist.
- **Tools** (Namen und Schemas im Design festlegen):
  - `list_documents`, `open_document(path)`, `new_document`, `save_document(id)`, `activate_document(id)`
  - `get_state(id)` (Cursor, Auswahl, Zeilenzahl, Encoding, Revision, Sprache)
  - `read_range(id, from_line, to_line, max_chars)` – hart begrenzt, nie die ganze Datei
  - `find(id, pattern, options, max_hits)`, `goto(id, line, col)`, `select(id, …)`
  - `type_text(id, text, tempo?, seed?)`, `replace_selection(id, text)`, `run_action(id, action_id)` (nur Aktionen
    aus dem `action-catalog`, Liste über `list_actions`)
  - `play_script(id, script)`, `start_recording`, `stop_recording(path?)` (aus 015)
- **Resources** optional: `editlicious://document/{id}/state`.
- **Nebenläufigkeit**: jeder schreibende Aufruf trägt die erwartete Revision; passt sie nicht (Nutzer hat
  dazwischen getippt) → Fehler `revision_conflict` mit aktuellem Zustand. Jeder MCP-Aufruf ist eine Undo-Gruppe.
- **Fehler** als `FsError`-Codes (neue Codes in `error.rs`, `fs-error.ts`, `describeFsError()`), im MCP als
  Tool-Fehler mit Code + kurzer englischer Meldung.
- **Einstellungen**: „KI-Steuerung (MCP) erlauben“ – standardmäßig **aus**; Anzeige der Konfig-Zeile für Claude
  Code/Desktop zum Kopieren (`claude mcp add editlicious -- <pfad>/editlicious-mcp`).
- **Sichtbarkeit**: Statusleiste zeigt dezent „KI verbunden“ bzw. „KI tippt …“; Esc/Aktion „KI-Steuerung trennen“
  beendet laufende Aufrufe.
- **Distribution**: Sidecar-Binary in die Pakete aus 012 aufnehmen (Tauri `externalBin`), signiert wie die App.

## Sicherheit

- Socket nur für den aktuellen Nutzer (Dateirechte 0600 bzw. Pipe-ACL), zusätzlich Zufalls-Token je App-Start
  (in einer nur für den Nutzer lesbaren Datei im Konfigverzeichnis).
- Kein Shell-, kein beliebiger Dateisystemzugriff: `open_document` nutzt denselben Ladeweg wie der Dialog;
  Speichern nur auf bereits geöffnete Dokumente oder mit ausdrücklichem `save_as`-Pfad, optional mit Rückfrage im UI
  („KI möchte speichern unter … – erlauben?“, Einstellung).
- Ausgaben begrenzt (`max_chars`, `max_hits`), damit auch eine 1-GB-Datei nie ganz über IPC oder MCP geht.
- Dateiinhalte nicht loggen, auch nicht im MCP-Binary.

## Tests

- Rust: Protokoll-Schicht App ↔ Sidecar (Nachrichten, Token, Revision-Konflikt, Grenzen), Tool-Schemas als
  Snapshot, `read_range`-Begrenzung mit großer Datei (ignored-Test).
- MCP-Integration: Sidecar gegen eine kopflose Engine-Instanz starten und Tools über stdio aufrufen
  (offizieller MCP-Inspector oder eigener Test-Client).
- Angular: Einstellung, Statusanzeige, Trennen.
- Ende-zu-Ende: echte App im Container, Claude-Code-ähnlicher Client tippt per `type_text` mit Tempo `menschlich`.

## Akzeptanzkriterien

- Mit `claude mcp add …` kann Claude Code eine Datei öffnen, ein Beispiel im Video-Tempo eintippen und speichern;
  das Ergebnis ist mit Strg+Z in einem Schritt rückgängig zu machen.
- Ist die Einstellung aus, ist kein Socket offen und das Sidecar meldet einen klaren Fehler.
