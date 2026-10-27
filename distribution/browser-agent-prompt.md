# Prompt für Claude Browser: Distribution von editlicious

> So benutzen: den Abschnitt ab **„— PROMPT BEGINNT —“** vollständig in eine neue Sitzung von Claude im Browser
> (Claude in Chrome oder der eingebaute Browser der Claude-App) kopieren. Voraussetzung: der Ordner `distribution/`
> ist auf `main` von `krocon/editlicious` gepusht (der Agent liest die Manifeste dort im Browser).
> Erstellt am 27.10.2026 für Release **v1.0.0** (Prompt `prompts/done/022-distribution.md`). Für ein neues Release
> zuerst `distribution/facts.json` und die Manifeste aktualisieren (`distribution/README.md`).

---

— PROMPT BEGINNT —

# Auftrag: editlicious bei Paketmanagern, Software-Verzeichnissen und Discovery-Plattformen eintragen

Du arbeitest im Browser für **Marc Kronberg**, den Entwickler von **editlicious** – einem ruhigen,
tastaturgesteuerten Texteditor für sehr große Dateien. Du trägst editlicious bei geeigneten Plattformen ein bzw.
bereitest die Einträge so weit vor, dass Marc nur noch das tun muss, was ein Browser nicht kann. Ziel sind **wenige,
aber hochwertige, dauerhaft funktionierende Einträge** – nicht möglichst viele.

Arbeite sequenziell, Plattform für Plattform. Halte nach jeder Aktion den tatsächlichen Stand fest. Melde nie einen
Erfolg, den du nicht auf der Plattform selbst gesehen hast (Bestätigungsseite, PR-Nummer, Eintrags-URL).

## 1. Unverrückbare Regeln

1. **Nichts erfinden.** Funktionen, Plattformen, Versionen, Lizenz, Download-URLs, Prüfsummen, Nutzerzahlen,
   Auszeichnungen, Zitate, Leistungswerte – nur aus den Quellen in Abschnitt 2. Was dort fehlt, ist **unbekannt** und
   wird weggelassen oder als offen gemeldet. Messwerte nur mit ihrem Rahmen („release builds, small two-core Linux
   machine“) und nie zugespitzt („fastest editor“).
2. **Nur offizielle Einreichungswege** (Formular der Plattform, Pull Request im offiziellen Repository, Developer-
   Konsole). Keine angeblichen Ein-Klick-Wege, keine Drittanbieter-„Submission-Services“, keine bezahlten Listungen.
3. **Keine Duplikate, keine Verwechslung.** Vor jeder Einreichung auf der Plattform nach „editlicious“,
   „editlicio.us“ und „krocon“ suchen; bei Paket-Repos zusätzlich offene und geschlossene PRs/Issues durchsuchen.
   Gefunden → Status `DUPLIKAT_VORHANDEN`, Link festhalten, nichts ändern. Ein fremdes Projekt mit ähnlichem Namen
   ist kein Duplikat – aber ein Namenskonflikt (Paketname belegt) → `BLOCKIERT`, Marc fragen.
4. **Sicherheit:** Captchas, MFA, Login und Sicherheitswarnungen nie umgehen; Passwörter, Tokens, API-Keys nie
   anzeigen, kopieren oder eintippen. Ist ein Login nötig → anhalten, Marc bitten, sich anzumelden, danach weiter.
5. **Persönliche Daten:** Öffentlich nur „Marc Kronberg“, https://krocon.de/ und die GitHub-Issues als Kontakt.
   **Keine E-Mail-Adresse, Postanschrift oder Telefonnummer** in öffentliche Felder. Verlangt ein Formular eine
   E-Mail für die Rückmeldung (nicht öffentlich), frage Marc, welche Adresse du nehmen darfst.
6. **Keine blinden Wiederholungen.** Schlägt etwas fehl: Fehlermeldung lesen, Ursache bestimmen, beheben (nur mit
   belegten Angaben), höchstens **ein** weiterer Versuch. Danach `BLOCKIERT` mit Ursache.
7. **Vorbereitet ≠ eingereicht.** Ein ausgefülltes, nicht abgeschicktes Formular ist `VORBEREITET`. Ein abgeschicktes
   ist `EINGEREICHT`, erst eine sichtbare Freischaltung ist `VERÖFFENTLICHT`.
8. **Name immer klein:** „editlicious“, auch am Satzanfang und in Titelfeldern (sofern das Formular es zulässt).

## 2. Quellen und Fakten (Stand v1.0.0)

Lies zuerst diese Seiten im Browser und gleiche ab. Weicht etwas von der Tabelle ab, gilt die Quelle – dann den
Unterschied im Bericht nennen.

| Quelle | URL |
|---|---|
| Produktseite (maßgeblich für Features) | https://krocon.de/products/editlicious/ |
| Live-Demo (Engine als WebAssembly) | https://krocon.de/products/editlicious/demo/ |
| Repository + README | https://github.com/krocon/editlicious |
| Release v1.0.0 (Pakete, Prüfsummen je Asset) | https://github.com/krocon/editlicious/releases/tag/v1.0.0 |
| KI-Steuerung (MCP) | https://github.com/krocon/editlicious/blob/main/docs/mcp.md |
| **Faktendatei** | https://github.com/krocon/editlicious/blob/main/distribution/facts.json |
| **Eintragstexte EN/DE, Tags, Kategorien** | https://github.com/krocon/editlicious/blob/main/distribution/listing.md |
| **Manifeste** | https://github.com/krocon/editlicious/tree/main/distribution/manifests |
| Rohdateien (zum Kopieren) | `https://raw.githubusercontent.com/krocon/editlicious/main/distribution/<pfad>` |

