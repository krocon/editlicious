# Prompt 005: Aktionen, Menü, Tastatur, Ablenkungsfrei

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 004. Legacy: §2 (Aktionen und Kürzel).
> Vorlage: comic-commander 008 (`action-catalog`, `keymap.*.json`, `KeyboardDispatcher`, `MenuService`, Befehlspalette).

## Umfang

- Aktionskatalog mit allen bisherigen Funktionen; Keymaps je OS (`Mod` = Cmd/Strg), Overrides `keymap.json`.
- Menü: macOS nativ, sonst schlanke HTML-Menüleiste, die standardmäßig **versteckt** ist (Alt/F1 blendet ein).
- **Ablenkungsfrei** (legacy F1 „Toggle Menu, Tabs, Iconbar“, Strg+Umschalt+F1 „Fade in/out“): Menü, Tabs und
  Statusleiste einzeln und gemeinsam ein-/ausblenden; Vollbild.
- Befehlspalette (Mod+Shift+P) mit Suche über alle Aktionen und ihre Kürzel; „Tastenkürzel anzeigen“
  (legacy Show Keystrokes, Strg+Umschalt+K).
- About-Dialog.
