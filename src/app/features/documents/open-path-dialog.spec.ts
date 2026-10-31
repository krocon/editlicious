import { TestBed } from '@angular/core/testing';
import { EditorApi } from '../../core/editor/editor-api';
import { SettingsApi } from '../../core/settings/settings-api';
import { WorkspaceApi } from '../../core/workspace/workspace-api';
import { PathCompletion } from '../../core/workspace/workspace.models';
import { DialogService } from '../../shared/dialog/dialog.service';
import { FakeEditorApi } from '../../../testing/fake-editor-api';
import { FakeSettingsApi } from '../../../testing/fake-settings-api';
import { FakeWorkspaceApi } from '../../../testing/fake-workspace-api';
import { DocumentsStore } from './documents.store';
import { OpenPathDialog } from './open-path-dialog';

const CFG = '~/Library/Application Support/Claude/claude_desktop_config.json';
const ABS = '/Users/m/Library/Application Support/Claude/claude_desktop_config.json';

const comp = (entries: [string, boolean][], common: string | null = null): PathCompletion => ({
  entries: entries.map(([value, dir]) => ({ label: value, value, dir })),
  common,
  more: false,
});

async function setup(configure?: (ws: FakeWorkspaceApi) => void) {
  const api = new FakeEditorApi();
  const ws = new FakeWorkspaceApi();
  ws.existing[CFG] = ABS;
  ws.completions['~/'] = comp([['~/Library/', true]]);
  ws.completions['~/Lib'] = comp([['~/Library/', true]], '~/Library/');
  ws.completions['~/Library/'] = comp([['~/Library/Application Support/', true]]);
  ws.completions['~/Library/Application Support/'] = comp([
    ['~/Library/Application Support/Claude/', true],
  ]);
  ws.completions['~/Library/Application Support/Claude/'] = comp([[CFG, false]]);
  configure?.(ws);
  TestBed.configureTestingModule({
    imports: [OpenPathDialog],
    providers: [
      { provide: SettingsApi, useClass: FakeSettingsApi },
      { provide: EditorApi, useValue: api },
      { provide: WorkspaceApi, useValue: ws },
    ],
  });
  const store = TestBed.inject(DocumentsStore);
  await store.start();
  const dialogs = TestBed.inject(DialogService);
  dialogs.show('openPath');
  const fixture = TestBed.createComponent(OpenPathDialog);
  document.body.appendChild(fixture.nativeElement);
  await settle(fixture);
  const el = fixture.nativeElement as HTMLElement;
  const field = el.querySelector<HTMLInputElement>('input')!;
  return { fixture, el, field, ws, api, store, dialogs };
}

async function settle(fixture: { whenStable(): Promise<unknown> }) {
  for (let i = 0; i < 3; i++) {
    await new Promise((r) => setTimeout(r));
    await fixture.whenStable();
  }
}

const key = (target: Element, k: string, shiftKey = false) =>
  target.dispatchEvent(
    new KeyboardEvent('keydown', { key: k, shiftKey, bubbles: true, cancelable: true }),
  );

function type(field: HTMLInputElement, text: string) {
  field.value = text;
  field.dispatchEvent(new Event('input'));
}

const options = (el: HTMLElement) =>
  [...el.querySelectorAll('[role="option"]')].map((o) => o.textContent?.trim());

describe('OpenPathDialog (prompt 024)', () => {
  afterEach(() => document.body.replaceChildren());

  it('starts with the suggested folder selected and lists its entries', async () => {
    const { el, field } = await setup();
    expect(field.value).toBe('~/');
    expect(field.selectionStart).toBe(0);
    expect(field.selectionEnd).toBe(2);
    expect(options(el)).toEqual(['~/Library/']);
    expect(field.getAttribute('role')).toBe('combobox');
    expect(field.getAttribute('aria-expanded')).toBe('true');
  });

  it('opens a typed path and closes', async () => {
    const { fixture, field, api, store, dialogs } = await setup();
    type(field, CFG);
    key(field, 'Enter');
    await settle(fixture);
    expect(api.opened).toEqual([ABS]);
    expect(store.active()?.info.path).toBe(ABS);
    expect(dialogs.open()).toBeNull();
  });

  it('Tab completes like a shell, a chosen folder goes into the field, a file opens', async () => {
    const { fixture, el, field, api } = await setup();
    type(field, '~/Lib');
    await settle(fixture);
    expect(key(field, 'Tab')).toBe(false); // handled, focus stays
    await settle(fixture);
    expect(field.value).toBe('~/Library/');
    // ↓ + Enter on a folder: into the field
    key(field, 'ArrowDown');
    await settle(fixture);
    expect(field.getAttribute('aria-activedescendant')).toBe('open-path-0');
    key(field, 'Enter');
    await settle(fixture);
    expect(field.value).toBe('~/Library/Application Support/');
    // ↓ + Tab takes the chosen entry
    key(field, 'ArrowDown');
    key(field, 'Tab');
    await settle(fixture);
    expect(field.value).toBe('~/Library/Application Support/Claude/');
    // a click on a file opens it
    (el.querySelector('[role="option"]') as HTMLElement).click();
    await settle(fixture);
    expect(api.opened).toEqual([ABS]);
  });

  it('shows why a path cannot be opened and stays open', async () => {
    const { fixture, el, field, api, dialogs } = await setup();
    type(field, '~/nope.txt');
    key(field, 'Enter');
    await settle(fixture);
    expect(el.querySelector('[role="alert"]')?.textContent).toBe('The file does not exist.');
    expect(field.getAttribute('aria-invalid')).toBe('true');
    expect(api.opened).toEqual([]);
    expect(dialogs.open()).toBe('openPath');
    // typing again clears the message
    type(field, '~/n');
    await settle(fixture);
    expect(el.querySelector('[role="alert"]')?.textContent).toBe('');
  });

  it('drops completions of older inputs', async () => {
    let release: (c: PathCompletion) => void = () => undefined;
    const { fixture, el, field } = await setup((ws) => {
      const original = ws.completeOpenPath.bind(ws);
      ws.completeOpenPath = (input: string) =>
        input === '~/slow' ? new Promise<PathCompletion>((r) => (release = r)) : original(input);
    });
    type(field, '~/slow');
    type(field, '~/');
    await settle(fixture);
    release(comp([['~/slow-old', false]]));
    await settle(fixture);
    expect(options(el)).toEqual(['~/Library/']);
  });

  it('Tab without anything to complete leaves the field alone', async () => {
    const { fixture, field } = await setup();
    type(field, '~/zzz');
    await settle(fixture);
    expect(key(field, 'Tab')).toBe(true); // not handled: the focus trap moves on
    expect(field.value).toBe('~/zzz');
  });
});
