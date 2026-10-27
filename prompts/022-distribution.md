# Aufgabe: Meta-Prompt für die automatische Distribution von editlicious über den Browser erstellen

> Analog zu comic-commander-tauri `prompts/014-distribution.md`. Ergebnis dort: `comic-commander-tauri/distribution/`
> (`browser-agent-prompt.md`, `facts.json`, `listing.md`, `manifests/…`, Bericht) – **als Vorlage für Aufbau und
> Tiefe nutzen**, aber jede Angabe für editlicious neu prüfen. Nichts aus Comic Commander ungeprüft übernehmen.

## Deine Rolle

Du bist ein erfahrener Agent-Workflow-Architekt, spezialisiert auf Claude, Claude Browser, Browser-Automatisierung,
Software-Distribution, Paketmanager und Developer-Marketing.

Deine Aufgabe ist **nicht**, editlicious selbst einzutragen. Du sollst einen vollständigen, ausführbaren Prompt für
Claude Browser schreiben, der anschließend die Registrierung und Einreichung von editlicious bei möglichst vielen
geeigneten Paketmanagern, Software-Verzeichnissen und Discovery-Plattformen übernimmt.

Der von dir erstellte Prompt soll so konkret sein, dass Claude Browser ihn ohne weitere konzeptionelle Vorbereitung
abarbeiten kann.

Was du aber tun kannst: eine Liste von Paketmanagern, Software-Verzeichnissen und Discovery-Plattformen erstellen und
im Prompt angeben, die für editlicious in Frage kommen, sowie eine Liste der tatsächlich vorhandenen, stabilen
Download-URLs (aus den GitHub-Releases, nicht geraten).

Du kannst hier im Projekt die entsprechenden Manifest-Dateien anlegen, die Claude Browser dann zusammen mit dem
geschriebenen Prompt nutzt.

## Projekt

**Produkt:** editlicious
**Offizielle Produktseite:** https://krocon.de/products/editlicious/
**Live-Demo (Engine als WebAssembly):** https://krocon.de/products/editlicious/demo/
**Quellcode:** https://github.com/krocon/editlicious
**Releases:** https://github.com/krocon/editlicious/releases/
**Hersteller-/Entwicklerwebsite:** https://krocon.de/
**GitHub-Profil:** https://github.com/krocon

editlicious ist ein ruhiger, tastaturgesteuerter Texteditor für macOS, Windows und Linux, der **sehr große Dateien**
öffnet und bearbeitet (Millionen Zeilen, Zeilen mit Millionen Zeichen). Tauri 2 + Angular 22 + Rust (Engine
`ed-core`), MIT. Prüfe die tatsächlichen Produktinformationen auf der offiziellen Website, im README und in den
Releases. Erfinde keine Funktionen, Plattformen, Lizenzdetails, Versionsnummern oder Download-URLs.

Bekannt aus dem Repo (vor Verwendung gegen Release und Produktseite prüfen, Abweichungen melden):

- Tags `v0.1.0`, `v1.0.0`; Version in `package.json` (einzige Quelle, `docs/release.md`).
- Pakete laut `src-tauri/tauri.conf.json` und `docs/release.md`: macOS `.dmg`/`.app` (universal, ab macOS 12),
  Windows `.msi` (WiX, feste `upgradeCode`) und `-setup.exe` (NSIS, `currentUser`), Linux `.deb`, `.rpm`,
  `.AppImage` (x86-64, gebaut auf Ubuntu); `SHA256SUMS.txt` am Release.
- Bundle-Identifier `io.github.krocon.editlicious`, Kategorie `DeveloperTool`, deb-Abschnitt `editors`,
  `.desktop` mit `MimeType=text/plain` und `%F`.
- **Nicht signiert / nicht notarisiert** (macOS: Rechtsklick → Öffnen; Windows: SmartScreen) – laut Produktseite.
- Jedes Paket enthält das Sidecar-Programm `editlicious-mcp` (KI-Steuerung über MCP, standardmäßig aus,
  `docs/mcp.md`) und die CLI `editlicious render` (Video-Export, braucht externes ffmpeg).
- Lizenzhinweise: `THIRD-PARTY-NOTICES.txt` (im Paket), `docs/licenses.md`.

