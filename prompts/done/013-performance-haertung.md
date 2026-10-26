# Prompt 013: Performance und Härtung

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 002–010.

## Umfang

- Messungen (ignored-Tests + Skript) mit 1 GB, 10 Mio. Zeilen, einer 100-MB-Zeile, Datei mit gemischten Zeilenenden:
  Öffnen, Springen, Tippen, Suchen, Ersetzen-alle, Sortieren, Speichern, Speicherbedarf. Ergebnisse in
  `docs/architecture.md` und auf der Webseite.
- Engpässe beheben; Grenzen festlegen (z. B. maximale Dateigröße, Warnung ab X).
- Review durch einen separaten Agenten (Dateiverlust beim Speichern, Encoding, Undo, IPC-Grenzen, CSP).
- Paritätsmatrix gegen den Java-Editor (legacy §2) mit Testbelegen; Lücken → Folge-Ideen.