Kernfakten (aus Release-API, Paketen, README und Produktseite am 27.10.2026 geprüft):

- **Name:** editlicious · **Version:** 1.0.0 (Tag `v1.0.0`, veröffentlicht 26.10.2026, kein Pre-release; davor
  v0.1.0 als Pre-release) · **Lizenz:** MIT (Drittteile: `THIRD-PARTY-NOTICES.txt` im Paket) · **Preis:** kostenlos,
  Open Source · **Autor/Publisher:** Marc Kronberg · **Technik:** Tauri 2, Rust (Engine `ed-core`), Angular.
- **Was es ist:** Texteditor für riesige Dateien (Millionen Zeilen, Zeilen mit Millionen Zeichen, bis 4 GB),
  tastaturgesteuert und ablenkungsarm. **Keine IDE** (keine Projektansicht, kein Language Server, kein Plugin-Store).
  **Keine Telemetrie**, keine Konten, keine Update-Abfragen. Oberfläche Englisch und Deutsch.
- **Plattformen:** macOS ≥ 12 (universal: Apple Silicon + Intel, `.dmg`) · Windows 10/11 x64 (NSIS-`setup.exe` pro
  Nutzer, `.msi` pro Rechner; braucht WebView2) · Linux x86_64 (`.deb`, `.rpm` brauchen WebKitGTK 4.1; `.AppImage`
  bringt alles mit). Keine ARM-Builds für Windows/Linux.
- **Signierung:** **keine** (macOS weder signiert noch notarisiert, Windows unsigniert). Das blockiert manche
  Plattformen (s. Abschnitt 4).
- **Beigaben in jedem Paket:** `editlicious-mcp` (MCP-Server für KI-Assistenten, in der App standardmäßig aus, nur
  lokal) und die CLI `editlicious render` (Aufnahmen/Skripte als Video, MP4 braucht ein externes ffmpeg).
- **Prüfsummendatei:** v1.0.0 hat **kein** `SHA256SUMS.txt` (die Produktseite behauptet es für jedes Release) – die
  Release-Seite zeigt je Asset ein `sha256:`; das sind die Werte unten. Im Bericht als Befund nennen, nicht
  „reparieren“.
- **Downloads (stabil, versioniert):**

| Paket | URL | SHA-256 |
|---|---|---|
| macOS dmg | https://github.com/krocon/editlicious/releases/download/v1.0.0/editlicious_1.0.0_universal.dmg | `4ff4af99be572c636b260e932d7651296a7f8acc4703382408978b20a5d2320f` |
| Windows setup.exe | https://github.com/krocon/editlicious/releases/download/v1.0.0/editlicious_1.0.0_x64-setup.exe | `acec1979b3d76e0e4754c7b90a522b4be98bf14caa78b08a85d267c75ebd0982` |
| Windows msi | https://github.com/krocon/editlicious/releases/download/v1.0.0/editlicious_1.0.0_x64_en-US.msi | `6ce8722d1895eeb90c0a43f06b97de2f0071b3eb301d418a46870a6b4d03941e` |
| Linux deb | https://github.com/krocon/editlicious/releases/download/v1.0.0/editlicious_1.0.0_amd64.deb | `bacd01bf4325340079548f69b187f8df5d57a96c0cbd055da913972ee3c3b1a4` |
| Linux rpm | https://github.com/krocon/editlicious/releases/download/v1.0.0/editlicious-1.0.0-1.x86_64.rpm | `4d5902a36cc7861a01910c4618b09cbe7d368a67a0707488ea2e2f85edb8810e` |
| Linux AppImage | https://github.com/krocon/editlicious/releases/download/v1.0.0/editlicious_1.0.0_amd64.AppImage | `55e95eebd8a9397733605b14c98abfda331d001d983955ad9995d4d981b33a6a` |

  Für Verzeichnisse, die nur einen Download-Link wollen: **https://github.com/krocon/editlicious/releases** (bleibt
  bei neuen Versionen gültig). Für Paketmanager immer die versionierten URLs oben. `editlicious_universal.app.tar.gz`
  hat keine Version im Namen – nicht verwenden.
- **Bilder:** Icon https://raw.githubusercontent.com/krocon/editlicious/main/src-tauri/icons/icon.png · Screenshots
  (1100 × 760) https://krocon.de/products/editlicious/img/main.png (Rust-Datei, Suchleiste), …/img/big-log.png
  (Logdatei mit 1 Mio. Zeilen), …/img/wrap.png (Umbruch), …/img/palette.png (Befehlspalette), …/img/settings.png
  (Einstellungen); Ersatz: `https://raw.githubusercontent.com/krocon/editlicious/main/site/img/<name>.png`. Jede
  Bild-URL einmal öffnen und prüfen, dass sie lädt. Lädt ein Formular nur Dateien hoch: Marc bitten, die Datei
  bereitzustellen.
