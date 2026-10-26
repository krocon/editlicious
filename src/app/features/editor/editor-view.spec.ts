import { SettingsApi } from '../../core/settings/settings-api';
import { FakeSettingsApi } from '../../../testing/fake-settings-api';
import { TestBed } from '@angular/core/testing';
import { EditorApi } from '../../core/editor/editor-api';
import { FakeEditorApi, textFrame } from '../../../testing/fake-editor-api';
import { EditorView } from './editor-view';
import { PickService } from '../../shared/pick/pick.service';
import { ActionService } from '../../core/actions/action.service';
import { Command } from '../../core/editor/editor.models';
import { SettingsStore } from '../settings/settings.store';
import { KeyboardDispatcher } from '../../core/actions/keyboard-dispatcher';
import { OS } from '../../core/actions/platform';

const cleanups: (() => void)[] = [];
/** Lets the serial command chain of the view run. */
const flush = () => new Promise((r) => setTimeout(r, 0));

async function setup(api = new FakeEditorApi()) {
  TestBed.configureTestingModule({
    imports: [EditorView],
    providers: [
      { provide: SettingsApi, useClass: FakeSettingsApi },
      { provide: EditorApi, useValue: api },
      { provide: OS, useValue: 'other' },
    ],
  });
  // jsdom has no layout: give the host a size
  const rect = { top: 0, left: 0, right: 800, bottom: 440, width: 800, height: 440 } as DOMRect;
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue(rect);
  const dispatcher = TestBed.inject(KeyboardDispatcher);
  dispatcher.setContextProvider(() => ['editor', 'global']);
  const listener = (e: KeyboardEvent) => dispatcher.handle(e);
  document.addEventListener('keydown', listener);
  cleanups.push(() => document.removeEventListener('keydown', listener));
  const fixture = TestBed.createComponent(EditorView);
  document.body.appendChild(fixture.nativeElement);
  cleanups.push(() => fixture.nativeElement.remove());
  fixture.componentRef.setInput('docId', 7);
  await fixture.whenStable();
  await new Promise((r) => setTimeout(r));
  await fixture.whenStable();
  return { fixture, api, el: fixture.nativeElement as HTMLElement };
}

