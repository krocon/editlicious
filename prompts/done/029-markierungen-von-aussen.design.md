# Design 029: Markierungen von außen

> Prompt: [029-markierungen-von-aussen.md](029-markierungen-von-aussen.md) (schema-commander E4). Stand 10.11.2026.
> Baut auf 026 (Farbvertrag), 027 (Positionen) und dem Änderungsprotokoll aus 030 (E2 dort).

## Bestand

Wort-Marker (007/019) sind **Suchmuster** (bis zu zehn Wörter, jede Fundstelle gefärbt) – keine Bereiche, kein
Besitzer, keine Meldung. Die Übersichtsleiste kennt Bit 1 (Treffer) und Bit 2 (Wort-Marker). Die neuen Markierungen
sind etwas anderes und heißen in der Engine **Dekorationen** (`deco`), damit die Befehle `setMarkers`/`clearMarkers`
(Wort-Marker, Sitzungsdatei) bleiben; an der Web Component heißen sie, wie der Prompt will, `setMarkers(owner, …)`.

## Entscheidungen

- **E1 In der Engine** (`ed-core`, `editor/deco.rs`): Bereiche müssen mit dem Text wandern und nur Sichtbares darf
  in den Frame – beides kann nur, wer den Text hat. `Decos`: alle Markierungen aller Besitzer in **einem** nach
  Start sortierten Vektor (Zeichen-Offsets), dazu die größte Länge (`max_len`), um für ein Fenster per binärer Suche
  die Kandidaten zu finden, die davor beginnen.
- **E2 Befehle** `setDecorations { owner, items }` (ersetzt alle des Besitzers), `clearDecorations { owner? }`,
  `decorationStep { backwards, owner? }`. Setzen/Löschen sind Ansichtsbefehle (`is_view_only`): kein Undo-Schritt,
  nicht aufgezeichnet, auch bei Schreibschutz. Abfrage `Session::decorations(owner)` (aktuelle Bereiche).
  Markierung: `from`, `to`, `severity` (`error|warning|info|hint` oder freie Klasse), `message`, `gutter`
  (Randsymbol: leer = Punkt, sonst Text), `line` (Zeilenhintergrund), `onEdit`.
- **E3 Mitwandern** über das Änderungsprotokoll (030 E2), ein Durchlauf je Bearbeitung: Text davor verschiebt;
  Einfügen genau am Anfang schiebt, genau am Ende lässt die Markierung, wie sie ist. Text **innerhalb**:
  `onEdit: 'track'` (Vorgabe) – die Markierung wächst/schrumpft mit, bleibt nichts von ihr übrig, verschwindet sie;
  `'remove'` – jede Änderung im Bereich entfernt sie (eine Diagnose, die nicht mehr stimmt). Undo/Redo und
  „Zeilenenden umwandeln“ werden mitverfolgt. Ein neuer Text (`setValue`, Neuladen) verwirft alle Markierungen.
  Protokolliert wird nur, solange es Markierungen (oder ein Snippet) gibt – **ohne Markierungen kostet es nichts**.
- **E4 Frame**: `Frame.decos` = die Markierungen im sichtbaren Zeilenbereich (mit Meldung), `FrameLine.decos` =
  `[von, bis, Index]` in Zellen je Zeile; leere Bereiche und leere Zeilen eines mehrzeiligen Bereichs zeigen eine
  Zelle, ein Bereich, der am Zeilenanfang endet, deckt diese Zeile nicht; in langen Zeilen wird aufs Fenster
  beschnitten. `Frame.decoColumn`: der Rand hat eine Spalte für Symbole, solange es welche gibt.
