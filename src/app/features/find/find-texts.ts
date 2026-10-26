/** Texts of find/replace and go-to-line (English = source; German follows in prompt 009). */
export const FIND_TEXTS = {
  find: 'Find',
  replace: 'Replace',
  replaceOne: 'Replace',
  replaceAll: 'Replace all',
  next: 'Next match',
  prev: 'Previous match',
  close: 'Close',
  caseSensitive: 'Match case',
  wholeWord: 'Whole word',
  regex: 'Regular expression',
  toggleReplace: 'Toggle replace',
  noMatch: 'No matches',
  invalid: 'Invalid pattern',
  counting: '…',
  count: (current: number, total: number) =>
    current > 0 ? `${current} of ${total}` : `${total} ${total === 1 ? 'match' : 'matches'}`,
  replaced: (n: number) => `Replaced ${n} ${n === 1 ? 'match' : 'matches'}.`,
  gotoLabel: 'Go to line',
  gotoPlaceholder: 'line or line:column',
};