Lies Paketdetails (Binärnamen, Desktop-Datei, Icon-Name, deb-Abhängigkeiten, MSI-`ProductCode`, Größen, SHA-256)
**aus den echten Release-Assets**, nicht aus dem Gedächtnis oder aus Comic Commander.

## Hauptziel

Erstelle einen Browser-Agent-Prompt, der selbstständig:

1. editlicious und seine tatsächlichen technischen Eigenschaften recherchiert.
2. Die geeigneten Paketmanager, Software-Kataloge und Discovery-Plattformen identifiziert.
3. Alle Einreichungsmöglichkeiten über Weboberflächen, GitHub-Webseiten oder offizielle Einreichungsformulare ermittelt.
4. Die notwendigen Einträge vorbereitet und – soweit autorisiert, technisch möglich und ohne unbelegte Angaben –
   einreicht.
5. Den Fortschritt, die Ergebnisse, offene Voraussetzungen und erforderliche manuelle Aktionen dokumentiert.

**Wichtig:** Das Ziel ist nicht, möglichst viele beliebige Einträge zu erzeugen, sondern möglichst viele qualitativ
hochwertige, dauerhaft funktionierende und passende Distributionseinträge.

## Zu prüfende Plattformen

Der erstellte Prompt soll mindestens die folgenden Kandidaten überprüfen und bei Eignung bearbeiten.

### A. Paketmanager

**macOS**
- Homebrew Cask: https://github.com/Homebrew/homebrew-cask (aktuelle Regeln zu Signierung/Notarisierung und
  Bekanntheit prüfen; Alternative: eigener Tap `krocon/homebrew-tap`)
- MacPorts: https://www.macports.org/

**Windows**
- WinGet: https://github.com/microsoft/winget-pkgs (NSIS-User- und MSI-Installer, `ProductCode`, Upgrade-Verhalten)
- Scoop: https://github.com/ScoopInstaller (Bucket `Extras`, Regeln für Bekanntheit; sonst eigener Bucket)
- Chocolatey: https://community.chocolatey.org/

**Linux**
- Flathub: https://flathub.org/ (App-Id `io.github.krocon.editlicious` passt zur Verifizierung über GitHub;
  AppStream-`metainfo.xml`, Build aus Quellen vs. Wiederverwendung des .deb, Flathub-Regeln dazu prüfen)
- Snap Store: https://snapcraft.io/
- AppImageHub (`AppImage/appimage.github.io`) beziehungsweise geeignete AppImage-Verzeichnisse
- Debian-/Ubuntu-Paketarchive, sofern realistisch und technisch geeignet (Node-/Cargo-Abhängigkeiten!)
- Fedora-Paketquellen bzw. COPR, sofern realistisch und technisch geeignet
- AUR für Arch Linux (`editlicious-bin` aus dem .deb; ggf. später `editlicious` aus Quellen)

**Sprachspezifisch / Entwickler**
- crates.io: prüfen, ob eine Veröffentlichung von `ed-core` überhaupt sinnvoll und gewünscht ist (nur als Frage an
  Marc, nicht selbstständig veröffentlichen – Namensvergabe ist praktisch unumkehrbar).

Prüfe für jede Plattform, ob editlicious die technischen Voraussetzungen erfüllt. Berücksichtige Tauri (WebView2,
WebKitGTK 4.1), die tatsächlich vorhandenen Build-Artefakte, unterstützte Betriebssysteme, Architektur (Linux nur
x86-64?), Signierung, Lizenz, das mitgelieferte Sidecar `editlicious-mcp` und das Installationsverfahren.

Unterscheide zwischen:
- direkter Einreichung über ein Webformular,
- Einreichung per Pull Request über die Weboberfläche,
- erforderlicher Veröffentlichung eines Git-Repositories (Tap, Bucket, COPR),
- notwendiger Konfiguration eines externen Build- oder Distributionssystems,
- Plattformen, bei denen eine echte Browser-Einreichung nicht ausreicht.

Erfinde keine APIs, Einreichungsformulare oder angeblichen Ein-Klick-Registrierungen.

### B. Software-Verzeichnisse

Prüfe unter anderem:

- AlternativeTo: https://alternativeto.net/ (als Alternative zu passenden Editoren/Viewern großer Dateien eintragen –
  welche, recherchiert der Agent; nur ehrliche Vergleiche)
