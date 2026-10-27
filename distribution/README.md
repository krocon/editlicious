# Distribution – editlicious

> Prompt `prompts/done/022-distribution.md` (27.10.2026), analog zu comic-commander-tauri `prompts/014-distribution.md`.
> Inhalt: ein Prompt für Claude im Browser, der editlicious bei Paketmanagern, Verzeichnissen und
> Discovery-Plattformen einträgt, dazu Fakten, Texte und Manifeste, die er nutzt.
> Stand: Release **v1.0.0** (26.10.2026, unsigniert).

## Dateien

| Datei | Zweck |
|---|---|
| [`browser-agent-prompt.md`](browser-agent-prompt.md) | **Der Prompt** – Abschnitt zwischen „PROMPT BEGINNT/ENDET“ in Claude (Browser) kopieren |
| [`facts.json`](facts.json) | einzige Quelle der Fakten: Version, URLs, SHA-256, OS, Paketdetails, Features, Unbekanntes |
| [`listing.md`](listing.md) | Texte EN/DE (Tagline, kurz, lang), Kategorien, Tags, Show-HN- und This-Week-in-Rust-Entwurf |
| `manifests/winget/…/1.0.0/` | WinGet: version, installer (NSIS user + MSI machine), locale en-US |
| `manifests/scoop/editlicious.json` | Scoop (Extras oder eigener Bucket), NSIS als 7z entpackt, `checkver`/`autoupdate` |
| `manifests/homebrew/editlicious.rb` | Homebrew Cask (+ `binary` für `editlicious-mcp`) – **offizieller Tap blockiert** bis Signierung + Notarisierung |
| `manifests/chocolatey/editlicious/` | Chocolatey: `.nuspec` + `tools/chocolateyinstall.ps1` (MSI) |
| `manifests/aur/editlicious-bin/` | AUR: `PKGBUILD` + `.SRCINFO` aus dem .deb (ffmpeg optional) |
| `manifests/appimagehub/editlicious` | AppImageHub: Datei für `data/` in `AppImage/appimage.github.io` |
| `manifests/flathub/` | Flatpak-Manifest (Entwurf aus dem .deb, nur lokal testen) + AppStream-`metainfo.xml` |
| `manifests/snap/snapcraft.yaml` | Snap (Entwurf aus dem .deb, ungetestet) |

Der Browser-Agent liest die Dateien auf GitHub → **`distribution/` muss auf `main` gepusht sein**, bevor der Prompt
läuft (ist es seit dem Commit dieses Ordners).

## Woher die Werte kommen (geprüft am 27.10.2026)

- Version, Asset-Namen, Größen, SHA-256: GitHub-Release-API für `v1.0.0`; alle sieben Pakete heruntergeladen und die
  Summen lokal nachgerechnet (alle gleich).
- Aus den Paketen gelesen: Linux-Binaries `/usr/bin/editlicious` und `/usr/bin/editlicious-mcp`, Desktop-Datei
  `editlicious.desktop` (`Categories=Development;`, `MimeType=text/plain;`), Icon-Name `editlicious`,
  deb-Abhängigkeiten, keine AppStream-Datei; Windows-Exe `editlicious.exe` + `editlicious-mcp.exe` (MSI und NSIS),
  MSI-`ProductCode` `{1B63B59C-…}`, `UpgradeCode` `{0CAA1787-…}` (= `tauri.conf.json`), `ALLUSERS=1`; macOS
  `editlicious.app` universal (x86_64 + arm64, auch das Sidecar), `LSMinimumSystemVersion` 12.0.
- Features, Plattformen, Hinweise: Produktseite https://krocon.de/products/editlicious/ und `README.md`.
- LICENSE am Tag `v1.0.0`: SHA-256 `786cdc49…` (im PKGBUILD).
- Plattformregeln: Homebrew (Gatekeeper-Pflicht seit 1.9.2026, Homebrew 5.0), awesome-rust (> 50 Stars oder
  > 2 000 crates.io-Downloads), MCP Registry (Paket in npm/PyPI/NuGet/crates.io/OCI oder `.mcpb` am Release).
