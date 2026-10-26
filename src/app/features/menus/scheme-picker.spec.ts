import { TestBed } from '@angular/core/testing';
import { OS } from '../../core/actions/platform';
import { DialogService } from '../../shared/dialog/dialog.service';
import { ChromeStore, SCHEMES } from './chrome.store';
import { SchemePicker } from './scheme-picker';

async function setup() {
  TestBed.configureTestingModule({
    imports: [SchemePicker],
    providers: [{ provide: OS, useValue: 'other' }],
  });
  const chrome = TestBed.inject(ChromeStore);
  chrome.scheme.set('sand');
  const dialogs = TestBed.inject(DialogService);
  dialogs.show('scheme');
  const fixture = TestBed.createComponent(SchemePicker);
  document.body.appendChild(fixture.nativeElement);
  await fixture.whenStable();
  const el = fixture.nativeElement as HTMLElement;
  return { fixture, el, chrome, dialogs };
}

const key = (target: Element, k: string) =>
  target.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true }));

describe('SchemePicker (prompt 021)', () => {
  afterEach(() => document.body.replaceChildren());

  it('shows every scheme as a thumbnail drawn in its own colours, light to dark', async () => {
    const { el } = await setup();
    const radios = [...el.querySelectorAll('[role="radio"]')];
    expect(radios.map((r) => r.getAttribute('data-scheme-id'))).toEqual([...SCHEMES]);
    expect(el.querySelector('[role="radiogroup"]')).not.toBeNull();
    const moon = el.querySelector('[data-scheme-id="moon"] .preview')!;
    expect(moon.getAttribute('data-preview')).toBe('moon');
    expect(moon.hasAttribute('data-dark')).toBe(true);
    const paper = el.querySelector('[data-scheme-id="paper"] .preview')!;
    expect(paper.getAttribute('data-preview')).toBe('');
    expect(paper.hasAttribute('data-dark')).toBe(false);
    // the system thumbnail shows light and dark
    expect(el.querySelectorAll('[data-scheme-id="auto"] .preview').length).toBe(2);
    expect(el.querySelector('[data-scheme-id="moss"] .name')?.textContent).toBe('Moss (dark)');
    // roving tab stop on the current scheme
    const sand = el.querySelector('[data-scheme-id="sand"]')!;
    expect(sand.getAttribute('aria-checked')).toBe('true');
    expect(sand.getAttribute('tabindex')).toBe('0');
    expect(el.querySelectorAll('[tabindex="0"][role="radio"]').length).toBe(1);
  });

  it('arrows try a scheme, Enter keeps it', async () => {
    const { fixture, el, chrome, dialogs } = await setup();
    const current = () => el.querySelector(`[data-scheme-id="${chrome.scheme()}"]`)!;
    key(current(), 'ArrowRight');
    expect(chrome.scheme()).toBe('harvest');
    key(current(), 'ArrowDown');
    expect(chrome.scheme()).toBe(SCHEMES[SCHEMES.indexOf('harvest') + 4]);
    key(current(), 'Home');
    expect(chrome.scheme()).toBe('auto');
    key(current(), 'ArrowLeft');
    expect(chrome.scheme()).toBe('amber');
    await fixture.whenStable();
    await new Promise((r) => setTimeout(r));
    expect(document.activeElement?.getAttribute('data-scheme-id')).toBe('amber');
    key(current(), 'Enter');
    expect(dialogs.open()).toBeNull();
    expect(chrome.scheme()).toBe('amber');
  });

  it('a click tries, Esc goes back to the scheme before', async () => {
    const { fixture, el, chrome, dialogs } = await setup();
    (el.querySelector('[data-scheme-id="graphite"]') as HTMLElement).click();
    expect(chrome.scheme()).toBe('graphite');
    await fixture.whenStable();
    key(el.querySelector('[data-scheme-id="graphite"]')!, 'Escape');
    expect(chrome.scheme()).toBe('sand');
    expect(dialogs.open()).toBeNull();
  });
});
