# Prompt 005: Aktionen, Menü, Tastatur, Ablenkungsfrei

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 004. Legacy: §2 (Aktionen und Kürzel).
> Vorlage: comic-commander 008 (`action-catalog`, `keymap.*.json`, `KeyboardDispatcher`, `MenuService`, Befehlspalette).

## Umfang

- Aktionskatalog, Keymaps je OS und Dispatcher gibt es seit 004 (Design 004 E1). Hier: Overrides `keymap.json`
  im App-Config-Ordner (Rust lädt), Prüfung auf doppelte Belegungen.
- Menü: macOS nativ, sonst schlanke HTML-Menüleiste, die standardmäßig **versteckt** ist (Alt/F1 blendet ein).
- **Ablenkungsfrei** (legacy F1 „Toggle Menu, Tabs, Iconbar“, Strg+Umschalt+F1 „Fade in/out“): Menü, Tabs und
  Statusleiste einzeln und gemeinsam ein-/ausblenden; Vollbild.
- Befehlspalette (Mod+Shift+P) mit Suche über alle Aktionen und ihre Kürzel; „Tastenkürzel anzeigen“
  (legacy Show Keystrokes, Strg+Umschalt+K).
- About-Dialog.