describe('EditorView', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    cleanups.splice(0).forEach((f) => f());
  });

  it('sends its size and renders the frame', async () => {
    const { api, el } = await setup();
    const resize = api.sent('resize');
    expect(resize.length).toBeGreaterThan(0);
    expect(api.commands.every((c) => c.id === 7)).toBe(true);
    const rows = el.querySelectorAll('.row');
    expect(rows.length).toBe(2);
    expect(rows[0].textContent).toContain('hello');
    expect(el.querySelectorAll('.ln')[1].textContent?.trim()).toBe('2');
  });

  it('follows the settings: indentation, relative line numbers, guides', async () => {
    const api = new FakeEditorApi();
    api.nextFrame = textFrame(['a', '    b', '        c'], {
      tabWidth: 4,
      caretLine: 1,
    });
    api.nextFrame.lines[1].indent = 4;
    api.nextFrame.lines[2].indent = 8;
    const { el, fixture } = await setup(api);
    expect(api.sent('configure').at(-1)).toEqual({
      type: 'configure',
      tabWidth: 4,
      insertSpaces: true,
      detect: true,
      wrap: false,
    });
    // one guide per indent step, none at the left edge
    expect(el.querySelectorAll('.row')[2].querySelectorAll('.guide').length).toBe(1);
    const settings = TestBed.inject(SettingsStore);
    settings.update({ lineNumbers: 'relative', indentWith: 'tabs', tabWidth: 8 });
    await fixture.whenStable();
    await flush();
    await fixture.whenStable();
    const ln = [...el.querySelectorAll('.ln')].map((e) => e.textContent?.trim());
    expect(ln).toEqual(['1', '2', '1']);
    expect(api.sent('configure').at(-1)).toEqual({
      type: 'configure',
      tabWidth: 8,
      insertSpaces: false,
      detect: false,
      wrap: false,
    });
    settings.update({ lineNumbers: 'off', indentGuides: false });
    await fixture.whenStable();
    expect(el.querySelector('.ln')?.textContent?.trim()).toBe('');
    expect(el.querySelectorAll('.guide').length).toBe(0);
  });

  it('draws wrapped lines row by row (design 010 E4)', async () => {
    const api = new FakeEditorApi();
    api.nextFrame = textFrame(['aaaa bbbb cccc', 'x'], { wrap: true });
    const l0 = api.nextFrame.lines[0];
    l0.wraps = [5, 10];
    l0.runs = [
      { cell: 0, width: 5, kind: 'text', text: 'aaaa ', style: 0 },
      { cell: 5, width: 5, kind: 'text', text: 'bbbb ', style: 0 },
      { cell: 10, width: 4, kind: 'text', text: 'cccc', style: 0 },
    ];
    l0.sel = [[3, 12]];
    api.nextFrame.lines[1].row = 3;
    api.nextFrame.carets = [{ line: 0, cell: 12 }];
    api.nextFrame.caretXy = [{ row: 2, x: 2 }];
    const { el, fixture } = await setup(api);
    await fixture.whenStable();
    const lh = parseFloat(el.style.getPropertyValue('--lh'));
    const runs = [
      ...el.querySelectorAll<HTMLElement>('.row')[0].querySelectorAll<HTMLElement>('.run'),
    ];
    expect(runs.map((r) => parseFloat(r.style.top))).toEqual([0, lh, 2 * lh]);
    expect(runs.map((r) => parseFloat(r.style.left))).toEqual([0, 0, 0]);
    // the selection splits into one box per row
    expect(el.querySelectorAll('.row')[0].querySelectorAll('.sel').length).toBe(3);
    // the second line starts below the three rows
    expect(el.querySelectorAll<HTMLElement>('.row')[1].style.transform).toBe(
      `translateY(${3 * lh}px)`,
    );
    const caret = el.querySelector<HTMLElement>('.caret')!;
    expect(caret.style.transform).toContain(`${2 * lh}px`);
    // wrapping follows the setting
    TestBed.inject(SettingsStore).update({ wordWrap: true });
    await fixture.whenStable();
    await flush();
    expect(api.sent('configure').at(-1)).toMatchObject({ wrap: true });
  });

  it('asks for the overview when search or markers are active (design 010 E3)', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const api = new FakeEditorApi();
    api.nextFrame = textFrame(
      Array.from({ length: 30 }, (_, i) => `line ${i}`),
      { deco: 7 },
    );
    api.nextFrame.info.lineCount = 10_000;
    api.overviewAnswer = [1, 0, 2, 3];
    const setupP = setup(api);
    await vi.advanceTimersByTimeAsync(10);
    const { el, fixture } = await setupP;
    await vi.advanceTimersByTimeAsync(400);
    await fixture.whenStable();
    vi.useRealTimers();
    expect(api.overviews.length).toBe(1);
    const ticks = el.querySelectorAll('.vbar .tick');
    expect(ticks.length).toBe(3);
    expect(el.querySelectorAll('.vbar .tick.marker').length).toBe(2);
  });

  it('converts line endings and copies as HTML through the engine', async () => {
    const { api, fixture } = await setup();
    const picker = TestBed.inject(PickService);
    vi.spyOn(picker, 'pick').mockResolvedValue('crlf');
    const actions = TestBed.inject(ActionService);
    actions.run('file.lineEnding');
    await flush();
    await fixture.whenStable();
    await flush();
    expect(api.sent('setLineEnding')).toEqual([{ type: 'setLineEnding', lineEnding: 'crlf' }]);
    actions.run('edit.copyHtmlNumbers');
    await flush();
    const html = api.sent('copyHtml')[0] as Extract<Command, { type: 'copyHtml' }>;
    expect(html.lineNumbers).toBe(true);
    expect(html.styles.length).toBe(26);
    expect(html.base).toContain('padding');
  });

  it('turns keys into engine commands', async () => {
    const { api, el, fixture } = await setup();
    const input = el.querySelector('textarea')!;
    input.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowDown', shiftKey: true, bubbles: true }),
    );
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await flush();
    await fixture.whenStable();
    expect(api.sent('move')).toEqual([{ type: 'move', to: 'down', select: true }]);
    expect(api.sent('newline').length).toBe(1);
  });

  it('sends typed text and clears the textarea', async () => {
    const { api, el, fixture } = await setup();
    const input = el.querySelector('textarea')!;
    input.value = 'ä';
    input.dispatchEvent(new Event('input'));
    await fixture.whenStable();
    expect(api.sent('type')).toEqual([{ type: 'type', text: 'ä' }]);
    expect(input.value).toBe('');
  });

  it('keeps IME compositions local until they end', async () => {
    const { api, el, fixture } = await setup();
    const input = el.querySelector('textarea')!;
    input.dispatchEvent(new CompositionEvent('compositionstart'));
    input.value = 'に';
    input.dispatchEvent(new Event('input'));
    input.dispatchEvent(new CompositionEvent('compositionend', { data: '日本' }));
    await fixture.whenStable();
    expect(api.sent('type')).toEqual([{ type: 'type', text: '日本' }]);
  });

  it('clicks become line/cell positions', async () => {
    const { api, el, fixture } = await setup();
    const text = el.querySelector('.text')!;
    text.dispatchEvent(new MouseEvent('mousedown', { clientX: 400, clientY: 50, detail: 2 }));
    await fixture.whenStable();
    const [click] = api.sent('click') as { line: number; cell: number; count: number }[];
    expect(click.count).toBe(2);
    expect(click.line).toBeGreaterThan(2);
    expect(click.cell).toBeGreaterThan(0);
    document.dispatchEvent(new MouseEvent('mouseup'));
  });

  it('draws selections and long-line markers from the frame', async () => {
    const api = new FakeEditorApi();
    const f = textFrame(['abc', 'def']);
    f.lines[0].sel = [[1, 3]];
    f.lines[0].selEol = true;
    f.lines[1].long = true;
    api.nextFrame = f;
    const { el } = await setup(api);
    expect(el.querySelectorAll('.sel').length).toBe(2);
    expect(el.querySelectorAll('.row.long').length).toBe(1);
  });

  it('find: Mod+F takes the selection, typing searches and counts, Escape clears', async () => {
    const api = new FakeEditorApi();
    api.nextFrame = textFrame(['one two one'], { selectionText: 'one' });
    api.matchCount = { total: 2, current: 1 };
    const { el, fixture } = await setup(api);
    const input = el.querySelector('textarea')!;
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'f', ctrlKey: true, bubbles: true }));
    await flush();
    await fixture.whenStable();
    const field = el.querySelector<HTMLInputElement>('app-find-bar input')!;
    expect(field.value).toBe('one');
    expect(api.sent('find')[0]).toMatchObject({ incremental: true, query: { pattern: 'one' } });
    await new Promise((r) => setTimeout(r, 150));
    await flush();
    await fixture.whenStable();
    expect(api.counted.length).toBe(1);
    expect(el.querySelector('app-find-bar .count')?.textContent).toContain('1 of 2');
    field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await flush();
    await fixture.whenStable();
    expect(el.querySelector('app-find-bar')).toBeNull();
    expect(api.sent('clearSearch').length).toBe(1);
  });

  it('go to line sends a position', async () => {
    const { el, fixture, api } = await setup();
    el.querySelector('textarea')!.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'g', ctrlKey: true, bubbles: true }),
    );
    await fixture.whenStable();
    const field = el.querySelector<HTMLInputElement>('app-goto-bar input')!;
    field.value = '20:4';
    field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    await flush();
    expect(api.sent('goTo')).toEqual([{ type: 'goTo', line: 19, ch: 3 }]);
  });

  it('draws search marks', async () => {
    const api = new FakeEditorApi();
    const f = textFrame(['abcabc']);
    f.lines[0].marks = [
      [0, 3, 1],
      [3, 6, 0],
    ];
    api.nextFrame = f;
    const { el } = await setup(api);
    expect(el.querySelectorAll('.mark').length).toBe(2);
    expect(el.querySelectorAll('.mark.current').length).toBe(1);
  });

  it('Alt-click adds a cursor, Alt-drag selects a column, Escape goes back to one', async () => {
    const api = new FakeEditorApi();
    api.nextFrame = textFrame(['abc', 'def'], { multi: true });
    const { el, fixture } = await setup(api);
    const text = el.querySelector('.text')!;
    text.dispatchEvent(new MouseEvent('mousedown', { clientX: 100, clientY: 5, altKey: true }));
    document.dispatchEvent(
      new MouseEvent('mousemove', { clientX: 140, clientY: 30, altKey: true }),
    );
    await new Promise((r) => requestAnimationFrame(() => r(null)));
    await flush();
    document.dispatchEvent(new MouseEvent('mouseup'));
    expect(api.sent('click')[0]).toMatchObject({ add: true });
    expect(api.sent('drag')[0]).toMatchObject({ column: true });
    el.querySelector('textarea')!.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }),
    );
    await flush();
    await fixture.whenStable();
    expect(api.sent('singleCursor').length).toBe(1);
  });

  it('draws word markers in their colour', async () => {
    const api = new FakeEditorApi();
    const f = textFrame(['let x = x']);
    f.lines[0].markers = [
      [4, 5, 0],
      [8, 9, 3],
    ];
    api.nextFrame = f;
    const { el } = await setup(api);
    const m = el.querySelectorAll('.marker');
    expect(m.length).toBe(2);
    expect(m[1].getAttribute('data-colour')).toBe('3');
  });
});