- Geprüft: `appstreamcli validate` und `desktop-file-validate` (s. unten), JSON/YAML syntaktisch. **Nicht** geprüft
  (kein Werkzeug im Container): `winget validate` (die drei winget-Dateien sind aber gegen die JSON-Schemas 1.10.0 aus `microsoft/winget-cli` geprüft), `makepkg`/`namcap`, `brew audit`, `flatpak-builder`, `snapcraft`,
  `choco pack` – im Prompt als ungetestet markiert.

## Einschätzung (Kurzfassung der Recherche)

| Sofort möglich (Browser) | Vorbereitet, braucht Marc | Blockiert / ungeeignet |
|---|---|---|
| WinGet (PR), AppImageHub (PR), AlternativeTo, OpenAlternative, Softpedia, FileHorse, Slant, awesome-tauri, open-source-mac-os-apps, awesome-mac | AUR (SSH-Push), Chocolatey (choco push), Flathub (Build aus Quellen, lokal testen), Snap (snapcraft), eigener Homebrew-Tap / Scoop-Bucket (neue Repos), Show HN / Product Hunt / Reddit / This Week in Rust (Posts in Marcs Namen, TWiR braucht einen Artikel) | Homebrew Cask offiziell, Microsoft Store (Signierung); Scoop Extras und awesome-rust (Bekanntheit, 0 Stars); offizielle MCP Registry (kein Paket/MCPB); MacPorts, Debian, Fedora (Build aus Quellen); Mac App Store (Sandbox) |

Größte Hebel: **macOS signieren + notarisieren** (öffnet Homebrew Cask) und **Windows signieren** (Microsoft Store,
weniger SmartScreen-Warnungen) – vorbereitet in `docs/release.md`. Daneben: GitHub-Sterne über Show HN/awesome-tauri
öffnen Scoop Extras und awesome-rust.

## Befunde für das nächste Release

- **v1.0.0 hat kein `SHA256SUMS.txt`** (v0.1.0 schon) – die Produktseite und `docs/release.md` versprechen es für
  jedes Release. Vermutlich wurde 1.0.0 nach dem fehlgeschlagenen macOS-Lauf (Fix `584fd18`) ohne den
  Prüfsummen-Job veröffentlicht. Nachreichen nur mit Marcs Freigabe (Release-Änderung).
- GitHub-Repo: **Description, Website und Topics sind leer** – vor den Verzeichnis-Einträgen setzen (der Prompt fragt
  danach, Freigabe A).
- Linux-Desktop-Datei: `Categories=Development;` – besser `Development;TextEditor;Utility;`; AppStream-
  `metainfo.xml` fehlt im .deb/.rpm/AppImage (Software-Center, AppImageHub, Flathub lesen sie). Vorlage:
  `manifests/flathub/…metainfo.xml` (Id dort `io.github.krocon.editlicious`; im Paket hieße die Desktop-Datei dann
  passend dazu).
- `editlicious_universal.app.tar.gz` hat keine Version im Namen – für Paketmanager ungeeignet (die Manifeste nutzen das
  `.dmg`).
- MCP: für die offizielle MCP Registry bräuchte es ein `.mcpb`-Asset am Release (URL mit „mcp“, SHA-256) – das Sidecar
  setzt aber die laufende App voraus; erst entscheiden, ob ein Registry-Eintrag sinnvoll ist.

## Neues Release – was anpassen

1. `facts.json`: `version`, `releaseTag`, `releaseDate`, `assets[].url/sha256/size` (Release-Seite zeigt je Asset
   `sha256:`; oder `gh api repos/krocon/editlicious/releases/latest`).
2. Manifeste: winget (neuer Ordner `…/X.Y.Z/`, Summen in GROSSBUCHSTABEN, MSI-ProductCode neu auslesen – `msiinfo
   export <msi> Property`), Scoop (`version`, `hash`; danach erledigt `autoupdate` das), Cask (`version`, `sha256`),
   Chocolatey (`version`, URL, Summe), AUR (`pkgver`, Summen, `.SRCINFO` per `makepkg --printsrcinfo`), metainfo
   (`<release>`), Flatpak/Snap.
3. Bei winget/Scoop/Chocolatey übernehmen nach der ersten Aufnahme meist Bots die Updates – im Bericht vermerken.
