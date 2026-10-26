# Prompt 014: Release-Pipeline

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 012. Vorlage: Release-Pipeline von
> comic-commander-tauri (Prompt 012, Release 1.0.1) und files-and-folders-tauri (Design 017).
> Auftrag (Marc, 26.10.2026): „Release-Pipeline bauen, analog zu Comic Commander oder Files and Folders; alles in main.“

## Ausgangslage

Prompt 012 hat die Bausteine übernommen: eine Version (`scripts/version.mjs`), Release Notes
(`scripts/release-notes.mjs`), Lizenzhinweise (`scripts/notices.mjs`), `.github/workflows/release.yml` (Tag `v*` →
Entwurf auf drei Runnern, Signierung vorbereitet) und `docs/release.md`. Gelaufen ist der Workflow noch nie – NSIS-Hook,
macOS-Universal-Build und Linux-Pakete sind ungeprüft. Releases selbst gibt es nur auf Marcs Anweisung (CLAUDE.md §6).

## Umfang

1. **Probelauf ohne Release** (`workflow_dispatch`, Modus `dry-run`, Standard): baut alle Pakete auf macOS, Windows,
   Linux und lädt sie als Workflow-Artefakte hoch – kein Tag, kein Release, kein Entwurf. Damit lässt sich die
   Pipeline jederzeit prüfen. Modus `draft` (manuell) und Tag `v*` legen wie bisher einen Entwurf an.
2. **Vorabprüfung** (Job `check`): Versionen gleich, Tag = Version, `scripts:test`, Lizenzhinweise aktuell,
   Release Notes für den Bereich seit dem letzten Tag erzeugen und als Ausgabe weiterreichen.
3. **Release Notes automatisch** in den Entwurf (statt „hier einfügen“), mit Hinweis auf unsignierte Pakete und
   Prüfsummen.
4. **Paketprüfung je OS** nach dem Bauen: erwartete Dateien vorhanden (dmg/app, msi + setup.exe, deb/rpm/AppImage),
   `.deb` enthält Programm, `.desktop` mit `%F`/`MimeType` und `THIRD-PARTY-NOTICES.txt`; Größen in der
   Zusammenfassung des Laufs.
5. **Prüfsummen**: `SHA256SUMS.txt` über alle Pakete – als Artefakt und im Entwurf.
6. **Lizenzbericht** `docs/licenses.md` (lesbare Übersicht, aus files-and-folders `scripts/licenses.mjs`) mit Prüfung,
   dass jede Lizenz bekannt/erlaubt ist.
7. **Probelauf wirklich ausführen** (kein Release) und Fehler beheben, bis alle drei Runner grün sind.
8. Doku: `docs/release.md` (Modi, Ablauf, Artefakte, Prüfsummen, was Marc für das erste Release tun muss),
   CLAUDE.md, README, Prompt-Status.

## Nicht im Umfang

- Ein echtes Release, Tag oder veröffentlichter Entwurf (nur auf Marcs Anweisung).
- Signier-Zertifikate einrichten (Secrets hat nur Marc), Auto-Update (erst nach Signierung, s. `docs/release.md`).
- Einträge in Paketmanagern/Verzeichnissen (Comic Commander Prompt 014 „Distribution“) – eigener Folge-Prompt.

## Akzeptanz

- Workflow „Release“ im Modus `dry-run` läuft auf allen drei Runnern grün und liefert Pakete + `SHA256SUMS.txt`
  als Artefakte; die Zusammenfassung listet Dateien und Größen.
- `pnpm check` grün (inkl. neuer Skript-Tests), `docs/licenses.md` aktuell.