- **E5 Darstellung** (nur DOM/CSS, Farben als Variablen – 026): die vier Schweregrade als Wellenlinie
  (`text-decoration: underline wavy` auf Leerzeichen in Bereichsbreite – kein Bild, also kein CSP-Thema; `hint`
  gepunktet), eine freie Klasse als getönter Hintergrund; `line` tönt die Zeile; Randsymbol in eigener Spalte links;
  Strich in der Übersichtsleiste (Bits 4 Fehler, 8 Warnung, 16 Rest; in `ed-wasm` mit der bestehenden Übersicht
  verodert). Farben: `--ed-mark-error|warning|info|hint` (Vorgabe: `--color-error`, `--color-warning`,
  `--color-accent`, `--color-text-muted`); freie Klasse `x` → `--ed-mark-x`, sonst `--ed-mark-info`. Die Seite gibt
  die reine Farbe, die Tönungen mischt der Editor (`color-mix`).
- **E6 Meldung**: Tooltip (`role="tooltip"`) unter der Maus und – für die Tastatur – sobald das Caret in der
  Markierung steht (ohne Auswahl; Rechen-Hinweis und Vorschlagsliste haben Vorrang); per `aria-describedby` am
  Eingabefeld und über die Live-Region angesagt.
- **E7 Navigation und Klick**: Aktionen `edit.noteNext` / `edit.notePrev` (F8 / Shift+F8, um die Enden herum; nicht
  in der Befehlspalette der App, dort gibt es keine Markierungen). Ereignis `ed-marker-click` mit
  `{ owner, index, start, end, severity, message }` – `index` ist die Stelle in der übergebenen Liste, darüber findet
  die Seite ihre eigenen Daten; der Klick setzt weiterhin das Caret.

## Verträge

```ts
setMarkers(owner: string, markers: Marker[]): Promise<void>
clearMarkers(owner?: string): Promise<void>
getMarkers(owner?: string): Promise<MarkerInfo[]>
interface Marker { start; end; severity?; message?; gutter?: boolean | string; lineBackground?; onEdit?: 'track' | 'remove' }
// Ereignis ed-marker-click: MarkerInfo { owner, index, start, end, severity, message }
```

Rust `deco::DecoSpec`/`DecoInfo` ↔ TS `DecoSpec`/`DecoInfo`; `Frame.decos`, `Frame.deco_column`, `FrameLine.decos`;
`ed-wasm` `Editor.decorations(owner?)`, `overview()` mit den neuen Bits.

## Tests und Messung

- Rust `editor/tests.rs`: Akzeptanzfall (Fehler bleibt nach eingefügten Zeilen am Wort, auch nach Undo);
  `track`/`remove`, Ränder, Löschen; Besitzer getrennt, neuer Text verwirft; Zellen (Tab, breite Zeichen, leere
  Zeile, leerer Bereich, Zeilenanfang), Randspalte; Navigation; Übersicht, Zeilenenden, Schreibschutz.
- Messung `measure_029…` (Release, M-Serie, 1 Mio. Zeilen): ohne Markierungen Tippen 120 µs/Taste, Scrollen
  113 µs/Frame; **mit 100 000 Markierungen** Tippen 663 µs/Taste, Scrollen 120 µs/Frame (Setzen 69 ms); nach dem
  Löschen wieder 126 µs – ohne Markierungen ist nichts langsamer.
- Vitest: Ansicht (Wellenlinie/Tönung/Zeile/Symbol, Tooltip Maus und Caret, Klick, Übersicht, F8), Fassade
  (Befehle, 1-basiert, `getMarkers`, Ereignis).
- Ende-zu-Ende (10.11.2026, eingebauter Browser, `site/embed/` Editor C): Markierungen zweier Besitzer, Zeile davor
  eingefügt → Markierung liegt weiter auf `NOW`; `clearMarkers('lint')` lässt `running` stehen; 100 000 Markierungen
  in 1 Mio. Zeilen in 346 ms gesetzt, Tippen und F8 flüssig, Tooltip „marker 0“. In `scripts/embed-e2e.mjs`.

## Offen / bewusst nicht

- Keine Desktop-Oberfläche dafür (die Engine-Befehle gibt es, die App setzt keine Markierungen).
- Die Übersichtsleiste des Desktop-Befehls `overview` zeigt die neuen Bits nicht (nur `ed-wasm`).
