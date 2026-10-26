import { buildMenuModel, isRadio, menuChord, menuLabel, resolveMenu, toSpec } from './menu-model';

const look = {
  label: (id: string) => `L:${id}`,
  enabled: () => true,
  checked: (id: string) => (id === 'view.tabs.auto' ? true : undefined),
  chord: (id: string) => (id === 'file.save' ? 'Mod+s' : ''),
};

const ids = (items: ReturnType<typeof buildMenuModel>[number]['items']): string[] =>
  items.flatMap((i) => (i.kind === 'action' ? [i.id] : i.kind === 'submenu' ? ids(i.items) : []));

describe('menu model', () => {
  it('puts About, Settings and Quit into the app menu on macOS', () => {
    const mac = buildMenuModel('mac', { recent: [], open: [] }).flatMap((m) => ids(m.items));
    const other = buildMenuModel('other', { recent: [], open: [] }).flatMap((m) => ids(m.items));
    for (const id of ['app.about', 'app.quit', 'app.settings', 'view.toggleMenuBar']) {
      expect(mac).not.toContain(id);
      expect(other).toContain(id);
    }
  });

  it('lists recent and open files by name', () => {
    const [file, , view] = buildMenuModel('other', { recent: ['a.txt', 'b.log'], open: ['x'] });
    expect(ids(file.items)).toContain('file.recent.2');
    const sw = view.items.find((i) => i.kind === 'submenu' && i.label === 'Switch to');
    expect(sw && sw.kind === 'submenu' && sw.items[0]).toEqual({
      kind: 'action',
      id: 'file.switch.1',
      label: 'x',
    });
  });

  it('resolves labels, checks and shortcuts and strips them for Rust', () => {
    const menus = resolveMenu(buildMenuModel('other', { recent: [], open: [] }), look, 'other');
    const save = menus[0].items.find((i) => i.kind === 'action' && i.id === 'file.save');
    expect(save).toMatchObject({ label: 'L:file.save', accelerator: 'Mod+s', shortcut: 'Ctrl+S' });
    const spec = JSON.stringify(toSpec(menus));
    expect(spec).not.toContain('"shortcut":');
    expect(spec).toContain('"checked":true');
  });

  it('shows only chords the native menu can take', () => {
    expect(menuChord(['Tab', 'Mod+]'])).toBe('Mod+]');
    expect(menuChord(['Backspace'])).toBe('');
    expect(menuChord(['F1'])).toBe('F1');
  });

  it('cleans labels and knows radio items', () => {
    expect(menuLabel('a\nb')).toBe('a b');
    expect(menuLabel('')).toBe('…');
    expect(isRadio('view.tabs.show')).toBe(true);
    expect(isRadio('file.save')).toBe(false);
  });
});
