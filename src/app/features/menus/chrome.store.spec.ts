import { TestBed } from '@angular/core/testing';
import { ChromeStore, previewAttributes, schemeAttributes, SCHEMES } from './chrome.store';

describe('ChromeStore', () => {
  it('cycles the colour schemes with F12 (legacy)', () => {
    const c = TestBed.inject(ChromeStore);
    const seen = [c.scheme()];
    SCHEMES.forEach(() => {
      c.nextScheme();
      seen.push(c.scheme());
    });
    expect(seen).toEqual([...SCHEMES, 'auto']);
  });

  it('lists the schemes from light to dark like the stylesheet test (prompt 021)', () => {
    // = ORDER in src-tauri/video/src/theme.rs, which checks it against the backgrounds
    expect(SCHEMES).toEqual([
      'auto',
      'paper',
      'glass',
      'sand',
      'harvest',
      'chalk',
      'one',
      'moss',
      'mocha',
      'moon',
      'night',
      'graphite',
      'ink',
      'amber',
    ]);
    expect(previewAttributes('paper')).toEqual({ preview: '', dark: null });
    expect(previewAttributes('ink')).toEqual({ preview: '', dark: '' });
    expect(previewAttributes('moon')).toEqual({ preview: 'moon', dark: '' });
    expect(previewAttributes('chalk')).toEqual({ preview: 'chalk', dark: null });
  });

  it('maps schemes to theme and scheme attributes', () => {
    expect(schemeAttributes('auto')).toEqual({ theme: null, scheme: null });
    expect(schemeAttributes('night')).toEqual({ theme: 'dark', scheme: 'night' });
    expect(schemeAttributes('paper')).toEqual({ theme: 'light', scheme: null });
    expect(schemeAttributes('ink')).toEqual({ theme: 'dark', scheme: null });
    expect(schemeAttributes('harvest')).toEqual({ theme: 'light', scheme: 'harvest' });
    expect(schemeAttributes('amber')).toEqual({ theme: 'dark', scheme: 'amber' });
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
