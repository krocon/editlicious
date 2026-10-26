# Design 005: Menü, Befehlspalette, Ablenkungsfrei

Status: umgesetzt · Datum: 26.10.2026

## Entscheidungen

- **E1 Ablenkungsfrei** (`ChromeStore`): F1 blendet Menüleiste, Tabs (und ab 010 die Statusleiste) gemeinsam aus/ein
  (legacy F1); Vollbild F11 (Windows/Linux) bzw. ⌘⌃F (macOS) über `toggle_fullscreen`. Statt Icon-Leiste: Befehlspalette.
- **E2 HTML-Menüleiste (Windows/Linux) standardmäßig versteckt**: ein einzelnes Alt öffnet sie für einen Gang
  (Escape oder Auswahl blendet wieder aus), „Ansicht › Menüleiste anzeigen“ (Strg+M, legacy) lässt sie stehen.
  F10 bleibt „nächste Datei“ (legacy) – deshalb Alt statt F10 für das Menü.
- **E3 macOS: natives Menü aus demselben Modell** (`app_menu.rs` aus comic-commander), **ohne** das vordefinierte
  Edit-Menü: dessen Cocoa-Selektoren (undo:, paste:) gingen an das versteckte Textfeld statt an die Engine. Unser
  Edit-Menü führt die Engine-Aktionen aus. Einschränkung: Textfelder in Dialogen (Palette) haben auf macOS kein
  ⌘V über das Menü. „Beenden“ fragt nach ungespeicherten Dateien (Ereignis statt `app.exit`).
- **E4 Befehlspalette** (Mod+Umschalt+P, APG-Combobox) aus comic-commander; Cursor-Aktionen ausgeblendet.
- **E5 `keymap.json`** im App-Config-Ordner (Rust lädt nachsichtig, `keymap_file.rs`), Frontend normalisiert und
  meldet unbekannte Aktionen/Kürzel in der Konsole; „Tastenkürzel“ (Mod+Umschalt+K, legacy „Show Keystrokes“) zeigt
  alle aktuellen Kürzel filterbar.
- **E6 Dynamische Einträge:** „Zuletzt geöffnet“ (10) und „Wechseln zu“ (Mod+1…9) als Aktionen mit festen Ids und
  Namen aus dem `DocumentsStore`; Tabs-Modus als Radio-Gruppe.
- **E7 Einstellungen** (Mod+,) sind als Aktion da, aber bis 009 deaktiviert. Die Chrome-Zustände werden in 009 in den
  Einstellungen gespeichert.

## Tests

- `menu-model.spec` (macOS vs. andere, zuletzt/offen, Auflösen + Spec ohne `shortcut`, `menuChord`, Radio),
  Dispatcher (Alt allein), `app.spec` (Menü erst nach Alt, Palette listet Speichern mit Kürzel und keine Cursor-Aktion,
  Tastenkürzel-Dialog), Rust: übernommene `app_menu`/`keymap_file`-Tests.
- Ende-zu-Ende (Xvfb): Alt + ↓ öffnet das Datei-Menü mit „Zuletzt geöffnet“, Palette filtert und führt „Tabs: always“
  aus, Tastenkürzel-Dialog. Nicht im Container prüfbar: natives macOS-Menü.
