# Prompt 012: Release und Distribution

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 001. Vorlage: comic-commander 012.

## Umfang

- `scripts/version.mjs` (eine Version aus `package.json`), `release-notes.mjs`, `notices.mjs` → `THIRD-PARTY-NOTICES.txt`
  (im Paket), Tests `scripts/*.test.mjs`.
- Pakete je OS (dmg/app, msi/nsis, deb/rpm/AppImage), Dateizuordnung für Textdateien (öffnen mit editlicious).
- `.github/workflows/release.yml`: Tag `v*` → Release-Entwurf; Signierung vorbereitet; `docs/release.md`.
