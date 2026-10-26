import { TestBed } from '@angular/core/testing';
import { EditorApi } from '../../core/editor/editor-api';
import { FakeEditorApi, textFrame } from '../../../testing/fake-editor-api';
import { EditorView } from './editor-view';
import { KeyboardDispatcher } from '../../core/actions/keyboard-dispatcher';
import { OS } from '../../core/actions/platform';

const cleanups: (() => void)[] = [];
/** Lets the serial command chain of the view run. */
const flush = () => new Promise((r) => setTimeout(r, 0));

async function setup(api = new FakeEditorApi()) {
  TestBed.configureTestingModule({
    imports: [EditorView],
    providers: [
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
});
