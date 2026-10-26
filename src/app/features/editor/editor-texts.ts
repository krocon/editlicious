import { translatable } from '../../core/i18n/i18n';

/** Texts of the editor surface (English = source, German below; design 009). */
const EN = {
  label: 'Text editor',
  trimmed: (n: number) => `Removed trailing whitespace on ${n} ${n === 1 ? 'line' : 'lines'}.`,
  clipboardEmpty: 'The clipboard holds no text.',
  clipboardFailed: 'The clipboard could not be used.',
  noMoreMatches: 'No more occurrences.',
};

export const EDITOR_TEXTS = translatable('editor', EN, {
  de: {
    label: 'Texteditor',
    trimmed: (n: number) =>
      `Leerraum am Zeilenende in ${n} ${n === 1 ? 'Zeile' : 'Zeilen'} entfernt.`,
    clipboardEmpty: 'Die Zwischenablage enthält keinen Text.',
    clipboardFailed: 'Die Zwischenablage konnte nicht benutzt werden.',
    noMoreMatches: 'Keine weiteren Vorkommen.',
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
    default:
      return '';
  }
}
