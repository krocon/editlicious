# Prompt 017: Video-Export aus Aufzeichnungen

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 015 (016 optional).

## Ziel

Eine Aufzeichnung oder ein Tipp-Skript aus 015 wird **ohne Bildschirmaufnahme** direkt als Video gerendert:
gestochen scharf, in beliebiger Auflösung (z. B. 1920×1080, 2160×3840 hochkant für Shorts), mit exakt
reproduzierbarem Timing. Die Engine liefert deterministisch Frames; gezeichnet wird mit denselben Stilen wie in der
App (Farbschema, Schrift, Zeilennummern).

## Umfang

- **Renderer** (im Design entscheiden, Empfehlung zuerst prüfen):
  - A) In der App: die Editor-Ansicht spielt die Aufzeichnung in einem versteckten/offscreen Fenster fester Größe
    ab und liefert Einzelbilder (Canvas/WebView-Capture) an Rust → Encoder. Vorteil: pixelgleich mit der App.
  - B) Kopflos in Rust: Frames aus `ed-core` + eigener Text-Rasterizer (z. B. `cosmic-text`/`tiny-skia`), Stile
    aus den Syntax-Stilklassen. Vorteil: CLI ohne Fenster, CI-fähig. Nachteil: zweite Zeichenlogik.
  Entscheidung mit Prototyp und Messung begründen (Bildqualität, Geschwindigkeit, Aufwand).
- **Zeitachse**: virtuelle Uhr statt Echtzeit (z. B. 60 fps); jedes Bild zeigt den Zustand zum Zeitpunkt t,
  inklusive Caret-Blinken (abschaltbar) und weichem Scrollen. Rendern schneller oder langsamer als Echtzeit ist egal.
- **Ausgabe**: PNG-Sequenz immer möglich; Video (H.264 MP4, optional ProRes/WebM) über ein externes `ffmpeg`, wenn
  vorhanden (Pfad in Einstellungen) – kein ffmpeg einbetten (Lizenz, Größe); Begründung in `docs/architecture.md`.
  H.264 standardmäßig mit `-preset veryfast`, Option „beste Qualität“.
- **Optionen** (Dialog + Kommandozeile): Auflösung/Seitenverhältnis, Schriftgröße/Zoom, Farbschema hell/dunkel,
  Ränder/Innenabstand, Zeilennummern, Fensterrahmen-Optik (keine, schlicht, macOS-artig), Hintergrund, Start-/End-
  Standbild in Sekunden, optional Tastenkürzel-Einblendung bei `action`-Schritten (für Lernvideos).
- **Kommandozeile**: `editlicious render <skript|aufnahme> -o out.mp4 [--size 1080x1920] [--theme dark] …`, damit
  Marcs Video-Pipeline DE/EN-Varianten automatisch erzeugen kann.
- **Aktion** „Aufzeichnung als Video exportieren …“ mit Fortschritt (Channel) und Abbruch.

## Regeln und Randfälle

- Deterministisch: gleiches Skript + Seed + Optionen → bit-gleiche PNG-Sequenz (Test).
- Lange Aufnahmen: Bilder werden gestreamt an den Encoder gegeben, nie alle im Speicher.
- Schriften: nur mitgelieferte oder vom Nutzer gewählte; fehlende Schrift → klarer Fehler statt Fallback-Zufall.
- Keine Dateiinhalte loggen; temporäre Bilder atomar in ein Temp-Verzeichnis, danach aufräumen.

## Tests

- Rust: virtuelle Uhr (Zustand zu t), Determinismus (Hash der Bildfolge), Abbruch, ffmpeg fehlt → klarer Fehler.
- Golden-Images für wenige Schlüsselbilder (hell/dunkel, CJK/Emoji, Blockauswahl) mit Toleranz.
- Ende-zu-Ende im Container: 10-s-Skript → MP4 mit ffmpeg, Länge und Auflösung per `ffprobe` prüfen.

## Akzeptanzkriterien

- Ein 60-Sekunden-Skript wird per Kommandozeile in 1080×1920 als MP4 gerendert; Ergebnis sieht aus wie die App
  und ist in beiden Sprachvarianten zeitlich identisch choreografiert.
