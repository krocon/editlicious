import { DOCUMENT } from '@angular/common';
import { computed, effect, inject, Injectable, signal, untracked } from '@angular/core';
import { setLanguage, systemLanguage } from '../../core/i18n/i18n';
import { SettingsApi } from '../../core/settings/settings-api';
import { DEFAULT_SETTINGS, Settings } from '../../core/settings/settings.models';
import { ChromeStore } from '../menus/chrome.store';

/** Bundled writes (zoom with a held key). */
const SAVE_DELAY = 400;
export const MIN_FONT = 6;
export const MAX_FONT = 72;

/**
 * The settings (design 009): loaded once, every change applied at once and saved shortly
 * after. Scheme and menu bar live in the {@link ChromeStore} at runtime (F12, Strg+M) and are
 * written back from there.
 */
@Injectable({ providedIn: 'root' })
export class SettingsStore {
  private readonly api = inject(SettingsApi);
  private readonly chrome = inject(ChromeStore);
  private readonly doc = inject(DOCUMENT);
  readonly settings = signal<Settings>(DEFAULT_SETTINGS);
  readonly loaded = signal(false);
  private timer: ReturnType<typeof setTimeout> | null = null;

  /** The editor font as a CSS value. */
  readonly fontFamily = computed(() => {
    const f = this.settings().fontFamily.trim();
    return f ? `'${f}', var(--font-editor-default)` : 'var(--font-editor-default)';
  });

  private readonly chromePart = computed(
    () => {
      const s = this.settings();
      return { scheme: s.scheme, menuBar: s.menuBar, statusBar: s.statusBar };
    },
    {
      equal: (a, b) =>
        a.scheme === b.scheme && a.menuBar === b.menuBar && a.statusBar === b.statusBar,
    },
  );

  constructor() {
    // apply: language, fonts, scheme and menu bar into the chrome
    effect(() => {
      const s = this.settings();
      setLanguage(s.language === 'auto' ? systemLanguage() : s.language);
      const root = this.doc.documentElement;
      root.style.setProperty('--editor-font-size', `${s.fontSize}px`);
      root.style.setProperty('--font-editor', this.fontFamily());
    });
    // only when these change – another setting must not undo a fresh F12
    effect(() => {
      const c = this.chromePart();
      untracked(() => {
        this.chrome.scheme.set(c.scheme);
        this.chrome.menuBar.set(c.menuBar);
        this.chrome.statusBar.set(c.statusBar);
      });
    });
    // chrome changes (F12, Strg+M …) go back into the settings
    effect(() => {
      const scheme = this.chrome.scheme();
      const menuBar = this.chrome.menuBar();
      const statusBar = this.chrome.statusBar();
      untracked(() => {
        const s = this.settings();
        if (s.scheme !== scheme || s.menuBar !== menuBar || s.statusBar !== statusBar) {
          this.update({ scheme, menuBar, statusBar });
        }
      });
    });
  }

  async load(): Promise<void> {
    try {
      const l = await this.api.load();
      this.settings.set({ ...DEFAULT_SETTINGS, ...l.value });
    } catch {
      // no backend: the defaults
    }
    this.loaded.set(true);
  }

  update(patch: Partial<Settings>): void {
    this.settings.update((s) => ({ ...s, ...patch }));
    if (!this.loaded()) return;
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => void this.flush(), SAVE_DELAY);
  }

  async flush(): Promise<void> {
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    try {
      const stored = await this.api.save(this.settings());
      // Rust may have clamped something
      if (JSON.stringify(stored) !== JSON.stringify(this.settings())) this.settings.set(stored);
    } catch {
      // not writable: the settings still apply for this run
    }
  }

  zoom(step: number): void {
    const size = step === 0 ? DEFAULT_SETTINGS.fontSize : this.settings().fontSize + step;
    this.update({ fontSize: Math.min(MAX_FONT, Math.max(MIN_FONT, size)) });
  }

  cycleLineNumbers(): void {
    const order: Settings['lineNumbers'][] = ['absolute', 'relative', 'off'];
    const i = order.indexOf(this.settings().lineNumbers);
    this.update({ lineNumbers: order[(i + 1) % order.length] });
  }
}
