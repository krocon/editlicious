import { Command, Motion } from '../editor/editor.models';

/**
 * Every function of the app as an action (taken over from comic-commander design 008): id,
 * text, context. Shortcuts live in the keymaps (`keymap.*.json`), handlers are registered by the
 * views (`ActionService`). Editor actions carry the engine command they run (design 004 E1) –
 * the Rust engine does the work, the action only names it.
 * Texts here (English = source; German follows in prompt 009).
 */
export type ActionContext = 'global' | 'editor';

export interface ActionDef {
  id: string;
  label: string;
  context: ActionContext;
  /** Held key repeats the action (cursor, deleting); else once per press. */
  repeat?: boolean;
  /** Shown in the command palette (cursor moves are not). */
  palette?: boolean;
  /** Editor actions: the engine command. */
  cmd?: Command;
}

/** Recently used files reachable from the menu. */
export const MAX_RECENT_ACTIONS = 10;
/** Open files reachable by `Mod+1…9` and the menu. */
export const MAX_SWITCH_ACTIONS = 9;

const G = (id: string, label: string, extra: Partial<ActionDef> = {}): ActionDef => ({
  id,
  label,
  context: 'global',
  ...extra,
});
const E = (id: string, label: string, cmd: Command, extra: Partial<ActionDef> = {}): ActionDef => ({
  id,
  label,
  context: 'editor',
  cmd,
  ...extra,
});

const MOTIONS: [Motion, string][] = [
  ['left', 'left'],
  ['right', 'right'],
  ['up', 'up'],
  ['down', 'down'],
  ['wordLeft', 'word left'],
  ['wordRight', 'word right'],
  ['lineStart', 'line start'],
  ['lineEnd', 'line end'],
  ['pageUp', 'page up'],
  ['pageDown', 'page down'],
  ['docStart', 'document start'],
  ['docEnd', 'document end'],
];

