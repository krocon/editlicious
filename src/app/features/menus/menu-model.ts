import { formatChord } from '../../core/actions/shortcut';
import { Os } from '../../core/actions/platform';
import { MAX_RECENT_ACTIONS, MAX_SWITCH_ACTIONS } from '../../core/actions/action-catalog';
import { MenuItemSpec, MenuSpec } from '../../core/menu/menu-api';
import { MENUS_TEXTS } from './menus-texts';

/**
 * One model for the native (macOS) and the HTML menu bar (taken over from comic-commander
 * design 008 E8, menus of the old editlicious, legacy §7). Pure.
 */
export type MenuNode =
  | { kind: 'action'; id: string; label?: string }
  | { kind: 'separator' }
  | { kind: 'submenu'; label: string; items: MenuNode[] };

export interface TopMenuNode {
  label: string;
  items: MenuNode[];
}

const a = (id: string, label?: string): MenuNode => ({ kind: 'action', id, label });
const sep: MenuNode = { kind: 'separator' };

export interface MenuInputs {
  /** Display names of recently used files. */
  recent: string[];
  /** Names of the open files (tab order). */
  open: string[];
}

export function buildMenuModel(os: Os, input: MenuInputs): TopMenuNode[] {
  const t = MENUS_TEXTS;
  const recent = input.recent
    .slice(0, MAX_RECENT_ACTIONS)
    .map((name, i) => a(`file.recent.${i + 1}`, name));
  // macOS: About, Settings, Quit live in the app menu Rust adds
  const file: MenuNode[] = [
    a('file.new'),
    a('file.open'),
    ...(recent.length ? [{ kind: 'submenu', label: t.recent, items: recent } as MenuNode] : []),
    sep,
    a('file.save'),
    a('file.saveAs'),
    a('file.saveAll'),
    sep,
    a('file.close'),
    a('file.closeAll'),
    ...(os === 'mac' ? [] : [sep, a('app.settings'), sep, a('app.quit')]),
  ];
  const edit: MenuNode[] = [
    a('edit.undo'),
    a('edit.redo'),
    sep,
    a('edit.cut'),
    a('edit.copy'),
    a('edit.paste'),
    a('edit.selectAll'),
    sep,
    a('edit.indent'),
    a('edit.outdent'),
    a('edit.deleteWordLeft'),
    a('edit.deleteWordRight'),
    a('edit.toggleOverwrite'),
    sep,
    {
      kind: 'submenu',
      label: t.lines,
      items: [
        a('edit.duplicateLines'),
        a('edit.deleteLines'),
        a('edit.moveLinesUp'),
        a('edit.moveLinesDown'),
        a('edit.joinLines'),
        sep,
        a('edit.sortAsc'),
        a('edit.sortDesc'),
        a('edit.sortNumeric'),
        a('edit.sortUnique'),
        sep,
        a('edit.trimTrailing'),
        a('edit.indentToSpaces'),
        a('edit.indentToTabs'),
      ],
    },
    {
      kind: 'submenu',
      label: t.case,
      items: [a('edit.toggleCase'), a('edit.upperCase'), a('edit.lowerCase'), a('edit.titleCase')],
    },
    sep,
    a('edit.addNextOccurrence'),
    a('edit.toggleMarker'),
    a('edit.clearMarkers'),
    sep,
    a('find.open'),
    a('find.replace'),
    a('find.next'),
    a('find.prev'),
    a('goto.line'),
  ];
  const switchTo = input.open
    .slice(0, MAX_SWITCH_ACTIONS)
    .map((name, i) => a(`file.switch.${i + 1}`, name));
  const view: MenuNode[] = [
    a('app.palette'),
    sep,
    ...(os === 'mac' ? [] : [a('view.toggleMenuBar')]),
    a('view.toggleChrome'),
    a('view.fullscreen'),
    {
      kind: 'submenu',
      label: t.tabs,
      items: [a('view.tabs.auto'), a('view.tabs.show'), a('view.tabs.hide')],
    },
    {
      kind: 'submenu',
      label: t.scheme,
      items: [
        a('view.scheme.auto'),
        a('view.scheme.paper'),
        a('view.scheme.ink'),
        a('view.scheme.sand'),
        a('view.scheme.night'),
        sep,
        a('view.nextScheme'),
      ],
    },
    a('view.language'),
    sep,
    a('view.zoomIn'),
    a('view.zoomOut'),
    a('view.zoomReset'),
    a('view.lineNumbers'),
    a('view.indentGuides'),
    sep,
    a('file.next'),
    a('file.prev'),
    ...(switchTo.length
      ? [{ kind: 'submenu', label: t.switchTo, items: switchTo } as MenuNode]
      : []),
  ];
  const help: MenuNode[] = [a('app.shortcuts'), ...(os === 'mac' ? [] : [sep, a('app.about')])];
  return [
    { label: t.file, items: file },
    { label: t.edit, items: edit },
    { label: t.view, items: view },
    { label: t.help, items: help },
  ];
}

