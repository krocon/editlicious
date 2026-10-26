/** Texts of the editor surface (English = source; German follows in prompt 009). */
export const EDITOR_TEXTS = {
  label: 'Text editor',
  notice(code: string): string {
    switch (code) {
      case 'clipboard_empty':
        return 'The clipboard holds no text.';
      case 'clipboard_failed':
        return 'The clipboard could not be used.';
      default:
        return '';
    }
  },
};
