import { TestBed } from '@angular/core/testing';
import { Command } from '../core/editor/editor.models';
import { FakeEmbedEngineApi } from '../../testing/fake-embed-engine-api';
import {
  COMPLETION_ROWS,
  dateVars,
  EdCompletionContext,
  EdCompletionItem,
  filterItems,
  kindName,
  ProviderCompletion,
} from './provider-completion';
import { EmbedEngineApi } from './wasm-editor-api';

const flush = async () => {
  for (let i = 0; i < 6; i++) await new Promise((r) => setTimeout(r));
};
const type = (text: string): Command => ({ type: 'type', text });

function setup() {
  const api = new FakeEmbedEngineApi();
  TestBed.configureTestingModule({
    providers: [ProviderCompletion, { provide: EmbedEngineApi, useValue: api }],
  });
  const c = TestBed.inject(ProviderCompletion);
  c.bind({ docId: () => 7, enqueue: (fn) => fn() });
  const labels = () => c.list()?.items.map((i) => i.label) ?? null;
  /** Types `text` character by character, the caret behind `before + typed`. */
  const typing = async (before: string, text: string) => {
    let now = before;
    for (const ch of text) {
      now += ch;
      api.caretAfter(now);
      c.after(type(ch));
      await flush();
    }
  };
  return { api, c, labels, typing };
}

const TABLES: EdCompletionItem[] = [
  { label: 'orders', kind: 'table', detail: 'public' },
  { label: 'order_items', kind: 'table' },
  { label: 'customers', kind: 'table', documentation: 'Who buys.' },
  { label: 'ORDER BY', kind: 'Key Word!', sortText: 'zz' },
];

