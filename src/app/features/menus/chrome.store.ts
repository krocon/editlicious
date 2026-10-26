import { computed, Injectable, signal } from '@angular/core';

/** Colour schemes (design 008 E6; legacy: four built-in styles, F12 switches). */
export const SCHEMES = ['auto', 'paper', 'ink', 'sand', 'night'] as const;
export type Scheme = (typeof SCHEMES)[number];

/** `data-theme` (chrome light/dark) and `data-scheme` of a scheme; `null` = follow the system. */
export function schemeAttributes(s: Scheme): {
  theme: 'light' | 'dark' | null;
  scheme: string | null;
} {
  switch (s) {
    case 'auto':
      return { theme: null, scheme: null };
    case 'paper':
      return { theme: 'light', scheme: null };
    case 'ink':
      return { theme: 'dark', scheme: null };
    case 'sand':
      return { theme: 'light', scheme: 'sand' };
    case 'night':
      return { theme: 'dark', scheme: 'night' };
  }
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