- SaaSHub: https://www.saashub.com/
- MacUpdate: https://www.macupdate.com/
- Softpedia: https://www.softpedia.com/
- FileHorse: https://www.filehorse.com/
- MajorGeeks: https://www.majorgeeks.com/
- SourceForge: https://sourceforge.net/ (Spiegel der GitHub-Releases)
- FossHub: https://www.fosshub.com/
- Uptodown: https://www.uptodown.com/
- OpenAlternative: https://openalternative.co/
- Slant: https://www.slant.co/ (Fragen wie „best text editor for large files“)

Füge weitere seriöse, geeignete und aktuell aktive Portale hinzu. Prüfe die jeweiligen Richtlinien und tatsächlichen
Möglichkeiten, neue Software einzureichen.

### C. Produkt-Launch und Community-Discovery

Prüfe:

- Product Hunt: https://www.producthunt.com/
- Hacker News / Show HN: https://news.ycombinator.com/ (Live-Demo im Browser ist ein starker Aufhänger)
- Reddit-Communities mit passenden Regeln zur Eigenwerbung (z. B. r/rust, r/opensource, r/macapps, r/linux,
  r/software, r/coolgithubprojects – Regeln jeweils aktuell lesen)
- This Week in Rust (Abschnitt für Projekt-Updates, Einreichung per PR auf GitHub)
- kuratierte Listen für Open-Source-macOS-, -Windows- und -Linux-Apps
- kuratierte Listen für Rust- und Tauri-Desktop-Software (z. B. awesome-tauri, awesome-rust)
- kuratierte Listen für Texteditoren
- Listen/Verzeichnisse für MCP-Server: **nur** prüfen, ob ein in eine Desktop-App eingebautes Sidecar dort überhaupt
  passt (viele Register erwarten ein eigenständig installierbares Paket); nicht passend → begründet ausschließen

Diese Plattformen sollen getrennt von klassischen Paketmanagern behandelt werden. Ein Community-Post ist kein
Softwarepaket und kein offizieller Katalogeintrag.

## Arbeitsweise des zu erstellenden Browser-Agent-Prompts

### Phase 1: Bestandsaufnahme

Der Browser-Agent soll zunächst die offizielle Produktseite, die Live-Demo, das README und die Release-Seite öffnen.

Er soll feststellen:
- Produktname und offizielle Beschreibung,
- tatsächliche Features (nur was Produktseite/README belegen),
- unterstützte Betriebssysteme und Architekturen,
- Lizenz,
- Quellcode-Repository,
- aktuelle Version (neuester veröffentlichter, nicht Entwurfs-Release),
- vorhandene Releases und Installer,
- stabile und dauerhaft erreichbare Download-URLs,
- vorhandene Prüfsummen (`SHA256SUMS.txt`),
- Signierungs- und Notarisierungsstatus,
- Screenshots und Produktgrafiken (`site/img/`, Icon `src-tauri/icons/`),
- Kontaktmöglichkeiten des Entwicklers (nur öffentlich angegebene).

Fehlende Angaben müssen als unbekannt dokumentiert werden. Der Agent darf keine Angaben erfinden.

Falls notwendige Informationen fehlen, soll er zunächst alle anderen vorbereitenden Arbeiten durchführen und
anschließend gezielt nur die wirklich erforderlichen Rückfragen stellen.

### Phase 2: Plattforminventar und Priorisierung

Der Agent soll eine vollständige Arbeitsliste mit mindestens folgenden Feldern erstellen:

- Plattform
- Kategorie
- offizielle Einreichungs-URL
- Einreichungsmethode
- technische Voraussetzungen
- erforderliche Accounts
- Kosten, sofern verlässlich feststellbar
- benötigte Dateien und Metadaten
- Status
- konkrete nächste Aktion
- erwarteter Nutzen
- Begründung für Aufnahme oder Ausschluss

Priorisiere nach Reichweite, Relevanz für Entwickler und Nutzer großer Dateien (Logs, Datenexporte, CSV/JSON),
Einreichungsaufwand und Erfolgsaussicht.

Prüfe, ob die Plattform aktuell aktiv ist. Verlasse dich nicht allein auf Suchmaschinenausschnitte oder veraltete
Anleitungen.

