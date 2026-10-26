# Release und Distribution

> Stand: Prompt 012, übernommen aus comic-commander-tauri (Prompt 012) bzw. files-and-folders-tauri (Design 017). **Tags, Veröffentlichung und Signierung nur mit Marcs ausdrücklicher
> Freigabe.** Lizenz: **MIT** (`LICENSE`, `license` in `Cargo.toml`/`package.json`, `bundle.license` in
> `tauri.conf.json`).

## Version – eine Quelle

| Datei | Version |
|---|---|
| `package.json` | **Quelle** (`"version"`) |
| `src-tauri/tauri.conf.json` | `"version": "../package.json"` – Tauri liest sie dort (App, Pakete, About) |
| `src-tauri/Cargo.toml`, `Cargo.lock` | Kopie, gesetzt von `scripts/version.mjs` |

```bash
pnpm version:set 0.2.0      # schreibt package.json, Cargo.toml, Cargo.lock (kein Commit, kein Tag)
pnpm version:check          # Teil von `pnpm check` und beider Workflows
```

## Ablauf eines Releases (Checkliste)

1. `main` ist grün (Workflow **CI**: Frontend + Rust auf Linux, macOS, Windows).
2. `pnpm version:set X.Y.Z`, `pnpm check`, Commit `chore: release X.Y.Z`, pushen.
3. Release Notes: `pnpm release:notes` (Conventional Commits seit dem letzten Tag, gruppiert: Breaking, Features,
   Fixes, Performance, Doku; `chore`/`test`/`ci` ausgeblendet). Vor dem ersten Tag: die ganze Historie.
4. Tag pushen – `git tag vX.Y.Z && git push origin vX.Y.Z` – **oder** GitHub → Actions → **Release** → Run workflow.
   Der Workflow prüft, dass Tag und `package.json` übereinstimmen, baut auf drei Runnern und legt einen
   **Entwurf** an (`editlicious vX.Y.Z`, Versionen `0.x` als Pre-release).
5. Pakete aus dem Entwurf installieren und starten (s. „Prüfen“), Notes einfügen, Entwurf veröffentlichen.

Dauer: ca. 15–25 min (macOS universal ist der langsamste Runner). Ein fehlgeschlagener Runner lässt die anderen
weiterlaufen (`fail-fast: false`); erneut starten über „Re-run failed jobs“.

## Pakete je OS (`tauri.conf.json` → `bundle`)

| OS | Pakete | Einstellungen |
|---|---|---|
| macOS | `.app`, `.dmg` (universal: Apple Silicon + Intel) | `minimumSystemVersion` **12.0**, `hardenedRuntime`, DMG-Fenster 660×400 |
| Windows | `.msi` (WiX), `-setup.exe` (NSIS) | NSIS `installMode: currentUser` (keine Admin-Rechte), Englisch/Deutsch; WebView2 per Bootstrapper; feste WiX-`upgradeCode` `0caa1787-…` (**nie ändern** – sonst ersetzt ein Update die alte Version nicht) |
| Linux | `.deb`, `.rpm`, `.AppImage` | Abhängigkeiten (`libwebkit2gtk-4.1-0`, `libgtk-3-0`) ergänzt Tauri; deb-Abschnitt `editors`; AppImage ohne GStreamer. Gebaut auf `ubuntu-22.04` (ältere glibc) |

Lokal: `pnpm tauri:build:mac` (universal, braucht `rustup target add aarch64-apple-darwin x86_64-apple-darwin`),
`pnpm tauri:build:mac:local` (nur die eigene Architektur), `pnpm tauri:build:linux`, `pnpm tauri:build:windows`.

## Dateizuordnung („Öffnen mit editlicious“)

| OS | Wie | Wirkung |
|---|---|---|
| macOS | `bundle.fileAssociations` (Rang `Alternate`) → `CFBundleDocumentTypes` | erscheint unter „Öffnen mit“, wird **nicht** Standard. Finder schickt die Dateien als `RunEvent::Opened` – auch an die laufende App |
| Windows | Tauri-Zuordnungen **aus** (`tauri.windows.conf.json`), stattdessen NSIS-Hook `src-tauri/windows/open-with.nsh` | trägt die App unter `HKCU\Software\Classes\Applications\editlicious.exe` und in die `OpenWithList` der Endungen ein – „Öffnen mit“, ohne Notepad & Co. als Standard zu verdrängen; Deinstallation räumt auf. Das MSI trägt nichts ein |
| Linux | `fileAssociations` → `MimeType=text/plain` in der `.desktop`-Datei | im Dateimanager unter „Öffnen mit“ |