- **Kontakt öffentlich:** https://github.com/krocon/editlicious/issues

## 3. Statusmodell und Arbeitsliste

Führe von Anfang an eine Tabelle (im Chat, nach jeder Plattform aktualisiert) mit den Spalten:

`# | Plattform | Kategorie | Einreichungs-URL | Methode | Voraussetzungen | Account nötig | Kosten | Dateien/Metadaten | Status | Nächste Aktion | Nutzen (H/M/N) | Begründung | Beleg-Link`

**Methoden:** `FORMULAR` (Webformular) · `PR-WEB` (Pull Request über die GitHub-Weboberfläche) · `REPO-NÖTIG`
(eigenes neues Repository nötig, z. B. Tap/Bucket) · `BUILD-SYSTEM` (externer Build/Store-Upload, z. B. Snapcraft,
Flathub-Build, Chocolatey-Push) · `CLI-NÖTIG` (nur per Kommandozeile/SSH, Browser reicht nicht) · `POST` (Community).

**Status:** `NEU` → `RECHERCHIERT` → `VORBEREITET` → `WARTET_AUF_MARC` → `EINGEREICHT` → `IN_PRÜFUNG` →
`VERÖFFENTLICHT`; außerdem `ABGELEHNT` (mit Grund), `BLOCKIERT` (mit Blocker), `UNGEEIGNET` (mit Begründung),
`DUPLIKAT_VORHANDEN` (mit Link).

**Entscheidungsregeln für Aufnahme:**

- Plattform aktiv? (letzte Einträge/Commits/Antworten der Moderation aus den letzten 6 Monaten; auf der Seite selbst
  prüfen, nicht aus Suchergebnissen.) Nein → `UNGEEIGNET`.
- Passt das Produkt? (Desktop-App, kostenlos, Open Source, die angebotenen OS, Kategorie Texteditor/Entwicklerwerkzeug.)
  Nein → `UNGEEIGNET`.
- Technische Voraussetzung erfüllt? (Signierung, Architektur, Build aus Quellen, Bekanntheitsschwellen wie Stars –
  das Repo ist neu, Stand 27.10.2026: 0 Stars.) Nein → `BLOCKIERT` mit genauer Anforderung und Quelle (Link auf die
  Regel).
- Kostet etwas? → nur mit Freigabe (Abschnitt 6), sonst `WARTET_AUF_MARC`.
- Priorität = Reichweite × Relevanz × Erfolgsaussicht ÷ Aufwand. Zielgruppe: Entwickler, Admins, Datenleute – wer
  Logdateien, Datenexporte, CSV/JSON-Dumps öffnet. Reihenfolge innerhalb von Abschnitt 4 einhalten, außer eine
  Prüfung ergibt etwas anderes (dann begründen).

## 4. Plattforminventar mit Vorbewertung

Die Vorbewertung stammt aus einer Recherche vom 27.10.2026 (und den Erfahrungen mit Comic Commander, gleicher Autor,
gleiche Technik). **Prüfe jede Zeile auf der Plattform selbst** (Regeln, Formular, Aktivität) und korrigiere sie,
wenn sie nicht mehr stimmt.

### 4.1 Vorarbeit (zuerst)

| Plattform | Methode | Vorbewertung |
|---|---|---|
| GitHub-Repo `krocon/editlicious` | Einstellungen | **Description, Website und Topics sind leer.** Viele Kuratoren und Verzeichnisse lesen sie. Vorschlag: Description = Kurzbeschreibung EN aus `listing.md`; Website = https://krocon.de/products/editlicious/; Topics = `text-editor editor large-files log-viewer rust tauri angular webassembly mcp desktop-app macos windows linux`. Öffentliche Änderung → **Freigabe A** (Abschnitt 6). |

### 4.2 Paketmanager

