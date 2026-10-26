# Prompt 008: Syntax-Hervorhebung und Farbschemata

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 003. Legacy: §6 (Farbstile, Grammatiken).

## Umfang

- Hervorhebung nur für den Viewport: Zustand am Zeilenanfang an Checkpoints cachen (alle N Zeilen), bei Änderungen
  ab der geänderten Zeile verwerfen. Für sehr lange Zeilen begrenzen (nur Fenster, Zustand ggf. ignorieren).
- Sprachen nach Endung (mind. JS/TS, JSON, HTML/XML, CSS, Java, Rust, Python, Markdown, Shell, SQL, YAML, Log-Dateien
  mit Leveln); Entscheidung `syntect` (TextMate-Grammatiken, wie legacy-Wunsch) vs. eigener schlanker Tokenizer.
- Farbschemata: mind. 2 helle und 2 dunkle (legacy: 4 eingebaute Stile), F12 schaltet weiter (legacy).
