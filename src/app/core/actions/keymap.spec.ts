import { ACTIONS } from './action-catalog';
import { buildKeymap } from './keymap';

describe('keymap', () => {
  it('has only known actions and valid chords', () => {
    for (const os of ['mac', 'other'] as const) {
      expect(buildKeymap(os).notes).toEqual([]);
    }
  });

  it('moves word-wise with Alt on macOS and Ctrl elsewhere', () => {
    expect(buildKeymap('mac').byContext.editor.get('Alt+ArrowRight')).toBe('cursor.wordRight');
    expect(buildKeymap('mac').byContext.editor.get('Mod+ArrowRight')).toBe('cursor.lineEnd');
    expect(buildKeymap('other').byContext.editor.get('Mod+ArrowRight')).toBe('cursor.wordRight');
    expect(buildKeymap('other').byContext.editor.get('Mod+y')).toBe('edit.redo');
  });

  it('switches files with Ctrl+Tab on every OS and F10 like the old editor', () => {
    expect(buildKeymap('mac').byContext.global.get('Ctrl+Tab')).toBe('file.next');
    expect(buildKeymap('other').byContext.global.get('Mod+Tab')).toBe('file.next');
    expect(buildKeymap('other').byContext.global.get('Shift+F10')).toBe('file.prev');
  });

  it('every editing and cursor action names an engine command', () => {
    for (const a of ACTIONS.filter((x) => /^(edit|cursor|select)\./.test(x.id))) {
      expect(a.cmd, a.id).toBeDefined();
    }
  });

  it('applies user overrides in the action’s own context', () => {
    const k = buildKeymap('other', { 'file.save': ['F2'], 'nope.x': ['F3'] });
    expect(k.byContext.global.get('F2')).toBe('file.save');
    expect(k.byContext.global.get('Mod+s')).toBeUndefined();
    expect(k.notes).toEqual(['keymap.json: unknown action nope.x']);
  });
});
