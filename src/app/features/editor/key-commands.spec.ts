import { detectOs, KeyInput, keyToCommand } from './key-commands';

const k = (key: string, mods: Partial<KeyInput> = {}): KeyInput => ({
  key,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
});

describe('keyToCommand', () => {
  it('moves word-wise with Ctrl elsewhere and Alt on macOS', () => {
    expect(keyToCommand(k('ArrowRight', { ctrlKey: true }), 'other')).toEqual({
      type: 'move',
      to: 'wordRight',
      select: false,
    });
    expect(keyToCommand(k('ArrowLeft', { altKey: true, shiftKey: true }), 'mac')).toEqual({
      type: 'move',
      to: 'wordLeft',
      select: true,
    });
  });

  it('uses Cmd+arrows for line and document ends on macOS', () => {
    expect(keyToCommand(k('ArrowLeft', { metaKey: true }), 'mac')).toMatchObject({
      to: 'lineStart',
    });
    expect(keyToCommand(k('ArrowDown', { metaKey: true }), 'mac')).toMatchObject({ to: 'docEnd' });
    expect(keyToCommand(k('End', { ctrlKey: true }), 'other')).toMatchObject({ to: 'docEnd' });
  });

  it('maps editing keys and the clipboard', () => {
    expect(keyToCommand(k('Enter'), 'other')).toEqual({ type: 'newline' });
    expect(keyToCommand(k('Tab', { shiftKey: true }), 'other')).toEqual({ type: 'outdent' });
    expect(keyToCommand(k('Backspace', { ctrlKey: true }), 'other')).toEqual({
      type: 'deleteWordLeft',
    });
    expect(keyToCommand(k('v', { metaKey: true }), 'mac')).toEqual({ type: 'paste' });
    expect(keyToCommand(k('Z', { ctrlKey: true, shiftKey: true }), 'other')).toEqual({
      type: 'redo',
    });
    expect(keyToCommand(k('y', { ctrlKey: true }), 'other')).toEqual({ type: 'redo' });
  });

  it('leaves typing and foreign shortcuts alone', () => {
    expect(keyToCommand(k('a'), 'other')).toBeNull();
    expect(keyToCommand(k('v', { ctrlKey: true }), 'mac')).toBeNull();
    expect(keyToCommand(k('s', { ctrlKey: true }), 'other')).toBeNull();
  });

  it('detects macOS', () => {
    expect(detectOs({ userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)' })).toBe('mac');
    expect(detectOs({ userAgent: 'Mozilla/5.0 (X11; Linux x86_64)' })).toBe('other');
  });
});