### Phase 3: Vorbereitung der Einträge

Bereite für jede geeignete Plattform die benötigten Inhalte vor, soweit die tatsächlichen Projektdaten dies erlauben:

- Produktname (immer klein geschrieben: „editlicious“)
- kurze und lange Beschreibung
- englische Beschreibung und gegebenenfalls deutsche Beschreibung
- offizielle Website
- Repository-URL
- Lizenz
- aktuelle Version
- unterstützte Betriebssysteme
- passende Kategorien und Tags
- Release- und Download-URLs
- Prüfsummen
- Screenshots und gegebenenfalls Icon
- versionsspezifische Installationsanweisungen (inkl. Hinweis auf fehlende Signierung)

Nutze konsistente Produktinformationen. Erstelle keine widersprüchlichen Versionsnummern, erfundenen Download-Links
oder unbelegten Feature-Aussagen. Leistungsangaben (Messwerte der Produktseite) nur mit deren Rahmen
(Release-Build, Messmaschine) übernehmen.

Wo sinnvoll, bereite die erforderlichen Paketmanifest-Dateien und Einreichungsbeschreibungen vor. Der Browser-Agent
soll jedoch keine nicht vorhandenen Dateien oder Release-Artefakte vortäuschen.

### Phase 4: Browser-Einreichungen

Der Browser-Agent soll jede geeignete Plattform nacheinander öffnen und die offiziellen Einreichungsprozesse
bearbeiten.

Er soll:
- vorhandene Accounts verwenden, sofern der Nutzer bereits angemeldet ist,
- Formulare vollständig ausfüllen,
- korrekte Projektinformationen verwenden,
- bereits vorhandene editlicious-Einträge erkennen (auch gleichnamige fremde Projekte – nicht verwechseln),
- Duplikate vermeiden,
- Einreichungen und Pull Requests nachvollziehbar benennen,
- Validierungsfehler beheben, sofern das ohne unbelegte Angaben möglich ist,
- den tatsächlichen Status nach jeder Aktion prüfen.

**Keine blinden Wiederholungen:** Wenn eine Einreichung fehlschlägt, muss der Agent den Fehler untersuchen, bevor er
es erneut versucht.

### Phase 5: Freigaben und Sicherheitsgrenzen

Der Agent darf normale, reversible Formularvorbereitungen und eindeutig autorisierte Einreichungen selbstständig
ausführen.

Vor folgenden Aktionen muss er die ausdrückliche Zustimmung des Nutzers einholen:
- kostenpflichtige Registrierungen oder Bestellungen,
- Veröffentlichung eines öffentlichen Community-Posts im Namen des Nutzers,
- rechtlich verbindliche Erklärungen oder Lizenzbestätigungen,
- Änderungen an bestehenden öffentlichen Releases,
- Anlegen öffentlicher Repositories (Tap, Bucket) oder Reservieren von Paketnamen (crates.io, npm),
- Löschungen oder Änderungen bestehender Einträge mit möglichen Auswirkungen,
- Akzeptieren von Bedingungen, die über gewöhnliche Plattformnutzung hinausgehen,
- Nutzung oder Weitergabe persönlicher Daten, die nicht ausdrücklich freigegeben wurden,
- Erstellen neuer Zugangsdaten oder Änderungen an Sicherheitskonfigurationen.

Der Agent darf niemals:
- Captchas oder Sicherheitsmechanismen umgehen,
- Passwörter oder Zugangsdaten ausgeben,
- Sicherheitswarnungen ignorieren,
- kostenpflichtige Optionen ungefragt aktivieren,
- falsche Angaben machen, um eine Plattformprüfung zu bestehen (z. B. Signierung behaupten),
- eine Einreichung als erfolgreich melden, wenn sie lediglich vorbereitet wurde.

Falls ein Login, eine MFA-Bestätigung oder eine Freigabe erforderlich ist, soll der Agent anhalten, den Nutzer um die
notwendige Aktion bitten und danach fortfahren.

### Phase 6: Abschlussbericht

Der Agent soll einen strukturierten Abschlussbericht erstellen mit:

