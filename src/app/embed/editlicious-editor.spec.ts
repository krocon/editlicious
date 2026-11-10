import { Component, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { OS } from '../core/actions/platform';
import { textFrame } from '../../testing/fake-editor-api';
import { FakeEmbedEngineApi } from '../../testing/fake-embed-engine-api';
import {
  EdCaret,
  EdChange,
  EdCompletionProvider,
  EditliciousEditor,
  EdSnippet,
} from './editlicious-editor';
import { Command } from '../core/editor/editor.models';
import { EmbedEngineApi } from './wasm-editor-api';

const flush = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r));
};

@Component({
  imports: [EditliciousEditor],
  template: `
    <ed-editor
      id="a"
      [value]="valueA()"
      language="Rust"
      [scheme]="schemeA()"
      [colorScheme]="colorSchemeA()"
      [completionProvider]="providerA()"
      [snippets]="snippetsA()"
      [wordSuggest]="suggestA()"
      [readOnly]="readOnlyA()"
      [wordWrap]="true"
      lineNumbers="relative"
      [statusBar]="true"
      (textChange)="changes.push($event)"
      (caretChange)="carets.push($event)"
      (ready)="ready.push('a')"
    />
    <ed-editor id="b" value="second" scheme="night" (ready)="ready.push('b')" />
  `,
})
class Host {
  readonly valueA = signal('fn main() {}\n');
  readonly schemeA = signal<'paper' | 'night' | 'auto' | 'none'>('paper');
  readonly colorSchemeA = signal<'light' | 'dark' | null>(null);
  readonly providerA = signal<EdCompletionProvider | null>(null);
  readonly snippetsA = signal<EdSnippet[] | null>(null);
  readonly suggestA = signal(false);
  readonly readOnlyA = signal(false);
  changes: EdChange[] = [];
  carets: EdCaret[] = [];
  ready: string[] = [];
}

async function setup() {
  const attrs = () =>
    [document.documentElement, document.body]
      .map((e) => Array.from(e.attributes, (x) => `${x.name}=${x.value}`).join(' '))
      .join(' | ');
  const before = attrs();
  const api = new FakeEmbedEngineApi();
  TestBed.configureTestingModule({
    imports: [Host],
    providers: [
      { provide: EmbedEngineApi, useValue: api },
      { provide: OS, useValue: 'other' },
    ],
  });
  const rect = { top: 0, left: 0, right: 600, bottom: 300, width: 600, height: 300 } as DOMRect;
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue(rect);
  const fixture = TestBed.createComponent(Host);
  document.body.appendChild(fixture.nativeElement);
  await fixture.whenStable();
  await flush();
  await fixture.whenStable();
  const el = fixture.nativeElement as HTMLElement;
  const [a, b] = Array.from(el.querySelectorAll('ed-editor')) as HTMLElement[];
  const comp = (e: HTMLElement) =>
    fixture.debugElement.children.find((d) => d.nativeElement === e)!
      .componentInstance as EditliciousEditor;
  return {
    before,
    attrs,
    fixture,
    api,
    a,
    b,
    host: fixture.componentInstance,
    ea: comp(a),
    eb: comp(b),
  };
}