/** What the menus ask about an action. */
export interface ActionLookup {
  label(id: string): string;
  enabled(id: string): boolean;
  checked(id: string): boolean | undefined;
  /** First chord (canonical), `''` = none. */
  chord(id: string): string;
}

/** A resolved item: the native spec plus the shortcut text for the HTML bar. */
export type ResolvedItem =
  | (Extract<MenuItemSpec, { kind: 'action' }> & { shortcut: string })
  | { kind: 'separator' }
  | { kind: 'submenu'; label: string; items: ResolvedItem[] };

export interface ResolvedMenu {
  label: string;
  items: ResolvedItem[];
}

function resolveItems(items: MenuNode[], look: ActionLookup, os: Os): ResolvedItem[] {
  return items.map((n): ResolvedItem => {
    if (n.kind === 'separator') return n;
    if (n.kind === 'submenu') return { ...n, items: resolveItems(n.items, look, os) };
    const chord = look.chord(n.id);
    const checked = look.checked(n.id);
    return {
      kind: 'action',
      id: n.id,
      label: n.label ?? look.label(n.id),
      enabled: look.enabled(n.id),
      ...(checked === undefined ? {} : { checked }),
      ...(chord ? { accelerator: chord } : {}),
      shortcut: chord ? formatChord(chord, os) : '',
    };
  });
}

export function resolveMenu(model: TopMenuNode[], look: ActionLookup, os: Os): ResolvedMenu[] {
  return model.map((m) => ({ label: m.label, items: resolveItems(m.items, look, os) }));
}

/** Longest label Rust accepts (`app_menu.rs`). */
const MAX_LABEL = 120;

/** Control characters out, length cut – a file name with a newline must not cost the menu. */
export function menuLabel(text: string): string {
  // eslint-disable-next-line no-control-regex
  const flat = text.replace(/[\u0000-\u001f\u007f-\u009f]+/g, ' ').trim();
  return [...flat].slice(0, MAX_LABEL).join('') || '…';
}

/** The spec Rust gets (the HTML-only `shortcut` stays here). */
export function toSpec(menus: ResolvedMenu[]): MenuSpec {
  const strip = (items: ResolvedItem[]): MenuItemSpec[] =>
    items.map((i) => {
      if (i.kind === 'submenu') {
        return { kind: 'submenu', label: menuLabel(i.label), items: strip(i.items) };
      }
      if (i.kind === 'separator') return i;
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { shortcut, ...rest } = i;
      return { ...rest, label: menuLabel(rest.label) };
    });
  return { menus: menus.map((m) => ({ label: menuLabel(m.label), items: strip(m.items) })) };
}

/**
 * The chord a menu shows for an action: the first one the native menu can take (with a
 * modifier or an F key, `app_menu.rs`), else the first. Editing keys without a modifier
 * (Tab, Backspace …) are not shown in the native menu – it would catch them before the WebView.
 */
export function menuChord(chords: readonly string[]): string {
  const native = chords.find((c) => /(^|\+)(Mod|Ctrl|Alt)\+/.test(c) || /^F\d{1,2}$/.test(c));
  return native ?? '';
}

/** Items that exclude each other are radio items. */
export function isRadio(id: string): boolean {
  return (
    id.startsWith('view.tabs.') || id.startsWith('file.switch.') || id.startsWith('view.scheme.')
  );
}
