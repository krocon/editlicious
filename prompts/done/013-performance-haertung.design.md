# Design 013: Performance und Härtung

## Entscheidungen

- **E1 Messungen** als ignorierter Test `measure_013` (`src-tauri/core/src/measure.rs`): Generator schreibt 1 GB /
  10 Mio. Zeilen, eine 100-MB-Zeile und 1 Mio. Zeilen mit gemischten Zeilenenden; jeder Schritt läuft über die
  Befehle der Sitzung wie in der App und meldet Zeit und Speicher (RSS). Ergebnisse in `docs/architecture.md`
  („Große Dateien“) und auf der Webseite.
- **E2 Engpässe** (vorher → nachher, 1 GB): Zeilenenden umwandeln 8,8 s + 1,4 GB Undo → 3,5 s + 10 MB (Undo-Schritt
  hält 1 Byte je Umbruch, Text in einem Durchgang neu gebaut); Alle ersetzen 2,1 s, Speicher ×2 → 0,4 s, flach
  (Neubau der Rope erst ab 50 000 Änderungen); 2 Mio. Zeilen sortieren 4,9 s → 1,3 s (ein Block statt eines Strings
  je Zeile, natürlicher Vergleich ohne Allokation, `sort_unstable`); Muster mit leeren Treffern (`a*`) bei der
  Suche ~200× langsamer → `find_iter`; Normalisieren eingefügten Texts byteweise.
- **E3 Grenzen**: 4 GB je Datei; Sortieren ≤ 512 MB (sonst Hinweis `sort_too_large`, statt Speicher auszuschöpfen);
  Ansicht ≤ 2 000 × 4 000, Scrollschritt ≤ 100 000; Geräte und Pipes werden abgelehnt (`not_a_file`), eine
  wachsende Datei bricht bei 4 GB ab. Keine Warnung beim Öffnen großer Dateien – 1 GB bleibt flüssig.
- **E4 Review** durch einen separaten Agenten (15 Befunde). Behoben:
  1. Unlesbare Bytes (BOM-Dateien, „neu lesen als“) wurden still als „�“ gespeichert → einmalige Frage vor dem
     Speichern.
  2. Speichern überschrieb Änderungen anderer Programme → `changed_on_disk` (Größe/mtime), Frage „Überschreiben“,
     `force`.
  3. Neu laden nach Änderung von außen konnte während des Ladens Getipptes verwerfen → `onlyIfClean`, Prüfung
     unter der Sperre (`document_edited`).
  4. Schließen/Beenden fragte nach veraltetem Frame → `document_info` vor der Frage (wartet auf laufende Befehle).
  5. `/dev/zero`, FIFOs → `not_a_file`; Abbruch über 4 GB.
  6. `$1` in Ersetzungen ohne Kontext (`\B(\w)`) → Gruppen im Block bzw. in den Zeilen um den Treffer.
  7. Ersetzen innerhalb eines CRLF (`\r`) wirkungslos → `Document::apply_chars` mit Zeichen-Offsets.
  8. Temporäre Datei beim Speichern zunächst mit Standardrechten → bei vorhandener Datei 0600 bis zum Kopieren
     der Rechte.
  10. Unlesbare `settings.json`/`session.json` wurden beim Beenden mit Standardwerten überschrieben → nicht mehr in
      diesem Lauf.
  11. Leere Treffer (s. E2). 13. Begrenzte Ansicht/Scroll (s. E3).
- **E5 Bekannt, nicht behoben** (Folge-Ideen): (9) fällt das Überschreiben an Ort und Stelle (Ordner nicht
  beschreibbar) mitten im Schreiben aus, ist die Datei auf der Platte beschädigt – der Text bleibt im Editor;
  (12) mehrzeilige Muster finden keine Treffer über die unsichtbaren 1-MiB-Blockgrenzen der Suche; (14) Ersetzen per
  `rename` trennt Hardlinks, verliert Eigentümer/ACLs, ein toter Symlink wird zur Datei; (15) die WebView kann jeden
  absoluten Pfad öffnen/speichern (CSP streng, kein HTML aus Daten – Risiko gering; später nur Pfade aus Dialogen,
  Verlauf und Sitzung annehmen).
- **E6 Parität** mit dem Java-Editor: `docs/parity.md` – 37 Aktionen, Belege je Test, Lücken: Faltung,
  Autovervollständigung, Ein-/Ausblenden mit Übergang, Farbschema-Import, Update-Prüfung.

## Tests

Rust: `regex_groups_are_taken_in_context`, `replace_all_can_cut_into_crlf`,
`line_ending_conversion_undoes_and_redoes_compactly`, `devices_are_not_files`,
`save_does_not_overwrite_changes_on_disk`, `saving_keeps_a_private_file_private`, `paths_from_args_*`.
Angular: Überschreiben nach Rückfrage, Warnung vor „�“, Schließen fragt die Engine.