| # | Plattform | Methode | Vorbewertung | Vorbereitete Datei |
|---|---|---|---|---|
| P1 | **WinGet** (microsoft/winget-pkgs) | PR-WEB | geeignet. Unsignierte Installer sind zulässig, die Validierung prüft aber mit Defender/SmartScreen. Identifier `MarcKronberg.editlicious` (Schreibweise im Repo bei ähnlichen Paketen prüfen). Microsoft-CLA wird im PR verlangt → **Freigabe C**. | `manifests/winget/manifests/m/MarcKronberg/editlicious/1.0.0/` (3 Dateien) |
| P2 | **Scoop** – Bucket `ScoopInstaller/Extras` | PR-WEB | Aufnahmekriterien in `CONTRIBUTING.md` des Buckets lesen (Bekanntheit!) – bei 0 Stars wahrscheinlich `BLOCKIERT`. Alternative: eigener Bucket `krocon/scoop-bucket` → neues Repo → **Freigabe B**. | `manifests/scoop/editlicious.json` |
| P3 | **AUR** (`editlicious-bin`) | CLI-NÖTIG | geeignet, Binärpaket aus dem .deb. Account-Anlage geht im Browser (aur.archlinux.org/register, **Freigabe G**), das Hochladen nur per `git push` über SSH → Marc. Prüfe, dass der Name frei ist. | `manifests/aur/editlicious-bin/PKGBUILD`, `.SRCINFO` |
| P4 | **AppImageHub** (AppImage/appimage.github.io) | PR-WEB | wahrscheinlich geeignet: neue Datei `data/editlicious` mit der Releases-URL. Die CI dort startet das AppImage auf einer älteren Distribution und liest AppStream-Daten – fehlt `metainfo.xml` im AppImage oder scheitert der Start, CI-Log lesen und `BLOCKIERT` melden. README/Regeln des Repos zuerst lesen. | `manifests/appimagehub/editlicious` |
| P5 | **Chocolatey** Community | BUILD-SYSTEM / CLI-NÖTIG | geeignet, aber das Paket wird per `choco push` mit API-Key hochgeladen; prüfe, ob die Website heute einen Upload im Browser anbietet. Moderation prüft das Paket, Wochen Laufzeit. Account im Browser (Freigabe G); API-Key nie anzeigen. | `manifests/chocolatey/editlicious/` (`.nuspec`, `tools/chocolateyinstall.ps1`; Platzhalter `{{CHOCOLATEY_USERNAME}}`) |
| P6 | **Homebrew Cask** – offizieller Tap | PR-WEB | **BLOCKIERT**: Homebrew unterstützt seit 1. September 2026 keine Casks mehr, die Gatekeeper nicht bestehen (signiert + notarisiert); v1.0.0 ist unsigniert. Regel auf https://docs.brew.sh/Acceptable-Casks nachlesen und zitieren. | `manifests/homebrew/editlicious.rb` |
| P6b | Homebrew – eigener Tap `krocon/homebrew-tap` | REPO-NÖTIG | möglich (auch für Comic Commander nutzbar), aber Nutzer müssen die unsignierte App selbst freigeben. Nur mit **Freigabe B**; sonst `WARTET_AUF_MARC`. | dieselbe `.rb` unter `Casks/` |
| P7 | **Flathub** | PR-WEB + BUILD-SYSTEM | grundsätzlich geeignet (App-ID `io.github.krocon.editlicious` passt zum GitHub-Konto, Verifizierung über GitHub). Flathub erwartet bei Open Source in der Regel einen Build aus den Quellen (Cargo- und pnpm-Quellen offline); der Entwurf nutzt das .deb und ist **nur für lokale Tests**. Ein Texteditor braucht breiten Dateizugriff (`--filesystem=home`) – das muss im PR begründet werden. Einreichung = PR gegen `flathub/flathub` (Branch `new-pr`) – erst nach lokalem Build durch Marc → `VORBEREITET`, `WARTET_AUF_MARC`. Anforderungen unter https://docs.flathub.org/docs/for-app-authors/requirements lesen. | `manifests/flathub/io.github.krocon.editlicious.yml`, `.metainfo.xml` |
| P8 | **Snap Store** | BUILD-SYSTEM | möglich: Name `editlicious` auf snapcraft.io registrieren (Account nötig, **Freigabe C/G**), Bauen/Hochladen per snapcraft-CLI oder Launchpad → Marc. Strict confinement sperrt Dateien außerhalb von Home (z. B. `/var/log`) – für einen Log-Editor relevant; „classic“ müsste beantragt werden (Regeln lesen). | `manifests/snap/snapcraft.yaml` (Entwurf, ungetestet) |
| P9 | **Fedora Copr** | BUILD-SYSTEM | prüfen: Copr-Regeln zu Binärpaketen bzw. Repackaging eines fremd gebauten RPM lesen. Erlaubt nur Builds aus Quellen → `UNGEEIGNET` vorerst (Node/pnpm offline). Account = Fedora-Konto → Marc. | – |
| P10 | MacPorts | PR-WEB | **UNGEEIGNET vorerst**: Ports bauen aus den Quellen; ein Tauri-Build mit pnpm und Rust ohne Netz im Build-Schritt ist aufwendig, ein Binär-Port aus der unsignierten .dmg unüblich. Im Bericht als „später prüfen“ führen. | – |
| P11 | Debian/Ubuntu-Archiv, Fedora-Repos | – | **UNGEEIGNET**: braucht Sponsor/Paketbetreuer und einen Build aus Quellen mit paketierten npm- und Rust-Abhängigkeiten (Hunderte Crates/Pakete). Lizenzlich unproblematisch (MIT), nur der Aufwand. | – |
| P12 | Microsoft Store | BUILD-SYSTEM | **BLOCKIERT**: Win32-Installer (MSI/EXE) müssen mit einem vertrauenswürdigen Zertifikat signiert sein. Erst nach Windows-Signierung. Regel nachlesen und verlinken. | – |
| P13 | Mac App Store | – | **UNGEEIGNET**: verlangt App-Sandbox und Apple-Entwicklerprogramm; die App ist nicht sandboxed (öffnet beliebige Pfade, Sidecar). | – |
| P14 | **crates.io** (`ed-core`) | CLI-NÖTIG | **nur Frage an Marc**, nicht veröffentlichen: Namensvergabe ist praktisch unumkehrbar, `ed-core` ist kein eigenständiges Produkt. Im Bericht als offene Entscheidung. | – |

### 4.3 MCP-Verzeichnisse (eigener Abschnitt – Sonderfall)

`editlicious-mcp` steuert nur eine **laufende editlicious-App** (Brücke über 127.0.0.1, in der App einzuschalten). Es
ist kein eigenständiger MCP-Server. Ein Eintrag ist nur ehrlich, wenn er das klar sagt („requires the editlicious
desktop app“).

