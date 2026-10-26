import { translatable } from '../../core/i18n/i18n';

/** Texts of find/replace and go-to-line (English = source, German below; design 009). */
const EN = {
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
  count: (current: number, total: number) =>
    current > 0 ? `${current} of ${total}` : `${total} ${total === 1 ? 'match' : 'matches'}`,
  replaced: (n: number) => `Replaced ${n} ${n === 1 ? 'match' : 'matches'}.`,
  gotoLabel: 'Go to line',
  gotoPlaceholder: 'line or line:column',
};

export const FIND_TEXTS = translatable('find', EN, {
  de: {
    find: 'Suchen',
    replace: 'Ersetzen',
    replaceOne: 'Ersetzen',
    replaceAll: 'Alle ersetzen',
    next: 'Nächster Treffer',
    prev: 'Vorheriger Treffer',
    close: 'Schließen',
    caseSensitive: 'Groß-/Kleinschreibung',
    wholeWord: 'Ganzes Wort',
    regex: 'Regulärer Ausdruck',
    toggleReplace: 'Ersetzen ein/aus',
    noMatch: 'Keine Treffer',
    invalid: 'Ungültiges Muster',
    count: (current: number, total: number) =>
      current > 0 ? `${current} von ${total}` : `${total} ${total === 1 ? 'Treffer' : 'Treffer'}`,
    replaced: (n: number) => `${n} ${n === 1 ? 'Treffer' : 'Treffer'} ersetzt.`,
    gotoLabel: 'Gehe zu Zeile',
    gotoPlaceholder: 'Zeile oder Zeile:Spalte',
  },
});
