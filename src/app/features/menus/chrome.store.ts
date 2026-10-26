import { computed, Injectable, signal } from '@angular/core';

import { Scheme, SCHEMES } from '../../core/settings/settings.models';

/** Colour schemes (design 008 E6, prompt 021; legacy: four built-in styles, F12 switches). */
export { SCHEMES };
export type { Scheme };

/** The schemes built on the dark base (`--scheme-base: dark` in `styles.css`). */
const DARK: ReadonlySet<Scheme> = new Set<Scheme>([
  'one',
  'moss',
  'mocha',
  'moon',
  'night',
  'graphite',
  'ink',
  'amber',
]);

export function isDarkScheme(s: Scheme): boolean {
  return DARK.has(s);
}

/** `data-theme` (chrome light/dark) and `data-scheme` of a scheme; `null` = follow the system. */
export function schemeAttributes(s: Scheme): {
  theme: 'light' | 'dark' | null;
  scheme: string | null;
} {
  if (s === 'auto') return { theme: null, scheme: null };
  const theme = isDarkScheme(s) ? 'dark' : 'light';
  return { theme, scheme: s === 'paper' || s === 'ink' ? null : s };
}

/** Attributes that draw a scheme inside one element (`[data-preview]` in `styles.css`). */
export function previewAttributes(s: Exclude<Scheme, 'auto'>): {
  preview: string;
  dark: '' | null;
} {
  return { preview: schemeAttributes(s).scheme ?? '', dark: isDarkScheme(s) ? '' : null };
}

/**
 * What is visible around the text (design 005 E1/E2, legacy F1 „Toggle Menu, Tabs, Iconbar“):
 * the HTML menu bar (Windows/Linux; hidden by default, Alt shows it for one use), everything at
 * once (F1) and full screen. Settings make it permanent in prompt 009.
 */
@Injectable({ providedIn: 'root' })
export class ChromeStore {
  /** The HTML menu bar stays visible. */
  readonly menuBar = signal(false);
  /** Shown for one visit (Alt) while `menuBar` is off. */
  readonly menuOnce = signal(false);
  /** F1: menu bar, tabs and status bar hidden – only the text. */
  readonly bare = signal(false);
  readonly fullscreen = signal(false);
  readonly scheme = signal<Scheme>('auto');
  /** The status bar (prompt 010). */
  readonly statusBar = signal(true);

  readonly menuVisible = computed(() => this.menuOnce() || (this.menuBar() && !this.bare()));

  toggleMenuBar(): void {
    this.menuBar.update((v) => !v);
    this.menuOnce.set(false);
  }

  toggleBare(): void {
    this.bare.update((v) => !v);
  }

  /** F12: the next colour scheme. */
  nextScheme(): void {
    const i = SCHEMES.indexOf(this.scheme());
    this.scheme.set(SCHEMES[(i + 1) % SCHEMES.length]);
  }
}
