# Design 009: Einstellungen und Sprache

## Entscheidungen

- **E1 settings.json** (`src-tauri/src/settings.rs`): im App-Config-Ordner, atomar geschrieben, nachsichtig geladen
  (unbekannte Felder ignoriert, fehlende = Standard, Werte begrenzt: Schrift 6–72, Tab 1–16). Kaputte Datei →
  Standardwerte + Warnung, die Datei wird erst beim nächsten Ändern überschrieben.
- **E2 Sofort anwenden, gebündelt speichern**: `SettingsStore` (Root) hält ein Signal; jede Änderung wirkt sofort
  (Sprache, CSS-Variablen `--editor-font-size`/`--font-editor`, Chrome), gespeichert wird 400 ms später und beim
  Beenden. Kein „OK/Abbrechen“ – der Dialog hat nur „Schließen“.
- **E3 Chrome-Zustand**: Farbschema, Menüleiste, Statusleiste bleiben zur Laufzeit im `ChromeStore` (F12, Strg+M)
  und werden in die Einstellungen zurückgeschrieben. Die Übernahme Einstellungen → Chrome hängt an einem
  `computed` nur dieser Felder, damit z. B. Zoom ein frisches F12 nicht zurücksetzt (Test).
- **E4 Einrückung**: „wie die Datei“ (`detect`) – ist die Datei mit Leerzeichen eingerückt, gelten deren Breite und
  Leerzeichen; mit Tabs: Tabs, Breite aus den Einstellungen; ohne Einrückung: die Einstellungen. Engine-Befehl
  `configure { tabWidth, insertSpaces, detect }` bei jedem Sitzungswechsel und jeder Änderung.
- **E5 Einrückungslinien** zeichnet die UI aus `FrameLine.indent` (Breite der führenden Leerzeichen/Tabs in
  Zellen, Leerzeilen erben die Zeile darüber) – nichts Dateigrößenabhängiges über die Brücke. Lange Zeilen: 0.
- **E6 Zeilennummern** absolut / relativ zum Cursor / aus (dann schmaler Rand). Strg+Umschalt+L schaltet durch,
  Strg+Alt+L Einrückungslinien (legacy).
- **E7 Zoom**: Strg+Plus / Strg+= / Strg+Minus / Strg+0 (Ziffernblock eingeschlossen, da `key` gleich ist);
  nach Schriftwechsel misst die Ansicht die Zellbreite neu und schickt `resize`.
- **E8 Sprache** zur Laufzeit ohne Bibliothek (`translatable()`-Proxy aus files-and-folders 015 E5): Englisch
  ist Quelle, Deutsch daneben, „auto“ = Systemsprache. Aktionsnamen deutsch in `action-catalog.de.ts`;
  `ActionService.label()` liefert den Namen in der aktuellen Sprache für Menüs, Palette und Kürzelliste.
  `catalogs.spec.ts` prüft gleiche Schlüssel/Signaturen und deutschen Namen für jede Aktion.
- **E9 Neue Dateien**: Zeilenende LF / CRLF / wie das System (`new_document(lineEnding)`); „letzte Dateien
  laden“ abschaltbar (Verlauf und Suchhistorie bleiben).

## Offen / später

- Umbruch (Soft Wrap) folgt mit 010 (Einstellung `wordWrap` ist schon da).
- Fragen beim Schließen bleiben immer an (Speichern ist heilig).
