import { computed, Injectable, signal } from '@angular/core';

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

  readonly menuVisible = computed(() => this.menuOnce() || (this.menuBar() && !this.bare()));

  toggleMenuBar(): void {
    this.menuBar.update((v) => !v);
    this.menuOnce.set(false);
  }

  toggleBare(): void {
    this.bare.update((v) => !v);
  }
}
