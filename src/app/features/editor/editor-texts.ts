import { translatable } from '../../core/i18n/i18n';

/** Texts of the editor surface (English = source, German below; design 009). */
const EN = {
  label: 'Text editor',
  trimmed: (n: number) => `Removed trailing whitespace on ${n} ${n === 1 ? 'line' : 'lines'}.`,
  clipboardEmpty: 'The clipboard holds no text.',
  clipboardFailed: 'The clipboard could not be used.',
  noMoreMatches: 'No more occurrences.',
  htmlTooLarge: 'Too much for “copy as HTML” – select at most 20,000 lines.',
  copiedHtml: 'Copied as HTML.',
  sortTooLarge: 'Too much to sort at once – select at most 512 MB of lines.',
  encoding: 'Encoding',
  encodingPlaceholder: 'Type to filter encodings',
  lineEndings: 'Line endings',
  lineEndingsPlaceholder: 'Convert every line break to …',
  lf: 'LF – Unix, macOS',
  crlf: 'CRLF – Windows',
  cr: 'CR – classic Mac OS',
  // status bar (design 010 E1)
  status: 'Status',
  position: (line: number, col: number) => `Ln ${line}, Col ${col}`,
  selected: (chars: number, lines: number) =>
    lines > 1 ? `${chars} selected (${lines} lines)` : `${chars} selected`,
  carets: (n: number) => `${n} carets`,
  lines: (n: string) => `${n} lines`,
  overwrite: 'OVR',
  wrap: 'Wrap',
  longUnwrapped: 'long lines not wrapped',
  plainText: 'Plain text',
  bom: 'with BOM',
  mixed: 'mixed',
  changeEncoding: 'Change the encoding',
  changeLineEndings: 'Convert the line endings',
  changeLanguage: 'Change the language',
  noColor: 'No colour value at the cursor.',
  caretColor: (v: string) => `Colour ${v}`,
};

export const EDITOR_TEXTS = translatable('editor', EN, {
  de: {
    label: 'Texteditor',
    trimmed: (n: number) =>
      `Leerraum am Zeilenende in ${n} ${n === 1 ? 'Zeile' : 'Zeilen'} entfernt.`,
    clipboardEmpty: 'Die Zwischenablage enthält keinen Text.',
    clipboardFailed: 'Die Zwischenablage konnte nicht benutzt werden.',
    noMoreMatches: 'Keine weiteren Vorkommen.',
    htmlTooLarge: 'Zu viel für „Als HTML kopieren“ – höchstens 20.000 Zeilen auswählen.',
    copiedHtml: 'Als HTML kopiert.',
    sortTooLarge: 'Zu viel zum Sortieren – höchstens 512 MB an Zeilen auswählen.',
    encoding: 'Zeichenkodierung',
    encodingPlaceholder: 'Tippen, um Kodierungen zu filtern',
    lineEndings: 'Zeilenenden',
    lineEndingsPlaceholder: 'Alle Zeilenumbrüche umwandeln in …',
    lf: 'LF – Unix, macOS',
    crlf: 'CRLF – Windows',
    cr: 'CR – klassisches Mac OS',
    status: 'Status',
    position: (line: number, col: number) => `Z ${line}, Sp ${col}`,
    selected: (chars: number, lines: number) =>
      lines > 1 ? `${chars} ausgewählt (${lines} Zeilen)` : `${chars} ausgewählt`,
    carets: (n: number) => `${n} Cursor`,
    lines: (n: string) => `${n} Zeilen`,
    overwrite: 'ÜB',
    wrap: 'Umbruch',
    longUnwrapped: 'lange Zeilen nicht umbrochen',
    plainText: 'Nur Text',
    bom: 'mit BOM',
    mixed: 'gemischt',
    changeEncoding: 'Zeichenkodierung ändern',
    changeLineEndings: 'Zeilenenden umwandeln',
    noColor: 'Am Cursor steht kein Farbwert.',
    caretColor: (v: string) => `Farbe ${v}`,
    changeLanguage: 'Sprache ändern',
  },
});

/** A notice code of the engine as text (`''` = nothing to say). */
export function noticeText(code: string): string {
  switch (code) {
    case 'clipboard_empty':
      return EDITOR_TEXTS.clipboardEmpty;
    case 'clipboard_failed':
      return EDITOR_TEXTS.clipboardFailed;
    case 'no_more_matches':
      return EDITOR_TEXTS.noMoreMatches;
    case 'html_too_large':
      return EDITOR_TEXTS.htmlTooLarge;
    case 'sort_too_large':
      return EDITOR_TEXTS.sortTooLarge;
    case 'no_color':
      return EDITOR_TEXTS.noColor;
    default:
      return '';
  }
}