describe('ProviderCompletion (design 028)', () => {
  it('asks the provider on request with the caret, its line and the word begun', async () => {
    const { api, c, labels } = setup();
    let seen: EdCompletionContext | null = null;
    c.provider.set((ctx) => {
      seen = ctx;
      return TABLES;
    });
    api.caretAfter('SELECT * FROM or', 2);
    c.invoke();
    await flush();
    expect(seen).toMatchObject({
      position: { line: 3, ch: 17 },
      lineText: 'SELECT * FROM or',
      word: 'or',
      wordStart: { line: 3, ch: 15 },
      trigger: 'invoke',
      triggerCharacter: null,
    });
    // sorted by sortText/label; those starting with the word first
    expect(labels()).toEqual(['order_items', 'orders', 'ORDER BY']);
    expect(c.list()).toMatchObject({ line: 2, cell: 14, total: 3, active: 0, top: 0 });
    expect(c.list()!.items[1]).toEqual({ label: 'orders', kind: 'table', detail: 'public' });
    // text around the caret only on request
    expect(await seen!.getRange({ line: 1, ch: 1 }, { line: 3, ch: 17 })).toBe('0:0-2:16');
    expect(api.textReads).toBe(0);
  });

  it('filters while typing without asking again when the answer is complete', async () => {
    const { api, c, labels, typing } = setup();
    let asked = 0;
    c.provider.set(() => {
      asked++;
      return TABLES;
    });
    api.caretAfter('FROM ');
    c.invoke();
    await flush();
    expect(labels()).toEqual(['customers', 'order_items', 'orders', 'ORDER BY']);
    await typing('FROM ', 'ord');
    expect(labels()).toEqual(['order_items', 'orders', 'ORDER BY']);
    await typing('FROM ord', 'ers');
    expect(labels()).toEqual(['orders']);
    expect(asked).toBe(1);
    // back again: the kept answer is filtered again
    api.caretAfter('FROM or');
    c.after({ type: 'backspace' });
    await flush();
    expect(labels()?.length).toBe(3);
    // contained, not at the start: behind the others
    api.caretAfter('FROM ');
    c.invoke();
    await flush();
    await typing('FROM ', 'ite');
    expect(labels()).toEqual(['order_items']);
    // nothing fits: no list; moving away closes for good
    await typing('FROM ite', 'x');
    expect(c.list()).toBeNull();
    c.after({ type: 'move', to: 'left' });
    expect(c.list()).toBeNull();
  });

  it('asks again on every key when the answer is incomplete', async () => {
    const { api, c, labels, typing } = setup();
    const words: string[] = [];
    c.provider.set((ctx) => {
      words.push(ctx.word);
      return { items: [{ label: ctx.word + 'X' }], incomplete: true };
    });
    api.caretAfter('a');
    c.invoke();
    await flush();
    await typing('a', 'bc');
    expect(words).toEqual(['a', 'ab', 'abc']);
    expect(labels()).toEqual(['abcX']);
  });

  it('asks while typing from two characters and at trigger characters – when it is on', async () => {
    const { c, labels, typing } = setup();
    const triggers: string[] = [];
    const provider = (ctx: EdCompletionContext) => {
      triggers.push(`${ctx.trigger}:${ctx.triggerCharacter ?? ''}:${ctx.word}`);
      return TABLES;
    };
    provider.triggerCharacters = ['.', ' '];
    c.provider.set(provider);
    // typing does not ask unless the editor suggests while typing
    await typing('', 'or');
    expect(triggers).toEqual([]);
    c.auto.set(true);
    await typing('', 'o');
    expect(triggers).toEqual([]);
    await typing('o', 'r');
    expect(triggers).toEqual(['type:r:or']);
    expect(labels()?.[0]).toBe('order_items');
    // a trigger character asks at once, the word is empty
    c.close();
    await typing('o', '.');
    expect(triggers.at(-1)).toBe('character:.:');
    expect(c.list()?.total).toBe(4);
    // typing on into another word (a blank, not a trigger here) closes
    c.provider.set((ctx) => provider(ctx));
    await typing('o.', 'cu');
    expect(labels()).toEqual(['customers']);
    await typing('o.cu', ' ');
    expect(c.list()).toBeNull();
  });

  it('drops a late or aborted answer: no stale list, no flicker', async () => {
    const { api, c, typing } = setup();
    const answers: ((items: EdCompletionItem[]) => void)[] = [];
    const signals: AbortSignal[] = [];
    c.provider.set(
      (ctx) =>
        new Promise<EdCompletionItem[]>((resolve) => {
          answers.push(resolve);
          signals.push(ctx.signal);
        }),
    );
    c.auto.set(true);
    await typing('', 'or');
    expect(answers.length).toBe(1);
    // typed on before the answer: a new request, the old one is aborted
    await typing('or', 'd');
    expect(signals[0].aborted).toBe(true);
    expect(answers.length).toBe(2);
    answers[0](TABLES);
    await flush();
    expect(c.list()).toBeNull();
    // the caret moved away: the answer comes too late
    c.after({ type: 'move', to: 'down' });
    expect(signals[1].aborted).toBe(true);
    answers[1](TABLES);
    await flush();
    expect(c.list()).toBeNull();
    // a failing provider is no list and no error for the editor
    c.provider.set(() => Promise.reject(new Error('down')));
    const err = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    api.caretAfter('ord');
    c.invoke();
    await flush();
    expect(c.list()).toBeNull();
    expect(err).toHaveBeenCalled();
    err.mockRestore();
  });

  it('takes an entry: the word up to the caret is replaced, a snippet is inserted as one', async () => {
    const { api, c, typing } = setup();
    c.provider.set(() => [
      { label: 'orders', insertText: 'public.orders' },
      { label: 'sel', insertText: 'SELECT ${1:*} FROM $0', snippet: true },
      { label: 'zrange', range: { start: { line: 1, ch: 1 }, end: { line: 1, ch: 4 } } },
    ]);
    api.caretAfter('x or');
    c.invoke();
    await flush();
    // typed on right before taking it: the replaced word ends at the caret
    await typing('x or', 'd');
    api.commands = [];
    c.accept();
    await flush();
    expect(api.commands.at(-1)?.cmd).toEqual({
      type: 'replaceRange',
      from: { line: 0, ch: 2 },
      to: { line: 0, ch: 5 },
      text: 'public.orders',
      caret: true,
    });
    expect(c.list()).toBeNull();
    api.caretAfter('se');
    c.invoke();
    await flush();
    c.accept(0);
    await flush();
    expect(api.commands.at(-1)?.cmd).toMatchObject({
      type: 'insertSnippet',
      template: 'SELECT ${1:*} FROM $0',
      from: { line: 0, ch: 0 },
      to: { line: 0, ch: 2 },
    });
    // the entry's own range
    api.caretAfter('z');
    c.invoke();
    await flush();
    c.accept();
    await flush();
    expect(api.commands.at(-1)?.cmd).toMatchObject({
      from: { line: 0, ch: 0 },
      to: { line: 0, ch: 3 },
      text: 'zrange',
    });
  });

  it('mixes sources: provider first, then snippets and words – or only one of them', async () => {
    const { api, c, labels } = setup();
    let asked = 0;
    c.provider.set(() => {
      asked++;
      return [{ label: 'orders' }, { label: 'ordinal' }];
    });
    c.snippets.set([{ prefix: 'ord', body: ['ORDER BY ${1:col}', '$0'], description: 'order by' }]);
    api.caretAfter('ord');
    api.context.words = [
      ['ordinal', false],
      ['ordered', true],
    ];
    c.invoke();
    await flush();
    // a word the provider already names is not listed twice
    expect(labels()).toEqual(['orders', 'ordinal', 'ord', 'ordered']);
    expect(c.list()!.items.map((i) => i.kind)).toEqual(['', '', 'snippet', 'keyword']);
    expect(c.list()!.items[2].detail).toBe('order by');
    c.sources.set('provider');
    c.invoke();
    await flush();
    expect(labels()).toEqual(['orders', 'ordinal', 'ord']);
    c.sources.set('words');
    c.invoke();
    await flush();
    expect(labels()).toEqual(['ord', 'ordinal', 'ordered']);
    expect(asked).toBe(2);
    // without provider and snippets the engine's own completion does everything
    c.snippets.set([]);
    expect(c.on()).toBe(false);
    c.sources.set('both');
    expect(c.on()).toBe(true);
    c.provider.set(null);
    expect(c.on()).toBe(false);
  });

  it('Tab expands the snippet whose prefix stands before the caret', async () => {
    const { api, c } = setup();
    c.snippets.set([{ prefix: 'sel', body: 'SELECT ${1:*} FROM ${2:table};$0' }]);
    api.caretAfter('  sel');
    expect(await c.tabCommand()).toMatchObject({
      type: 'insertSnippet',
      template: 'SELECT ${1:*} FROM ${2:table};$0',
      from: { line: 0, ch: 2 },
      to: { line: 0, ch: 5 },
    });
    api.caretAfter('  sele');
    expect(await c.tabCommand()).toBeNull();
    api.caretAfter('  sel ');
    expect(await c.tabCommand()).toBeNull();
    api.context.single = false;
    api.caretAfter('  sel');
    expect(await c.tabCommand()).toBeNull();
  });

  it('keeps 50 000 entries fluid: ten rows on screen, a filter pass per key', async () => {
    const { api, c, typing } = setup();
    const items: EdCompletionItem[] = [];
    for (let i = 0; i < 50_000; i++) {
      items.push({ label: `tbl_${(i * 7919) % 50_000}_col_${i % 13}`, kind: 'column' });
    }
    let asked = 0;
    c.provider.set(() => {
      asked++;
      return items;
    });
    api.caretAfter('t');
    c.invoke();
    await flush();
    expect(c.list()).toMatchObject({ total: 50_000, top: 0 });
    expect(c.list()!.items.length).toBe(COMPLETION_ROWS);
    const t0 = performance.now();
    await typing('t', 'bl_49');
    const perKey = (performance.now() - t0) / 5;
    expect(asked).toBe(1);
    expect(c.list()!.total).toBeLessThan(5_000);
    expect(perKey).toBeLessThan(60);
    // the list scrolls: arrow up from the first entry goes to the last, the wheel moves rows
    const total = c.list()!.total;
    c.move(-1);
    expect(c.list()).toMatchObject({ active: total - 1, top: total - COMPLETION_ROWS });
    c.move(1);
    expect(c.list()).toMatchObject({ active: 0, top: 0 });
    c.scroll(3);
    expect(c.list()).toMatchObject({ active: 3, top: 3 });
    // the pure filter: well under a frame for 50 000 entries
    const prepared = items.map((it) => ({ filter: it.label.toLowerCase() }));
    const f0 = performance.now();
    for (let i = 0; i < 20; i++) filterItems(prepared, 'col_1');
    expect((performance.now() - f0) / 20).toBeLessThan(16);
  });

  it('names kinds safely and knows the date variables', () => {
    expect(kindName('Key Word!')).toBe('key-word');
    expect(kindName(undefined)).toBe('');
    const v = dateVars(new Date(2026, 10, 9, 8, 7, 6));
    expect(v).toMatchObject({
      CURRENT_YEAR: '2026',
      CURRENT_MONTH: '11',
      CURRENT_DATE: '09',
      DATE: '2026-11-09',
      TIME: '08:07:06',
    });
  });
});