Ein zweiter Start (Windows/Linux, „Öffnen mit“ bei laufender App) reicht seine Dateien per
`tauri-plugin-single-instance` an die laufende App weiter (Ereignis `open-files`) und holt das Fenster nach vorn.
Dateien, die vor dem Start der Oberfläche ankommen, warten in Rust und kommen mit `startup_paths`.

## Lizenzhinweise

**`THIRD-PARTY-NOTICES.txt`** im Repo-Wurzelordner, erzeugt mit `pnpm notices` (`scripts/notices.mjs`): die
Lizenztexte aller ausgelieferten Teile – Rust-Crates über normale Abhängigkeiten aller Zielplattformen
(`cargo metadata`), npm-Laufzeitpakete, dazu fest die Schrift **Yellowtail** (Astigmatic, Apache-2.0), aus deren
Umrissen der Schriftzug besteht (`scripts/notices/yellowtail.txt`).

Die Datei liegt in jedem Paket (`bundle.resources`). `pnpm check` und der Release-Workflow prüfen mit
`pnpm notices:check`, dass sie aktuell ist – **nach jeder Änderung an Abhängigkeiten `pnpm notices` ausführen**
und die Datei mit committen.

## Signierung und Notarisierung (vorbereitet, nicht eingerichtet)

Ohne Secrets baut `release.yml` **unsignierte** Pakete: macOS – Rechtsklick → Öffnen (bzw. Systemeinstellungen →
Datenschutz & Sicherheit → „Trotzdem öffnen“); Windows – SmartScreen „Weitere Informationen“ → „Trotzdem ausführen“.
Secrets unter GitHub → Settings → Secrets and variables → Actions. Dieselben Zertifikate wie bei Files and Folders
und Comic Commander gehen auch hier (Schritt für Schritt: files-and-folders-tauri `docs/macos-signing.md`).

### macOS – Developer ID + Notarisierung

| Secret | Inhalt |
|---|---|
| `APPLE_CERTIFICATE` | `.p12` des Developer-ID-Application-Zertifikats, base64 (`base64 -i cert.p12`) |
| `APPLE_CERTIFICATE_PASSWORD` | Passwort des `.p12` |
| `APPLE_SIGNING_IDENTITY` | z. B. `Developer ID Application: … (TEAMID)` |
| `APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_PRIVATE_KEY` | empfohlen: App-Store-Connect-API-Schlüssel – Issuer-ID, Schlüssel-ID, Inhalt der `.p8` |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | alternativ: Apple-ID, app-spezifisches Passwort, Team-ID |

Nur gesetzte Secrets werden an den Bundler gegeben; Tauri signiert mit Hardened Runtime und notarisiert.
Entitlements braucht die App nicht (keine Sandbox).

### Windows – Code Signing

| Secret | Inhalt |
|---|---|
| `WINDOWS_CERTIFICATE` | `.pfx`, base64 |
| `WINDOWS_CERTIFICATE_PASSWORD` | Passwort des `.pfx` |

Der Workflow importiert es und setzt `bundle.windows.certificateThumbprint`; Tauri signiert `.exe`, `.msi` und den
NSIS-Installer (SHA-256, Zeitstempel DigiCert). Schlüssel auf Hardware/HSM (heute üblich): stattdessen
`bundle.windows.signCommand` (z. B. Azure Trusted Signing) – offen.

### Linux

Keine Signatur nötig.

## Offen

- **Auto-Update** (`tauri-plugin-updater`) erst nach Signierung und dem ersten öffentlichen Release
  (`TAURI_SIGNING_PRIVATE_KEY` steht auskommentiert im Workflow).
- Windows-MSI ohne „Öffnen mit“ (nur NSIS) – bei Bedarf WiX-Fragment.

## Prüfen (Akzeptanz)

| OS | Was | Stand |
|---|---|---|
| Linux | `.deb` installieren (`apt install ./…deb`), starten, About zeigt die Version; `.desktop` mit `MimeType` | im Container gebaut und installiert (Prompt 012) |
| macOS | `.dmg` öffnen, App nach Programme ziehen, starten (unsigniert: Rechtsklick → Öffnen) | offen – erster Lauf des Workflows |
| Windows | `-setup.exe` ohne Admin installieren, starten; `.msi` installieren/deinstallieren | offen – erster Lauf des Workflows |
