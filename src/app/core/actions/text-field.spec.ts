import { TestBed } from '@angular/core/testing';
import { MenuApi } from '../menu/menu-api';
import { ClipboardApi } from '../clipboard/clipboard-api';
import { DialogService } from '../../shared/dialog/dialog.service';
import { EditorApi } from '../editor/editor-api';
import { SettingsApi } from '../settings/settings-api';
import { WorkspaceApi } from '../workspace/workspace-api';
import { MenuService } from '../../features/menus/menu.service';
import { FakeClipboardApi } from '../../../testing/fake-clipboard-api';
import { FakeEditorApi } from '../../../testing/fake-editor-api';
import { FakeSettingsApi } from '../../../testing/fake-settings-api';
import { FakeWorkspaceApi } from '../../../testing/fake-workspace-api';
import { ActionService } from './action.service';
import { OS } from './platform';
import { TextFieldEdit } from './text-field';

/** The native macOS menu: its clicks and accelerators (⌘V …) arrive as `menu-action`. */
class FakeMenuApi extends MenuApi {
  click: (id: string) => void = () => undefined;
  setAppMenu(): Promise<void> {
    return Promise.resolve();
  }
  onAction(fn: (id: string) => void): Promise<() => void> {
    this.click = fn;
    return Promise.resolve(() => undefined);
  }
}

function setup() {
  const clipboard = new FakeClipboardApi();
  const menuApi = new FakeMenuApi();
  TestBed.configureTestingModule({
    providers: [
      { provide: OS, useValue: 'mac' },
      { provide: ClipboardApi, useValue: clipboard },
      { provide: MenuApi, useValue: menuApi },
      { provide: SettingsApi, useClass: FakeSettingsApi },
      { provide: EditorApi, useValue: new FakeEditorApi() },
      { provide: WorkspaceApi, useValue: new FakeWorkspaceApi() },
    ],
  });
  const ran: string[] = [];
  const actions = TestBed.inject(ActionService);
  for (const id of ['edit.paste', 'edit.copy', 'edit.cut', 'edit.selectAll', 'file.save']) {
    actions.register(id, { run: () => ran.push(id) });
  }
  TestBed.inject(MenuService).start();
  return { clipboard, menuApi, ran, edit: TestBed.inject(TextFieldEdit) };
}

const settle = () => new Promise((r) => setTimeout(r));

function field(html = '<input type="text">'): HTMLInputElement {
  const host = document.createElement('div');
  host.innerHTML = html;
  document.body.appendChild(host);
  const el = host.querySelector('input,textarea') as HTMLInputElement;
  el.focus();
  return el;
}

describe('text fields and the native menu (bugfix 024)', () => {
  afterEach(() => document.body.replaceChildren());

  it('⌘V of the menu pastes into the focused find field, not into the document', async () => {
    const { clipboard, menuApi, ran } = setup();
    const el = field();
    el.value = 'ab';
    el.setSelectionRange(1, 1);
    const inputs: string[] = [];
    el.addEventListener('input', () => inputs.push(el.value));
    clipboard.text = 'XY\n';
    menuApi.click('edit.paste');
    await settle();
    expect(el.value).toBe('aXYb');
    expect(el.selectionStart).toBe(3);
    expect(inputs).toEqual(['aXYb']); // the field's (input) handlers see the change
    expect(ran).toEqual([]);
  });

  it('pastes in an open dialog too ("Open path…")', async () => {
    const { clipboard, menuApi, ran } = setup();
    TestBed.inject(DialogService).show('openPath');
    const el = field();
    clipboard.text = '~/Library/Application Support/Claude/claude_desktop_config.json';
    menuApi.click('edit.paste');
    await settle();
    expect(el.value).toBe(clipboard.text);
    expect(ran).toEqual([]);
  });

  it('⌘C, ⌘X and ⌘A work on the field', async () => {
    const { clipboard, menuApi, ran } = setup();
    const el = field();
    el.value = 'hello world';
    el.setSelectionRange(0, 5);
    menuApi.click('edit.copy');
    await settle();
    expect(clipboard.text).toBe('hello');
    expect(el.value).toBe('hello world');
    el.setSelectionRange(5, 11);
    menuApi.click('edit.cut');
    await settle();
    expect(clipboard.text).toBe(' world');
    expect(el.value).toBe('hello');
    menuApi.click('edit.selectAll');
    expect([el.selectionStart, el.selectionEnd]).toEqual([0, 5]);
    expect(ran).toEqual([]);
  });

  it('multi-line text keeps its lines in a textarea, one line in an input', async () => {
    const { clipboard, edit } = setup();
    clipboard.text = 'a\r\nb\n';
    const input = field();
    edit.run('edit.paste');
    await settle();
    expect(input.value).toBe('a b');
    const area = field('<textarea></textarea>');
    edit.run('edit.paste');
    await settle();
    expect(area.value).toBe('a\nb\n'); // the textarea itself turns CRLF into LF
  });

  it('without a focused text field the menu runs the editor action', () => {
    const { menuApi, ran } = setup();
    const ed = field('<textarea data-editor-input></textarea>');
    expect(document.activeElement).toBe(ed);
    menuApi.click('edit.paste');
    menuApi.click('file.save');
    expect(ran).toEqual(['edit.paste', 'file.save']);
  });

  it('other actions are no text field business', () => {
    const { edit } = setup();
    field();
    expect(edit.run('file.save')).toBe(false);
    expect(edit.run('edit.paste', null)).toBe(false);
  });
});