export const ACTIONS: readonly ActionDef[] = [
  // file (design 004)
  G('file.new', 'New file'),
  G('file.open', 'Open file…'),
  G('file.save', 'Save'),
  G('file.saveAs', 'Save as…'),
  G('file.saveAll', 'Save all'),
  G('file.close', 'Close file'),
  G('file.closeAll', 'Close all files'),
  G('file.next', 'Next file', { repeat: true }),
  G('file.prev', 'Previous file', { repeat: true }),
  ...Array.from({ length: MAX_RECENT_ACTIONS }, (_, i) =>
    G(`file.recent.${i + 1}`, `Recent file ${i + 1}`, { palette: false }),
  ),
  ...Array.from({ length: MAX_SWITCH_ACTIONS }, (_, i) =>
    G(`file.switch.${i + 1}`, `Switch to file ${i + 1}`, { palette: false }),
  ),
  // app and view (design 005)
  G('app.palette', 'Command palette…'),
  G('app.shortcuts', 'Keyboard shortcuts…'),
  G('app.settings', 'Settings…'),
  G('app.focusMenu', 'Menu bar', { palette: false }),
  G('app.about', 'About editlicious'),
  G('app.quit', 'Quit'),
  G('view.toggleMenuBar', 'Show menu bar'),
  G('view.toggleChrome', 'Hide everything but the text'),
  G('view.fullscreen', 'Full screen'),
  G('view.tabs.auto', 'Tabs: from the second file on'),
  G('view.tabs.show', 'Tabs: always'),
  G('view.tabs.hide', 'Tabs: never'),
  // colours and languages (design 008)
  G('view.nextScheme', 'Next colour scheme'),
  G('view.scheme.auto', 'Colour scheme: follow the system'),
  G('view.scheme.paper', 'Colour scheme: Paper (light)'),
  G('view.scheme.ink', 'Colour scheme: Ink (dark)'),
  G('view.scheme.sand', 'Colour scheme: Sand (light)'),
  G('view.scheme.night', 'Colour scheme: Night (dark)'),
  { id: 'view.language', label: 'Change language…', context: 'editor' },
  // settings (design 009)
  G('view.zoomIn', 'Larger font', { repeat: true }),
  G('view.zoomOut', 'Smaller font', { repeat: true }),
  G('view.zoomReset', 'Reset font size'),
  G('view.lineNumbers', 'Line numbers: absolute / relative / off'),
  G('view.indentGuides', 'Show indentation guides'),
  // status bar and extras (design 010)
  G('view.wordWrap', 'Wrap long lines'),
  G('view.statusBar', 'Show the status bar'),
  G('view.overview', 'Show matches in the scrollbar'),
  { id: 'file.encoding', label: 'Encoding…', context: 'editor' },
  { id: 'file.lineEnding', label: 'Line endings…', context: 'editor' },
  { id: 'edit.copyHtml', label: 'Copy as HTML', context: 'editor' },
  { id: 'edit.copyHtmlNumbers', label: 'Copy as HTML with line numbers', context: 'editor' },
  // recording and playback (design 015)
  G('record.toggle', 'Start/stop recording'),
  G('record.replay', 'Play the last recording again'),
  G('record.play', 'Play a recording or script…'),
  G('record.pause', 'Pause/resume playback'),
  G('record.stop', 'Stop playback'),
  G('record.save', 'Save the last recording…'),
  G('record.saveScript', 'Save the last recording as a script…'),
  // editing – the engine does it
  E('edit.undo', 'Undo', { type: 'undo' }, { repeat: true }),
  E('edit.redo', 'Redo', { type: 'redo' }, { repeat: true }),
  E('edit.cut', 'Cut', { type: 'cut' }),
  E('edit.copy', 'Copy', { type: 'copy' }),
  E('edit.paste', 'Paste', { type: 'paste' }),
  E('edit.selectAll', 'Select all', { type: 'selectAll' }),
  E('edit.indent', 'Indent', { type: 'indent' }, { repeat: true }),
  E('edit.outdent', 'Outdent', { type: 'outdent' }, { repeat: true }),
  E('edit.newline', 'New line', { type: 'newline' }, { repeat: true, palette: false }),
  E('edit.backspace', 'Delete left', { type: 'backspace' }, { repeat: true, palette: false }),
  E('edit.delete', 'Delete right', { type: 'delete' }, { repeat: true, palette: false }),
  E('edit.deleteWordLeft', 'Delete word left', { type: 'deleteWordLeft' }, { repeat: true }),
  E('edit.deleteWordRight', 'Delete word right', { type: 'deleteWordRight' }, { repeat: true }),
  E('edit.toggleOverwrite', 'Toggle insert/overwrite', { type: 'toggleOverwrite' }),
  E(
    'view.scrollUp',
    'Scroll one line up',
    { type: 'scroll', lines: -1, cells: 0 },
    {
      repeat: true,
      palette: false,
    },
  ),
  E(
    'view.scrollDown',
    'Scroll one line down',
    { type: 'scroll', lines: 1, cells: 0 },
    {
      repeat: true,
      palette: false,
    },
  ),
  // find & go to (design 006) – the editor view runs them (bars, counting)
  { id: 'find.open', label: 'Find…', context: 'editor' },
  { id: 'find.replace', label: 'Replace…', context: 'editor' },
  { id: 'find.next', label: 'Find next', context: 'editor', repeat: true },
  { id: 'find.prev', label: 'Find previous', context: 'editor', repeat: true },
  {
    id: 'edit.escape',
    label: 'Close search / back to one cursor',
    context: 'editor',
    palette: false,
  },
  { id: 'goto.line', label: 'Go to line…', context: 'editor' },
  // lines, case, cursors, markers (design 007)
  E('edit.sortAsc', 'Sort lines', { type: 'sortLines', options: { ignoreCase: true } }),
  E('edit.sortDesc', 'Sort lines descending', {
    type: 'sortLines',
    options: { descending: true, ignoreCase: true },
  }),
  E('edit.sortNumeric', 'Sort lines by number', {
    type: 'sortLines',
    options: { mode: 'numeric' },
  }),
  E('edit.sortUnique', 'Sort lines and remove duplicates', {
    type: 'sortLines',
    options: { ignoreCase: false, unique: true },
  }),
  E('edit.toggleCase', 'Upper/lower case', { type: 'changeCase', to: 'toggle' }),
  E('edit.upperCase', 'Upper case', { type: 'changeCase', to: 'upper' }),
  E('edit.lowerCase', 'Lower case', { type: 'changeCase', to: 'lower' }),
  E('edit.titleCase', 'Title case', { type: 'changeCase', to: 'title' }),
  E('edit.duplicateLines', 'Duplicate lines', { type: 'duplicateLines' }, { repeat: true }),
  E('edit.deleteLines', 'Delete lines', { type: 'deleteLines' }, { repeat: true }),
  E('edit.moveLinesUp', 'Move lines up', { type: 'moveLines', up: true }, { repeat: true }),
  E('edit.moveLinesDown', 'Move lines down', { type: 'moveLines', up: false }, { repeat: true }),
  E('edit.joinLines', 'Join lines', { type: 'joinLines' }, { repeat: true }),
  E('edit.trimTrailing', 'Remove trailing whitespace', { type: 'trimTrailingWhitespace' }),
  E('edit.indentToSpaces', 'Indentation to spaces', { type: 'convertIndentation', spaces: true }),
  E('edit.indentToTabs', 'Indentation to tabs', { type: 'convertIndentation', spaces: false }),
  E(
    'edit.addNextOccurrence',
    'Add next occurrence',
    { type: 'addNextOccurrence' },
    { repeat: true },
  ),
  E('edit.toggleMarker', 'Mark word', { type: 'toggleMarker' }),
  E('edit.clearMarkers', 'Clear all marks', { type: 'clearMarkers' }),
  E(
    'block.left',
    'Column select left',
    { type: 'blockMove', dx: -1, dy: 0 },
    { repeat: true, palette: false },
  ),
  E(
    'block.right',
    'Column select right',
    { type: 'blockMove', dx: 1, dy: 0 },
    { repeat: true, palette: false },
  ),
  E(
    'block.up',
    'Column select up',
    { type: 'blockMove', dx: 0, dy: -1 },
    { repeat: true, palette: false },
  ),
  E(
    'block.down',
    'Column select down',
    { type: 'blockMove', dx: 0, dy: 1 },
    { repeat: true, palette: false },
  ),
  ...MOTIONS.flatMap(([to, text]) => [
    E(
      `cursor.${to}`,
      `Cursor ${text}`,
      { type: 'move', to, select: false },
      {
        repeat: true,
        palette: false,
      },
    ),
    E(
      `select.${to}`,
      `Select ${text}`,
      { type: 'move', to, select: true },
      {
        repeat: true,
        palette: false,
      },
    ),
  ]),
];

const BY_ID = new Map(ACTIONS.map((a) => [a.id, a]));

export function actionDef(id: string): ActionDef | undefined {
  return BY_ID.get(id);
}