1. Erfolgreich eingereichten Einträgen
2. Noch in Prüfung befindlichen Einträgen
3. Vorbereiteten, aber noch nicht eingereichten Einträgen
4. Plattformen mit technischen Blockern (z. B. Signierung, Notarisierung, ARM-Linux-Pakete)
5. Abgelehnten Einreichungen und den bekannten Gründen
6. Erforderlichen manuellen Aktionen
7. Nicht geeigneten Plattformen mit Begründung
8. Direkten Links zu Einträgen, Pull Requests und offenen Formularen
9. Empfohlenen nächsten Schritten

Erstelle zusätzlich eine Markdown-Datei `editlicious-distribution-report.md`, sofern die verwendete Umgebung Dateien
erstellen kann. Sie soll die Ergebnisse und die noch offenen Aufgaben dauerhaft dokumentieren.

## Qualitätskriterien

- Arbeite systematisch und sequenziell.
- Verwende ausschließlich offizielle Einreichungswege, soweit verfügbar.
- Prüfe die tatsächliche Eignung jedes Paketmanagers.
- Vermeide Duplikate.
- Bevorzuge kostenlose und seriöse Plattformen.
- Verwende Englisch für internationale Einträge, sofern sinnvoll.
- Berücksichtige, dass editlicious möglicherweise noch nicht alle Voraussetzungen für eine öffentliche Distribution
  erfüllt (Signierung, Bekanntheitsschwellen mancher Kataloge).
- Führe keine Aktionen aus, die durch fehlende Informationen oder Berechtigungen nicht gedeckt sind.
- Erfinde keine Ergebnisse und behaupte niemals einen Erfolg ohne überprüfbare Bestätigung.
- Dokumentiere jeden abgeschlossenen Schritt mit dem tatsächlichen Ergebnis.

## Ablage im Repo (für dich, nicht für den Browser-Agenten)

Wie in Comic Commander unter `distribution/`:

| Datei | Inhalt |
|---|---|
| `distribution/README.md` | Überblick, woher die Werte kommen (Datum, Release, nachgerechnete Summen), Kurzeinschätzung |
| `distribution/browser-agent-prompt.md` | **der Prompt**, zwischen „PROMPT BEGINNT/ENDET“ |
| `distribution/facts.json` | einzige Quelle der Fakten: Version, URLs, SHA-256, OS, Paketdetails, Unbekanntes |
| `distribution/listing.md` | Texte EN/DE (Tagline, kurz, lang), Kategorien, Tags, Show-HN-Entwurf |
| `distribution/manifests/…` | WinGet, Scoop, Homebrew Cask, Chocolatey, AUR (`PKGBUILD` + `.SRCINFO`), AppImageHub, Flathub (Manifest + `metainfo.xml`), Snap – nur für geeignete Plattformen, mit Status (getestet/ungetestet/blockiert) |

- Werte aus den echten Release-Assets (Größen, SHA-256 selbst nachrechnen); Manifeste, wo möglich, lokal prüfen
  (z. B. `makepkg --printsrcinfo`, `appstreamcli validate`, `winget validate` nur, wenn verfügbar – sonst als
  ungeprüft kennzeichnen).
- Der Browser-Agent liest die Dateien auf GitHub → `distribution/` muss auf `main` gepusht sein, bevor der Prompt
  läuft; im Prompt die Raw-URLs angeben.
- Nichts an Releases, Tags oder Workflows ändern. Fehlt etwas für eine Plattform (z. B. `metainfo.xml` im Paket,
  ARM-Builds), als Folge-Idee in `prompts/README.md` eintragen, nicht nebenbei umbauen.
- Abschluss wie immer: `pnpm check` grün, Commit `docs(distribution): browser agent prompt and package manifests`,
  Push auf `main`, Prompt nach `done/`, Status in `prompts/README.md`.

## Deine Ausgabe jetzt

Der fertige, in sich geschlossene Prompt für Claude Browser in `distribution/browser-agent-prompt.md` (plus die
Dateien oben). Er muss direkt kopierbar sein und den Browser-Agenten von der Recherche bis zum dokumentierten
Ergebnis führen. Er darf nicht lediglich eine Liste von Empfehlungen enthalten, sondern muss einen konkreten
Arbeitsablauf mit Entscheidungsregeln, Statusverwaltung, Fehlerbehandlung und Freigabepunkten definieren.
