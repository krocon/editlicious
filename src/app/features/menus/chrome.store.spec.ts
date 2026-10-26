import { TestBed } from '@angular/core/testing';
import { ChromeStore, schemeAttributes } from './chrome.store';

describe('ChromeStore', () => {
  it('cycles the colour schemes with F12 (legacy)', () => {
    const c = TestBed.inject(ChromeStore);
    const seen = [c.scheme()];
    for (let i = 0; i < 5; i++) {
      c.nextScheme();
      seen.push(c.scheme());
    }
    expect(seen).toEqual(['auto', 'paper', 'ink', 'sand', 'night', 'auto']);
  });

  it('maps schemes to theme and scheme attributes', () => {
    expect(schemeAttributes('auto')).toEqual({ theme: null, scheme: null });
    expect(schemeAttributes('night')).toEqual({ theme: 'dark', scheme: 'night' });
    expect(schemeAttributes('paper')).toEqual({ theme: 'light', scheme: null });
  });

  it('shows the menu bar for one use or for good', () => {
    const c = TestBed.inject(ChromeStore);
    expect(c.menuVisible()).toBe(false);
    c.menuOnce.set(true);
    expect(c.menuVisible()).toBe(true);
    c.toggleMenuBar();
    expect(c.menuVisible()).toBe(true);
    c.toggleBare();
    expect(c.menuVisible()).toBe(false);
  });
});
