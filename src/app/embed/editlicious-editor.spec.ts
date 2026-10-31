import { Component, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { OS } from '../core/actions/platform';
import { textFrame } from '../../testing/fake-editor-api';
import { FakeEmbedEngineApi } from '../../testing/fake-embed-engine-api';
import { EdCaret, EdChange, EditliciousEditor } from './editlicious-editor';
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
  readonly schemeA = signal<'paper' | 'night' | 'auto'>('paper');
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
    api.nextFrame = textFrame(['fn main() {}', 'x'], { caretLine: 1, caretCh: 1 });
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
    expect(api.textReads).toBe(0);
    expect(await ea.getValue()).toBe('fn main() {}\n');
    expect(api.textReads).toBe(1);
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

  it('closes its session when destroyed', async () => {
    const { api, fixture } = await setup();
    const ids = [...api.texts.keys()];
    fixture.destroy();
    await flush();
    expect(api.closed.sort()).toEqual(ids.sort());
  });
});
