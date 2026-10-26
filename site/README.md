# Werbeseite krocon.de/products/editlicious

Eine statische Seite auf Englisch (Prompt 011): `index.html` mit CSS inline, **ohne JavaScript, ohne Webfonts,
ohne Tracker, ohne Anfragen an fremde Server**, hell/dunkel nach System. Schriftzug inline (= `public/logo.svg`,
Yellowtail-Pfade, Apache-2.0), `favicon.svg` = App-Icon, Bilder in `img/`. Akzentrot `#e00034` aus der alten Seite (2013).

Dazu die **Live-Demo** in `demo/`: die Editor-Engine `ed-core` als WebAssembly (`demo/pkg/`), eine schlanke Ansicht
in `demo.js` (nur Tasten/Maus → Befehle, Frames zeichnen – wie die App). Die Demo braucht JavaScript und läuft
komplett im Browser.

## Hochladen

```
/products/editlicious/index.html
/products/editlicious/favicon.svg
/products/editlicious/img/*.png               (5 Bilder, zusammen ~0,6 MB)
/products/editlicious/demo/index.html, demo.js
/products/editlicious/demo/pkg/ed_wasm.js, ed_wasm_bg.wasm
```

Der Webserver muss `.wasm` als `application/wasm` ausliefern (sonst lädt der Browser die Engine langsamer bzw. gar
nicht per `instantiateStreaming`; wasm-bindgen fällt auf `ArrayBuffer` zurück). Kein Build-Schritt für die Seite.

## Demo neu bauen

- Lokal: `rustup target add wasm32-unknown-unknown`, `cargo install wasm-bindgen-cli --version <aus
  src-tauri/wasm/Cargo.lock>`, dann `pnpm site:demo`.
- Oder auf GitHub: Workflow **Site demo** (läuft bei Änderungen an `src-tauri/core`/`wasm`), Artefakt `demo-pkg`
  nach `site/demo/pkg/` kopieren und committen.

## Hinweise

- `pnpm site:test` prüft Bilder (vorhanden, Größe = `width`/`height`, Alt-Text), keine Skripte und keine fremden
  Anfragen auf der Hauptseite, Anker und dass die Demo ihre Engine hat.
- **Bilder** sind Fensteraufnahmen der App (1100 × 760, Xvfb im Container, Englisch): `main` (paper, Suche),
  `big-log` (night, 1 Mio. Zeilen), `wrap` (sand, Umbruch), `palette`, `settings` (ink). Neue Aufnahmen mit
  anderem Maß: `width`/`height` anpassen (Test).
- **Messwerte** in den Zahlen-Kacheln stammen aus `docs/architecture.md`; ändern sie sich, die Seite mitziehen.
- **Open Graph**: `og:url`/`og:image` zeigen auf `https://krocon.de/products/editlicious/` – bei anderem Ort anpassen.
- **Datenschutz**: keine Cookies, kein Tracking; Impressum und Datenschutz über den Link auf `https://krocon.de`.
