import { TestBed } from '@angular/core/testing';
import { FindBar } from './find-bar';
import { FindStore, stepHistory } from './find.store';
import { parseGoto } from './goto-bar';

describe('find helpers', () => {
  it('parses go-to positions', () => {
    expect(parseGoto('12')).toEqual({ line: 11, ch: 0 });
    expect(parseGoto(' 7:3 ')).toEqual({ line: 6, ch: 2 });
    expect(parseGoto('7,3')).toEqual({ line: 6, ch: 2 });
    expect(parseGoto('0')).toEqual({ line: 0, ch: 0 });
    expect(parseGoto('x')).toBeNull();
  });

  it('steps through a history', () => {
    const h = ['c', 'b', 'a'];
    expect(stepHistory(h, '', true)).toBe('c');
    expect(stepHistory(h, 'c', true)).toBe('b');
    expect(stepHistory(h, 'a', true)).toBe('a');
    expect(stepHistory(h, 'b', false)).toBe('c');
    expect(stepHistory(h, 'c', false)).toBeNull();
    expect(stepHistory([], 'x', true)).toBeNull();
  });
});

describe('FindBar', () => {
  async function setup() {
    TestBed.configureTestingModule({ imports: [FindBar] });
    const fixture = TestBed.createComponent(FindBar);
    const store = TestBed.inject(FindStore);
    const steps: { backwards: boolean; incremental?: boolean }[] = [];
    let closed = 0;
    fixture.componentInstance.findStep.subscribe((s) => steps.push(s));
    fixture.componentInstance.closed.subscribe(() => closed++);
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    return { fixture, store, steps, el, closed: () => closed };
  }

  it('searches while typing, Enter/Shift+Enter step, Escape closes', async () => {
    const { store, steps, el, closed } = await setup();
    const input = el.querySelector<HTMLInputElement>('input')!;
    input.value = 'needle';
    input.dispatchEvent(new Event('input'));
    expect(store.pattern()).toBe('needle');
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', shiftKey: true }));
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(steps).toEqual([
      { backwards: false, incremental: true },
      { backwards: false },
      { backwards: true },
    ]);
    expect(store.findHistory()).toEqual(['needle']);
    expect(closed()).toBe(1);
  });

  it('toggles options with Alt+C/W/R and the buttons', async () => {
    const { store, el } = await setup();
    const input = el.querySelector<HTMLInputElement>('input')!;
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'c', code: 'KeyC', altKey: true }));
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'r', code: 'KeyR', altKey: true }));
    expect(store.caseSensitive()).toBe(true);
    expect(store.regex()).toBe(true);
    const word = el.querySelectorAll<HTMLButtonElement>('button.opt')[1];
    word.click();
    expect(store.wholeWord()).toBe(true);
  });
});
