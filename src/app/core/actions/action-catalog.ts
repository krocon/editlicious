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
  G('app.about', 'About editlicious'),
  G('app.quit', 'Quit'),
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