| # | Plattform | Vorbewertung |
|---|---|---|
| M1 | Offizielle **MCP Registry** (registry.modelcontextprotocol.io) | **BLOCKIERT**: verlangt ein Paket in npm/PyPI/NuGet/crates.io/OCI oder ein `.mcpb`-Bundle am GitHub-Release (URL muss „mcp“ enthalten, `fileSha256`); gibt es für v1.0.0 nicht. Veröffentlichung geht außerdem per `mcp-publisher`-CLI, nicht im Browser. Nur im Bericht als Option für ein späteres Release (MCPB-Asset) führen. Regeln: https://modelcontextprotocol.io/registry/package-types |
| M2 | Kuratierte MCP-Listen (z. B. `punkpeye/awesome-mcp-servers`, Abschnitt für Editoren/Developer Tools) | prüfen: Regeln lesen; nur eintragen, wenn die Liste auch Server zulässt, die eine Desktop-App voraussetzen. Zeile mit Hinweis „needs the editlicious app“. |
| M3 | Weitere MCP-Kataloge (mcp.so, Glama, Smithery, PulseMCP) | einzeln prüfen: viele indizieren GitHub-Repos selbst oder wollen ein installierbares Paket. Nur offizielle Einreichungswege, keine Angaben, die einen eigenständigen Server vortäuschen. Sonst `UNGEEIGNET`. |

### 4.4 Software-Verzeichnisse

Für jedes: Richtlinie für neue Software lesen, Duplikat prüfen, Formular ausfüllen. Kategorie wählen, unter der dort
andere Texteditoren stehen. Texte aus `listing.md` (EN; deutsche Portale DE).

| # | Plattform | Einstieg | Vorbewertung |
|---|---|---|---|
| V1 | **AlternativeTo** | https://alternativeto.net/ → „Add an app“ (Login) | hoch: Nutzer suchen „alternative to EmEditor / UltraEdit / Large Text File Viewer / klogg“. Plattformen, Lizenz „Open Source“, „Free“, Tags. Als Alternative nur zu Programmen eintragen, die dort als Texteditor/Viewer für große Dateien geführt sind (`listing.md`). |
| V2 | **OpenAlternative** | https://openalternative.co/ → Submit | mittel–hoch: Open-Source-Alternativen zu kommerzieller Software (passend zu EmEditor/UltraEdit/Sublime Text). Kostenlose Einreichung von „Featured“-/Express-Optionen unterscheiden – nur kostenlos. |
| V3 | **SourceForge** | https://sourceforge.net/ → Create a project / GitHub-Import | mittel: Projekt mit Spiegelung der GitHub-Releases. Neues öffentliches Projekt in Marcs Namen → **Freigabe B**. |
| V4 | **Softpedia** | „Submit program“ auf https://www.softpedia.com/ (Windows, Mac, Linux getrennt) | mittel: kostenloses Formular. Prüfe, ob sie unsignierte Software mit einem Hinweis listen. |
| V5 | **MacUpdate** | Developer-Bereich auf https://www.macupdate.com/ | mittel: Mac-only. Nur kostenlose Option. |
| V6 | **FileHorse** | „Submit Software“ auf https://www.filehorse.com/ | mittel, Windows/Mac. |
| V7 | **Uptodown** | Developer-Konsole auf https://www.uptodown.com/ | mittel; prüfen, ob Desktop-Apps (Windows/Mac) eingereicht werden können und ob Uptodown die Datei selbst hostet (Prüfsumme gleich?). |
| V8 | **MajorGeeks** | Kontakt/Submit auf https://www.majorgeeks.com/ | niedrig–mittel, Windows; oft nur per E-Mail → dann `WARTET_AUF_MARC` mit fertigem Text. |
| V9 | **SaaSHub** | https://www.saashub.com/ → Submit | niedrig–mittel. Keine kostenpflichtige „Featured“-Option. |
| V10 | **Slant** | https://www.slant.co/ – Fragen wie „What are the best text editors for large files?“ | mittel: Option zu einer passenden bestehenden Frage hinzufügen (Login). Nur sachliche Pro-Punkte aus `listing.md`, keine Bewertung abgeben, keine Kontra-Punkte anderer Programme. |
| V11 | **FossHub** | Bewerbung auf https://www.fosshub.com/ | niedrig: für etablierte Projekte. Kriterien prüfen; voraussichtlich `UNGEEIGNET` (zu neu). |
| V12 | **CHIP.de** (Download-Bereich) | Software-Einreichung für Hersteller (auf chip.de suchen) | mittel für DE, deutsche Texte (Oberfläche gibt es auf Deutsch – erwähnen). |
| V13 | **heise Download** | „Software melden/einreichen“ (auf heise.de/download suchen) | mittel für DE. |
| V14 | **Softonic** | Developer-/Publisher-Bereich | niedrig; nutzt Softonic eigene Installer-Wrapper → `UNGEEIGNET` (kein Wrapper um Marcs Installer). |
| V15 | **Neowin Software** / **TechSpot Downloads** | jeweilige Submit-Seite | niedrig; nur, wenn ein Formular ohne Kosten existiert. |
| V16 | **LinuxLinks** | https://www.linuxlinks.com/ (Kontakt/Submit) | niedrig–mittel für Linux (Rubrik Text-Editoren). |

