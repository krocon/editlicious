import { TestBed } from '@angular/core/testing';
import { ActionService } from './action.service';
import { KeyboardDispatcher } from './keyboard-dispatcher';
import { OS } from './platform';

const cleanups: (() => void)[] = [];
afterEach(() => cleanups.splice(0).forEach((f) => f()));

function setup() {
  TestBed.configureTestingModule({ providers: [{ provide: OS, useValue: 'other' }] });
  const actions = TestBed.inject(ActionService);
  const dispatcher = TestBed.inject(KeyboardDispatcher);
  const ran: string[] = [];
  const listener = (e: KeyboardEvent) => dispatcher.handle(e);
  document.addEventListener('keydown', listener);
  cleanups.push(() => document.removeEventListener('keydown', listener));
  const reg = (id: string) => actions.register(id, { run: () => ran.push(id) });
  dispatcher.setContextProvider((t) => (t?.closest('.ed') ? ['editor', 'global'] : ['global']));
  return { ran, reg };
}

function key(target: Element, k: string, init: KeyboardEventInit = {}): KeyboardEvent {
  const e = new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(e);
  return e;
}

describe('KeyboardDispatcher', () => {
  let host: HTMLDivElement;
  beforeEach(() => {
    host = document.createElement('div');
    host.innerHTML = `<div class="ed"><textarea id="ed" data-editor-input></textarea></div>
      <input id="t" type="text">`;
    document.body.appendChild(host);
  });
  afterEach(() => host.remove());
  const el = (id: string) => host.querySelector('#' + id)!;

  it('gives every key of the editor input to the actions', () => {
    const { ran, reg } = setup();
    reg('cursor.left');
    reg('edit.newline');
    reg('file.save');
    expect(key(el('ed'), 'ArrowLeft').defaultPrevented).toBe(true);
    key(el('ed'), 'Enter');
    key(el('ed'), 's', { ctrlKey: true });
    expect(ran).toEqual(['cursor.left', 'edit.newline', 'file.save']);
  });

  it('lets plain typing through to the textarea', () => {
    const { ran } = setup();
    expect(key(el('ed'), 'a').defaultPrevented).toBe(false);
    expect(ran).toEqual([]);
  });

  it('other text fields keep their keys except Mod shortcuts', () => {
    const { ran, reg } = setup();
    reg('cursor.left');
    reg('file.open');
    expect(key(el('t'), 'ArrowLeft').defaultPrevented).toBe(false);
    key(el('t'), 'o', { ctrlKey: true });
    expect(ran).toEqual(['file.open']);
  });

  it('opens the menu bar on a lone Alt (Windows/Linux)', () => {
    const { ran, reg } = setup();
    const dispatcher = TestBed.inject(KeyboardDispatcher);
    const up = (e: KeyboardEvent) => dispatcher.keyup(e);
    document.addEventListener('keyup', up);
    cleanups.push(() => document.removeEventListener('keyup', up));
    reg('app.focusMenu');
    key(el('ed'), 'Alt', { altKey: true });
    el('ed').dispatchEvent(new KeyboardEvent('keyup', { key: 'Alt', bubbles: true }));
    expect(ran).toEqual(['app.focusMenu']);
    // Alt as a modifier of another key does not count
    key(el('ed'), 'Alt', { altKey: true });
    key(el('ed'), 'x', { altKey: true });
    el('ed').dispatchEvent(new KeyboardEvent('keyup', { key: 'Alt', bubbles: true }));
    expect(ran).toEqual(['app.focusMenu']);
  });

  it('ignores keys while composing', () => {
    const { ran, reg } = setup();
    reg('edit.newline');
    key(el('ed'), 'Enter', { isComposing: true });
    expect(ran).toEqual([]);
  });
});
