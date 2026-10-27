# 022 – Distribution: Design

> Prompt: [022-distribution.md](022-distribution.md) · Ergebnis: [`distribution/`](../../distribution/README.md)
> Vorlage: comic-commander-tauri `distribution/` (Prompt 014, gleicher Autor, gleiche Technik).

## Entscheidungen

- **E1 Ablage wie Comic Commander:** `distribution/` mit `browser-agent-prompt.md` (der Prompt), `facts.json`
  (einzige Faktenquelle), `listing.md` (Texte), `manifests/` je Plattform. Der Browser-Agent liest alles von `main`.
- **E2 Fakten nur aus dem Release:** alle sieben Assets von v1.0.0 geladen, SHA-256 nachgerechnet, Paketinhalt
  gelesen (deb-Control und Desktop-Datei, MSI-Property-Tabelle, NSIS-Inhalt, Info.plist). Nichts aus Comic Commander
  übernommen, ohne es bei editlicious zu prüfen.
- **E3 MCP als eigener Abschnitt:** `editlicious-mcp` ist kein eigenständiger Server (braucht die laufende App).
  Offizielle MCP Registry → blockiert (kein npm/PyPI/crates/OCI-Paket, kein `.mcpb`); Listen nur mit klarem Hinweis.
- **E4 Homebrew offiziell blockiert** (Gatekeeper-Pflicht seit 1.9.2026); Cask trotzdem fertig (eigener Tap nach
  Freigabe), mit `binary` für das Sidecar, damit `claude mcp add editlicious -- editlicious-mcp` reicht.
- **E5 Bekanntheitsschwellen** (Scoop Extras, awesome-rust > 50 Stars) bei einem neuen Repo realistisch als blockiert
  bewertet; Show HN mit der Live-Demo als Hebel für Sterne zuerst.
- **E6 Sandbox-Pakete (Flatpak, Snap):** ein Texteditor braucht breiten Dateizugriff – im Flatpak `--filesystem=home`
  mit Begründung, im Snap Hinweis auf strict vs. classic. Beide nur als lokal zu testende Entwürfe.
- **E7 Keine Änderungen an Release, Tags, Workflows oder App** in diesem Prompt; Befunde (fehlendes
  `SHA256SUMS.txt`, fehlende AppStream-Daten, Desktop-Kategorien, leere Repo-Metadaten) stehen in
  `distribution/README.md` und als Folge-Idee in `prompts/README.md`.

## Geprüft

`appstreamcli validate --no-net` (metainfo), `desktop-file-validate` (Desktop-Datei aus dem .deb), winget-Dateien gegen
die JSON-Schemas 1.10.0, YAML/JSON/XML syntaktisch, PKGBUILD `bash -n`. Nicht möglich im Container: `winget
validate`, `makepkg`/`namcap`, `brew audit`, `flatpak-builder`, `snapcraft`, `choco pack`.
