# Design 017: Video-Export aus Aufzeichnungen

> Prompt: [017-video-export.md](017-video-export.md). Stand 26.10.2026.

## Entscheidungen

- **E1 Renderer B – kopflos in Rust** (Crate `ed-video`): die Zeitachse aus 015 läuft auf einer **virtuellen Uhr**
  durch eine echte `ed-core`-Sitzung; jedes Bild wird aus dem Frame der Engine gezeichnet. Variante A (WebView
  offscreen abfilmen) verworfen: Tauri hat dafür keine Schnittstelle, unter Linux/CI ohne Fenster nicht machbar, und
  das Ergebnis hinge von Fenster-Skalierung und Timing ab. Prototyp-Messung (Container, 2 Kerne): 14,3 s Video
  1080×1920 bei 30 fps in **10 s** (Fenster einmal gezeichnet und kopiert, unveränderte Bilder wiederverwendet).
- **E2 Rasterizer:** `fontdue` (reines Rust, deterministisch) für Glyphen, eigene Rechtecke/abgerundete Rechtecke
  mit Teilpixel-Kanten. Schrift: **JetBrains Mono** (OFL, im Programm); Ersatzschriften nur ausdrücklich
  (`--fallback-font`, z. B. Noto CJK). Fehlende Schriftdatei → Fehler; fehlende Glyphe → Kästchen (nie Zufall).
- **E3 Gleiches Aussehen wie die App:** die Farben werden zur Bauzeit aus `src/styles.css` gelesen (`--ed-*`,
  `--syn-*`, Seite/Rahmen; `paper`/`ink` und jedes `data-scheme`, Grundlage hell/dunkel über `--scheme-base`).
  Layout wie die Editor-Ansicht: Gutter (Ziffern + 2 Zellen), eine Zelle Abstand, Zeilenhöhe 1,55 × Schrift,
  aktuelle Zeile, Einrückungslinien, Auswahl, Such-/Wortmarken, Carets. Dazu Fensterrahmen `none`/`plain`/`mac`.
- **E4 Ausgabe:** Ordner → PNG-Folge (`frame-000001.png`); Datei → rohe RGBA-Bilder per stdin in ein **externes
  `ffmpeg`** (`.mp4` H.264 `-preset veryfast -crf 18`, `--best` = `slow -crf 14`; `.mov` ProRes; `.webm` VP9).
  ffmpeg wird nicht eingebettet (Lizenz, Größe). Bilder werden gestreamt, nie gesammelt.
- **E5 Zeit und Bewegung:** Standbild am Anfang/Ende (1 s/2 s), Caret blinkt 530 ms (nach jeder Änderung an),
  weiches Scrollen: das Bild folgt dem Scrollstand der Engine mit Dämpfung (große Sprünge sofort), dafür wird die
  Sitzung je Bild per `scrollTo` gesetzt – es sind immer echte Frames. `keys "Ctrl+D"` im Skript blendet für 1,6 s
  eine Beschriftung ein (neuer Skriptbefehl, im Editor ohne Wirkung).
- **E6 Kommandozeile:** `editlicious render <skript|aufnahme> -o … [Optionen]` (Unterbefehl der App, ohne Fenster)
  und dasselbe als `editlicious-render` (Crate-Binary) für Pipelines; `--hash` für Tests.
- **E7 In der App:** Aktion „Als Video exportieren …“: Quelle (letzte Aufnahme oder Datei), Bildgröße
  (1920×1080, 1080×1920, 1280×720, 4K), Zieldatei; Schema = das sichtbare. Läuft auf einem Thread, Fortschritt
  „Video 37 %“ in der Statusleiste (Klick bricht ab), `ffmpeg`-Pfad in den Einstellungen. Die letzte Aufnahme
  merkt sich ihren Starttext (Rope-Kopie, O(1)); eine Aufnahmedatei startet vom aktuellen Text, wenn der Hash passt.

## Tests

- `ed-video`: Farben/Parser, Schemata aus `styles.css`, Rechtecke/Clip, Schrift, Determinismus (Hash gleich,
  anderes Schema anders, Standbilder wiederverwendet), virtuelle Uhr (Zustand zu t), Abbruch, Größen, PNG-Folge,
  ffmpeg fehlt, ffmpeg-Argumente, Golden-Images hell/dunkel mit CJK/Emoji/Auswahl/Beschriftung (Toleranz),
  CLI-Optionen, Skript mit `type @datei` per Hash.
- App: Export der letzten Aufnahme ab ihrem Starttext als PNG-Folge, Abbruch.
- Angular: Ablauf des Exports (Quelle, Größe, Datei, Fortschritt, Abbruch, Fehlertext), Fokus bei aufeinander
  folgenden Auswahlen (Fehler gefunden und behoben: die zweite Auswahl bekam keinen Fokus).
- Ende-zu-Ende: CLI → MP4 1080×1920 (ffprobe), in der App aufgenommen → exportiert → MP4 geprüft.