Allgemeine Regel für Verzeichnisse: Nur kostenlose Grundeinträge; keine „Featured“, „Express-Review“, „Sponsored“.
Ein Portal, das Software nur mit eigenem Downloader/Installer-Wrapper ausliefert, ist `UNGEEIGNET`.

### 4.5 Kuratierte Listen (GitHub)

Je Liste: `CONTRIBUTING.md` lesen (Kriterien, Format, Sortierung, Mindest-Stars), Duplikat prüfen, PR über die
Weboberfläche (Fork → Datei bearbeiten → PR). Eine Zeile im Stil der Liste, keine Werbesprache.

| # | Liste | Vorbewertung |
|---|---|---|
| L1 | `tauri-apps/awesome-tauri` | hoch: Abschnitt „Applications“ (Unterrubrik für Entwicklerwerkzeuge/Produktivität). |
| L2 | `serhii-londar/open-source-mac-os-apps` | mittel: Kategorie „Editors“ bzw. „Text“; Sprachen-Badges (Rust, TypeScript) laut Liste. |
| L3 | `jaywcjlove/awesome-mac` | mittel: Rubrik Text-Editoren, Kennzeichnung „Open Source“/„Free“ nach den Regeln der Liste. |
| L4 | `rust-unofficial/awesome-rust` | **wahrscheinlich BLOCKIERT**: Anwendungen brauchen > 50 GitHub-Stars (oder > 2 000 crates.io-Downloads, oder gleichwertig belegt). Aktuelle `CONTRIBUTING.md` prüfen; nicht erreicht → `BLOCKIERT` mit Kriterium. |
| L5 | `luong-komorebi/Awesome-Linux-Software` | mittel: Rubrik „Text Editors“; Regeln lesen. |
| L6 | Windows-Listen (z. B. `Awesome-Windows/Awesome`) | prüfen: Rubrik Text-Editoren, Aktivität, Regeln. |
| L7 | Listen für Texteditoren / große Dateien / Log-Viewer | suchen (GitHub-Suche „awesome text editors“, „awesome log viewer“): nur aktiv gepflegte Listen. |

### 4.6 Launch und Community (getrennt behandeln – das sind keine Pakete)

Jeder öffentliche Beitrag erscheint in Marcs Namen → **immer Freigabe D** mit dem fertigen Text, vorher nichts
posten. Regeln der Community zur Eigenwerbung vorher lesen und im Bericht verlinken.

| # | Plattform | Vorbewertung |
|---|---|---|
| C1 | **Show HN** (news.ycombinator.com) | hoch für Entwickler: die Live-Demo (ohne Installation, im Browser) erfüllt die HN-Regel „etwas, das man ausprobieren kann“. Entwurf in `listing.md`. Nur Marc selbst darf posten (eigener Account). Du bereitest Titel + Text vor. |
| C2 | **This Week in Rust** (`rust-lang/this-week-in-rust`, „Project/Tooling Updates“) | mittel: verlinkt Artikel, nicht Repos. Ohne Blogartikel von Marc → `WARTET_AUF_MARC` (Hinweis im Bericht). Mit Artikel: PR im Format der Datei der kommenden Ausgabe → Freigabe D. |
| C3 | **Product Hunt** | mittel; Launch braucht Vorbereitung (Bilder, Maker-Profil, Termin). Nur vorbereiten, Termin mit Marc. |
| C4 | Reddit: r/rust (Projekt-Posts, Regeln lesen), r/opensource, r/macapps (strenge Regeln!), r/linux, r/commandline (nur falls dort GUI-Tools erlaubt), r/sysadmin (meist keine Eigenwerbung) | einzeln Regeln prüfen (Selbstwerbung, Flair, Wochen-Threads). Wo Eigenwerbung verboten ist → `UNGEEIGNET`. |
| C5 | **Lobsters** | nur mit Einladung/eigenem Account von Marc; Regeln zur Eigenwerbung (Tag „show“) beachten → Freigabe D. |
| C6 | Tauri-Community (Discord „showcase“, GitHub Discussions von tauri-apps) | niedrig–mittel; nur mit Freigabe D. |

## 5. Ablauf

### Phase 1 – Bestandsaufnahme (≈ 15 min)

1. Produktseite, Live-Demo, Repository/README, Release v1.0.0, `facts.json` und `listing.md` öffnen (Abschnitt 2).
2. Prüfe: Name, Beschreibung, Features, OS, Lizenz, Version, Release-Assets und Prüfsummen (auf der Release-Seite
   steht je Asset ein `sha256:`), Signierstatus, Bild-URLs, Kontakt. Notiere Abweichungen.
3. Prüfe: Gibt es eine neuere Version als 1.0.0? Dann **anhalten**: Manifeste passen nicht mehr → Marc bitten,
   `distribution/` zu aktualisieren (oder nur Verzeichnisse mit der allgemeinen Releases-URL bearbeiten).
4. Ergebnis: kurze Faktenliste im Chat, Spalte „unbekannt“ für Fehlendes.

### Phase 2 – Inventar und Priorisierung (≈ 30–60 min)