describe('EditliciousEditor (design 025)', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    document.body.innerHTML = '';
  });

  it('starts one engine session per instance with its text and language', async () => {
    const { api, host } = await setup();
    expect([...api.texts.values()]).toEqual(['fn main() {}\n', 'second']);
    expect(api.languagesSet[0].spec).toBe('Rust');
    expect(host.ready.sort()).toEqual(['a', 'b']);
  });

  it('renders in a shadow root and colours each host, never <html>', async () => {
    const { a, b, fixture, host, before, attrs } = await setup();
    expect(a.shadowRoot?.querySelector('app-editor-view')).toBeTruthy();
    expect(a.shadowRoot?.querySelector('app-status-bar')).toBeTruthy();
    expect(b.shadowRoot?.querySelector('app-status-bar')).toBeNull();
    expect(a.getAttribute('data-theme')).toBe('light');
    expect(b.getAttribute('data-theme')).toBe('dark');
    expect(b.getAttribute('data-scheme')).toBe('night');
    host.schemeA.set('night');
    await fixture.whenStable();
    expect(a.getAttribute('data-scheme')).toBe('night');
    // the page's root element is as it was (no data-theme, data-scheme, lang, font variables)
    expect(attrs()).toBe(before);
  });

  it('scheme "none" leaves the colours to the page (design 026 E3)', async () => {
    const { a, fixture, host } = await setup();
    host.schemeA.set('none');
    await fixture.whenStable();
    expect(a.getAttribute('data-scheme')).toBe('none');
    expect(a.hasAttribute('data-theme')).toBe(false);
    host.colorSchemeA.set('dark');
    await fixture.whenStable();
    expect(a.getAttribute('data-theme')).toBe('dark');
    // a built-in scheme ignores `color-scheme`
    host.schemeA.set('paper');
    await fixture.whenStable();
    expect(a.getAttribute('data-theme')).toBe('light');
    expect(a.hasAttribute('data-scheme')).toBe(false);
    // the font variable is the page's unless `font-family` is set
    expect(a.style.getPropertyValue('--font-editor')).toBe('');
  });

  it('keeps instances apart: find, typing and ARIA ids', async () => {
    const { a, b, api, ea, eb } = await setup();
    const [ida, idb] = [...api.texts.keys()];
    const ta = a.shadowRoot!.querySelector('textarea')!;
    ta.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'f', ctrlKey: true, bubbles: true, composed: true }),
    );
    await flush();
    expect(a.shadowRoot!.querySelector('app-find-bar')).toBeTruthy();
    expect(b.shadowRoot!.querySelector('app-find-bar')).toBeNull();
    api.commands = [];
    ta.value = 'x';
    ta.dispatchEvent(new Event('input'));
    await flush();
    expect(api.commands.filter((c) => c.cmd.type === 'type').map((c) => c.id)).toEqual([ida]);
    await eb.runCommand({ type: 'selectAll' });
    expect(api.commands.at(-1)).toEqual({ id: idb, cmd: { type: 'selectAll' } });
    void ea;
    const ids = (e: HTMLElement) =>
      Array.from(e.shadowRoot!.querySelectorAll('textarea')).map((t) =>
        t.getAttribute('aria-describedby'),
      );
    expect(ids(a)).not.toEqual(['math-hint']);
  });

  it('reports changes without the text and fetches it only on request', async () => {
    const { api, ea, host, fixture } = await setup();
    const id = [...api.texts.keys()][0];
    api.nextFrame = textFrame(['fn main() {}', 'x'], {
      caretLine: 1,
      caretCh: 1,
      anchorLine: 1,
      anchorCh: 1,
    });
    api.infos.set(id, {
      ...api.nextFrame.info,
      id,
      revision: 3,
      dirty: true,
      lineCount: 2,
      charCount: 14,
    });
    await ea.runCommand({ type: 'type', text: 'x' });
    await fixture.whenStable();
    expect(host.changes.at(-1)).toEqual({ revision: 3, dirty: true, lineCount: 2, charCount: 14 });
    expect(host.carets.at(-1)).toMatchObject({ line: 2, ch: 2, carets: 1 });
    // nothing selected: the selection's range is the caret (design 027 E4)
    expect(host.carets.at(-1)).toMatchObject({
      selStart: { line: 2, ch: 2 },
      selEnd: { line: 2, ch: 2 },
    });
    api.nextFrame = textFrame(['fn main() {}', 'x'], {
      seq: 2,
      caretLine: 0,
      caretCh: 3,
      anchorLine: 1,
      anchorCh: 1,
    });
    await ea.runCommand({ type: 'frame' });
    expect(host.carets.at(-1)).toMatchObject({
      line: 1,
      ch: 4,
      selStart: { line: 1, ch: 4 },
      selEnd: { line: 2, ch: 2 },
    });
    expect(api.textReads).toBe(0);
    expect(await ea.getValue()).toBe('fn main() {}\n');
    expect(api.textReads).toBe(1);
  });

  it('hands out selections and ranges and replaces a range (design 027)', async () => {
    const { api, ea } = await setup();
    const id = [...api.texts.keys()][0];
    api.sels = {
      primary: 1,
      ranges: [
        { anchor: { line: 0, ch: 2 }, head: { line: 0, ch: 2 }, text: '' },
        // selected backwards: start and end come in document order, 1-based
        { anchor: { line: 3, ch: 0 }, head: { line: 1, ch: 4 }, text: 'x👍\r\ny' },
      ],
    };
    expect(await ea.getSelection()).toEqual({
      text: 'x👍\r\ny',
      start: { line: 2, ch: 5 },
      end: { line: 4, ch: 1 },
    });
    expect((await ea.getSelections()).map((s) => s.start)).toEqual([
      { line: 1, ch: 3 },
      { line: 2, ch: 5 },
    ]);
    expect(await ea.getRange({ line: 2, ch: 5 }, { line: 4, ch: 1 })).toBe('1:4-3:0');
    api.commands = [];
    await ea.replaceRange({ line: 2, ch: 5 }, { line: 4, ch: 1 }, 'new');
    await ea.replaceSelection('NOW()');
    expect(api.commands.slice(-2)).toEqual([
      {
        id,
        cmd: {
          type: 'replaceRange',
          from: { line: 1, ch: 4 },
          to: { line: 3, ch: 0 },
          text: 'new',
        },
      },
      { id, cmd: { type: 'insert', text: 'NOW()' } },
    ]);
    // the whole text never crossed for any of this
    expect(api.textReads).toBe(0);
  });

  it('shows the suggestions of the page in its list and takes one (design 028)', async () => {
    const { a, b, api, host, fixture } = await setup();
    const id = [...api.texts.keys()][0];
    const words = () =>
      (
        api.commands.filter((c) => c.id === id && c.cmd.type === 'configure').at(-1)!
          .cmd as Extract<Command, { type: 'configure' }>
      ).words;
    host.suggestA.set(true);
    await fixture.whenStable();
    await flush();
    expect(words()?.auto).toBe(true);
    host.providerA.set(() => [
      { label: 'orders', kind: 'table', detail: 'public', documentation: 'All orders.' },
      { label: 'order_items', kind: 'table' },
    ]);
    await fixture.whenStable();
    await flush();
    // the provider asks while typing now – the engine's own word list stays shut
    expect(words()?.auto).toBe(false);
    const ta = a.shadowRoot!.querySelector('textarea')!;
    const key = (k: string, init: KeyboardEventInit = {}) =>
      ta.dispatchEvent(
        new KeyboardEvent('keydown', { key: k, bubbles: true, composed: true, ...init }),
      );
    api.caretAfter('ord');
    key(' ', { ctrlKey: true, code: 'Space' });
    await flush();
    await fixture.whenStable();
    const options = () => [...a.shadowRoot!.querySelectorAll('[role="option"]')];
    expect(options().map((o) => o.querySelector('.label')!.textContent)).toEqual([
      'order_items',
      'orders',
    ]);
    expect(api.commands.some((c) => c.cmd.type === 'complete')).toBe(false);
    expect(options()[1].getAttribute('part')).toBe('completion-item item-table');
    expect(options()[1].querySelector('.badge')!.getAttribute('part')).toBe(
      'completion-icon icon-table',
    );
    expect(options()[1].querySelector('.kind')!.textContent).toBe('public');
    expect(options()[0].getAttribute('aria-setsize')).toBe('2');
    expect(ta.getAttribute('aria-activedescendant')).toBe(options()[0].id);
    expect(b.shadowRoot!.querySelector('[role="option"]')).toBeNull();
    key('ArrowDown');
    await fixture.whenStable();
    expect(ta.getAttribute('aria-activedescendant')).toBe(options()[1].id);
    expect(a.shadowRoot!.querySelector('.completion .doc')!.textContent).toBe('All orders.');
    expect(ta.getAttribute('aria-describedby')).toBe(
      a.shadowRoot!.querySelector('.completion .doc')!.id,
    );
    api.commands = [];
    key('Enter');
    await flush();
    await fixture.whenStable();
    expect(api.commands.find((c) => c.cmd.type === 'replaceRange')?.cmd).toMatchObject({
      text: 'orders',
      caret: true,
    });
    expect(api.commands.some((c) => c.cmd.type === 'newline')).toBe(false);
    expect(options()).toEqual([]);
  });

  it('inserts snippets: by method, and Tab behind a prefix (design 030)', async () => {
    const { a, api, ea, host, fixture } = await setup();
    await ea.insertSnippet('SELECT ${1:*} FROM $TABLE', { vars: { TABLE: 'orders' } });
    const cmd = api.commands.at(-1)!.cmd as Extract<Command, { type: 'insertSnippet' }>;
    expect(cmd).toMatchObject({ type: 'insertSnippet', template: 'SELECT ${1:*} FROM $TABLE' });
    expect(cmd.vars).toMatchObject({ TABLE: 'orders' });
    expect(cmd.vars!['CURRENT_YEAR']).toMatch(/^\d{4}$/);
    expect(cmd.from).toBeUndefined();
    await ea.insertSnippet('x', { start: { line: 2, ch: 1 }, end: { line: 2, ch: 4 } });
    expect(api.commands.at(-1)!.cmd).toMatchObject({
      from: { line: 1, ch: 0 },
      to: { line: 1, ch: 3 },
    });
    host.snippetsA.set([{ prefix: 'sel', body: 'SELECT ${1:*};$0' }]);
    await fixture.whenStable();
    const ta = a.shadowRoot!.querySelector('textarea')!;
    const tab = () =>
      ta.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, composed: true }));
    api.caretAfter('sel');
    api.commands = [];
    tab();
    await flush();
    expect(api.commands.map((c) => c.cmd.type)).toContain('insertSnippet');
    expect(api.commands.map((c) => c.cmd.type)).not.toContain('indent');
    // no prefix before the caret: a tab
    api.caretAfter('select ');
    api.commands = [];
    tab();
    await flush();
    expect(api.commands.map((c) => c.cmd.type)).toContain('indent');
  });

  it('follows value, readOnly and settles methods', async () => {
    const { api, ea, host, fixture } = await setup();
    const id = [...api.texts.keys()][0];
    host.valueA.set('new text');
    await fixture.whenStable();
    await flush();
    expect(api.texts.get(id)).toBe('new text');
    host.readOnlyA.set(true);
    await fixture.whenStable();
    await flush();
    expect(api.commands.some((c) => c.id === id && c.cmd.type === 'setReadOnly')).toBe(true);
    await ea.goTo(3, 2);
    expect(api.commands.at(-1)?.cmd).toEqual({ type: 'goTo', line: 2, ch: 1, select: false });
    api.matchCount = { total: 4, current: 1 };
    expect(await ea.find('fn')).toEqual({ total: 4, current: 1 });
    expect(ea.runAction('find.open')).toBe(true);
    await ea.setValue('set');
    expect(api.texts.get(id)).toBe('set');
    expect(await ea.getSlice(1, 1)).toBe('set');
  });

  it('plays a typing script on its own session, in order and stoppable', async () => {
    const { api, ea } = await setup();
    const id = [...api.texts.keys()][0];
    api.steps = [
      { at: 0, cmd: { type: 'goTo', line: 0, ch: 0, select: false }, clip: null },
      { at: 5, cmd: { type: 'type', text: 'a' }, clip: null },
      { at: 10, cmd: { type: 'paste' }, clip: 'from script' },
    ];
    const before = api.commands.length;
    await ea.playScript('ignored by the fake', 10);
    const played = api.commands.slice(before).filter((c) => c.id === id && c.cmd.type !== 'frame');
    expect(played.map((c) => c.cmd.type)).toEqual(['goTo', 'type', 'paste']);
    expect(api.clips).toEqual(['from script']);
    // a long script stops on stopScript
    api.steps = [{ at: 30, cmd: { type: 'type', text: 'late' }, clip: null }];
    const n = api.commands.length;
    const done = ea.playScript('');
    ea.stopScript();
    await done;
    expect(api.commands.slice(n).some((c) => c.cmd.type === 'type')).toBe(false);
  });

  it('closes its session when destroyed', async () => {
    const { api, fixture } = await setup();
    const ids = [...api.texts.keys()];
    fixture.destroy();
    await flush();
    expect(api.closed.sort()).toEqual(ids.sort());
  });
});
