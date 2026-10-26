/** Texts of the editor surface (English = source; German follows in prompt 009). */
export const EDITOR_TEXTS = {
  label: 'Text editor',
  trimmed: (n: number) => `Removed trailing whitespace on ${n} ${n === 1 ? 'line' : 'lines'}.`,
  notice(code: string): string {
    switch (code) {
      case 'clipboard_empty':
        return 'The clipboard holds no text.';
      case 'clipboard_failed':
        return 'The clipboard could not be used.';
      case 'no_more_matches':
        return 'No more occurrences.';
      default:
        return '';
    }
  },
};