1. Jede Zeile aus Abschnitt 4 auf der Plattform selbst prüfen: aktiv? Einreichungsweg? Kosten? Regeln (Link)?
   Duplikat? Paketname frei?
2. Weitere seriöse, aktive Portale ergänzen, die du dabei findest (höchstens 5, jeweils mit Begründung).
3. Arbeitsliste (Abschnitt 3) vollständig ausfüllen, nach Priorität sortieren.
4. **Sammel-Freigabe:** Stelle Marc *eine* Liste mit allen Freigaben, die du absehbar brauchst (Abschnitt 6), mit je
   einer Zeile Begründung und dem genauen Text/den Daten, die veröffentlicht würden. Arbeite in der Zwischenzeit an
   allem weiter, was keine Freigabe braucht.

### Phase 3 – Einträge vorbereiten

1. Texte aus `listing.md`, Fakten aus `facts.json`, Manifeste aus `distribution/manifests/`. Nicht umformulieren,
   was belegt ist; nur an Längengrenzen der Formulare anpassen (kürzen, nicht ergänzen).
2. Prüfe vor jeder PR-Einreichung das Manifest gegen die **aktuellen** Regeln der Plattform (Schema-Version,
   Pflichtfelder, Dateipfad, Benennung). Ist eine Anpassung nötig, nimm sie im PR vor und nenne sie im Bericht – mit
   Quelle. Niemals Prüfsummen, URLs oder Versionen ändern, außer sie sind nachweislich falsch (dann anhalten).
3. Platzhalter (`{{CHOCOLATEY_USERNAME}}`, `{{AUR_MAINTAINER}}`) nur mit Marcs Angabe füllen.

### Phase 4 – Einreichen (sequenziell, Reihenfolge = Priorität)

Für **jede** Plattform:

1. Duplikat-Check (Regel 3) – erneut, direkt vor dem Absenden.
2. Login nötig und nicht angemeldet → `WARTET_AUF_MARC` („bitte bei X anmelden“), mit der nächsten Plattform
   weitermachen, später zurückkehren.
3. Formular/PR vollständig ausfüllen. Bei Datei-Uploads (Screenshots, Icon): Marc bitten, die Datei auszuwählen,
   wenn du keinen Zugriff hast.
4. **PR über die GitHub-Weboberfläche:** Fork des Ziel-Repos in Marcs Konto (Fork ist reversibel, keine Freigabe
   nötig), Branch `editlicious-1.0.0`, Dateien anlegen (Inhalt aus der Rohdatei kopieren, Pfad exakt wie vorgegeben),
   Commit-Nachricht und PR-Titel nach der Konvention des Repos – Vorschläge:
   - winget: `New package: MarcKronberg.editlicious version 1.0.0`
   - Scoop Extras: `editlicious: Add version 1.0.0`
   - AppImageHub: `Add editlicious`
   - awesome-Listen: `Add editlicious`
   PR-Vorlage des Repos vollständig ausfüllen (Checkboxen nur ankreuzen, wenn wahr).
5. Nach dem Absenden: Bestätigung/PR-Seite lesen, **CI-/Bot-Ergebnisse abwarten oder später prüfen** (winget-Bot,
   AppImageHub-CI). Status `EINGEREICHT`/`IN_PRÜFUNG` mit Link.
6. Validierungsfehler: Meldung lesen, Ursache bestimmen, mit belegten Angaben beheben (Regel 6). Fehler, die eine
   Änderung an der App oder am Release erfordern (Signierung, Paketinhalt, AppStream-Daten, glibc), nicht umgehen →
   `BLOCKIERT`.

### Phase 5 – Freigaben einholen (s. Abschnitt 6) und nachziehen

Nach Marcs Antworten die freigegebenen Schritte ausführen; Abgelehntes als `WARTET_AUF_MARC`/`UNGEEIGNET` mit seinem
Grund vermerken.

### Phase 6 – Abschlussbericht

Erstelle den Bericht nach der Vorlage in Abschnitt 7. Kannst du keine Datei schreiben, gib ihn vollständig als
Markdown im Chat aus mit der Bitte, ihn als `distribution/editlicious-distribution-report.md` zu speichern (oder
biete an, ihn über die GitHub-Weboberfläche als Datei in einem eigenen Branch anzulegen – das ist ein Commit im
Repo → Freigabe B).

## 6. Freigabepunkte

Vor diesen Aktionen **anhalten und ausdrücklich fragen** (Marc muss mit „ja“ zu genau diesem Schritt antworten):

| Freigabe | Was |
|---|---|
| **A** | Änderungen an Marcs bestehenden öffentlichen Projekten: Repo-Description/Topics/Website, Releases (Texte, Assets), README, Produktseite. **Releases nie ändern**, nur vorschlagen (z. B. fehlendes `SHA256SUMS.txt`). |
| **B** | Neue öffentliche Repositories oder Projekte in Marcs Namen (Homebrew-Tap, Scoop-Bucket, SourceForge-Projekt), Commits in Marcs Repos, Reservieren von Paketnamen (crates.io, Snap-Name). |
| **C** | Bedingungen akzeptieren, die über normale Nutzung hinausgehen: CLA (Microsoft bei winget), Publisher-/Developer-Agreements (Snapcraft, Chocolatey, Stores), rechtlich verbindliche Erklärungen („Ich bin Rechteinhaber …“). Den Wortlaut bzw. Link zeigen. |
| **D** | Jeder öffentliche Beitrag in Marcs Namen (HN, Product Hunt, Reddit, Lobsters, This Week in Rust, Discord, Foren) – mit dem fertigen Text. |
| **E** | Alles mit Kosten (auch „kostenlose Testphase“ mit Zahlungsdaten), Bestellungen, bezahlte Listungen. |
| **F** | Weitergabe persönlicher Daten, die nicht in Abschnitt 2 als öffentlich freigegeben sind (E-Mail, Adresse, Telefon). |
| **G** | Neue Accounts/Zugangsdaten anlegen (Marc legt sie selbst an; du öffnest nur die Seite), API-Keys, SSH-Keys, Sicherheits-Einstellungen. |
| **H** | Löschen oder Ändern bestehender Einträge (auch eines gefundenen veralteten editlicious-Eintrags). |

Ohne Freigabe erlaubt: Seiten lesen, suchen, Formulare ausfüllen **ohne** Absenden, Forks in Marcs Konto, PRs an
Paket-/Listen-Repos, deren Einreichung eindeutig zum Auftrag gehört und keine der Freigaben A–H berührt, kostenlose
Formulareinreichungen bei Software-Verzeichnissen ohne Bedingungen über die normale Nutzung hinaus.

Fragen bündeln: lieber eine Liste mit fünf Freigaben als fünf Unterbrechungen.

## 7. Vorlage Abschlussbericht `editlicious-distribution-report.md`

```markdown
# editlicious – Distributionsbericht
Stand: <Datum, Uhrzeit> · Version: 1.0.0 · Bearbeitet von: Claude (Browser) für Marc Kronberg

## Kurzfassung
<3–5 Sätze: was ist live, was in Prüfung, was blockiert, wichtigster nächster Schritt>

## 1. Veröffentlicht
| Plattform | Link zum Eintrag | Datum | Anmerkung |

## 2. In Prüfung (eingereicht)
| Plattform | Link (PR/Ticket/Bestätigung) | eingereicht am | erwartete Dauer laut Plattform | wie man den Stand prüft |

## 3. Vorbereitet, nicht eingereicht
| Plattform | was fehlt | wer muss was tun | Link zum offenen Formular/Entwurf |

## 4. Technisch blockiert
| Plattform | Blocker | Quelle der Regel (Link) | was es lösen würde (z. B. Signierung, Source-Build, MCPB-Asset, Stars) |

## 5. Abgelehnt
| Plattform | Begründung der Plattform (Zitat) | Link | Einschätzung, ob neuer Versuch sinnvoll |

## 6. Manuelle Aktionen für Marc (Checkliste)
- [ ] <konkrete Aktion, z. B. „AUR: `git push` des Pakets editlicious-bin (Dateien in distribution/manifests/aur)“>

## 7. Nicht geeignet
| Plattform | Begründung |

## 8. Links (alle Einträge, PRs, Formulare)

## 9. Empfohlene nächste Schritte
<z. B. Signierung + Notarisierung (öffnet Homebrew Cask, Microsoft Store), SHA256SUMS.txt am Release, AppStream-Metadaten ins .deb/AppImage, Desktop-Kategorien, Flatpak-Build aus Quellen, MCPB-Bundle, ARM-Builds>

## Anhang: Arbeitsliste (vollständige Tabelle aus Abschnitt 3)
## Anhang: Abweichungen zwischen Quellen und facts.json
```

## 8. Fehlerbehandlung (Kurzreferenz)

| Situation | Reaktion |
|---|---|
| Seite lädt nicht / Formular weg | einmal neu laden; sonst Startseite der Plattform → Weg suchen; sonst `BLOCKIERT` („Einreichung nicht auffindbar“, Datum). |
| Captcha, MFA, „Bist du ein Mensch?“ | anhalten, Marc lösen lassen. |
| Pflichtfeld ohne belegte Angabe (z. B. Downloadzahlen, Telefonnummer, Firmen-USt-ID) | leer lassen, wenn möglich; sonst `WARTET_AUF_MARC` mit Frage. Nie Fantasiewerte. |
| Plattform will Datei statt URL hochladen (Installer) | nicht selbst herunter- und hochladen ohne Rückfrage; Marc fragen, ob die Plattform die Datei hosten darf. |
| PR-Bot meldet Fehler | Log lesen, Regel nachschlagen, gezielt korrigieren, Kommentar im PR mit kurzer Erklärung; zweiter Fehlschlag → `BLOCKIERT`. |
| Moderator stellt Rückfrage | Fakten aus Abschnitt 2 beantworten; Meinungs-/Rechtsfragen → Marc. |
| Eintrag existiert schon, aber veraltet/falsch | nicht ändern (Freigabe H); im Bericht mit Link und Korrekturvorschlag. |
| Plattform verlangt Signierung oder Mindest-Stars | `BLOCKIERT`, Regel verlinken. |
| Virenscanner meldet etwas (winget, Softpedia, Uptodown) | nichts umgehen, Meldung wörtlich festhalten, Marc informieren (möglicher Fehlalarm bei unsignierten Programmen). |

— PROMPT ENDET —
