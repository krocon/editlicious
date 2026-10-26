import { TestBed } from '@angular/core/testing';
import { SettingsApi } from '../../core/settings/settings-api';
import { DEFAULT_SETTINGS } from '../../core/settings/settings.models';
import { language, setLanguage } from '../../core/i18n/i18n';
import { FakeSettingsApi } from '../../../testing/fake-settings-api';
import { ChromeStore } from '../menus/chrome.store';
import { SettingsStore } from './settings.store';

function setup() {
  const api = new FakeSettingsApi();
  TestBed.configureTestingModule({ providers: [{ provide: SettingsApi, useValue: api }] });
  return { api, store: TestBed.inject(SettingsStore), chrome: TestBed.inject(ChromeStore) };
}

describe('SettingsStore', () => {
  afterEach(() => {
    setLanguage('en');
    vi.useRealTimers();
  });

  it('loads and applies language, font and chrome', async () => {
    const { api, store, chrome } = setup();
    api.stored = {
      ...DEFAULT_SETTINGS,
      language: 'de',
      fontSize: 18,
      fontFamily: 'Fira Code',
      scheme: 'night',
      statusBar: false,
    };
    await store.load();
    TestBed.tick();
    expect(language()).toBe('de');
    const root = document.documentElement.style;
    expect(root.getPropertyValue('--editor-font-size')).toBe('18px');
    expect(root.getPropertyValue('--font-editor')).toContain("'Fira Code'");
    expect(chrome.scheme()).toBe('night');
    expect(chrome.statusBar()).toBe(false);
  });

  it('saves changes bundled, chrome changes included', async () => {
    vi.useFakeTimers();
    const { api, store, chrome } = setup();
    await store.load();
    TestBed.tick();
    store.zoom(1);
    store.zoom(1);
    chrome.scheme.set('ink');
    TestBed.tick();
    expect(api.saves.length).toBe(0);
    await vi.advanceTimersByTimeAsync(500);
    expect(api.saves.length).toBe(1);
    expect(api.saves[0].fontSize).toBe(16);
    expect(api.saves[0].scheme).toBe('ink');
  });

  it('zoom stays in range and resets', async () => {
    const { store } = setup();
    await store.load();
    store.update({ fontSize: 71 });
    store.zoom(1);
    store.zoom(1);
    expect(store.settings().fontSize).toBe(72);
    store.zoom(0);
    expect(store.settings().fontSize).toBe(DEFAULT_SETTINGS.fontSize);
  });

  it('line numbers cycle absolute → relative → off', async () => {
    const { store } = setup();
    await store.load();
    store.cycleLineNumbers();
    expect(store.settings().lineNumbers).toBe('relative');
    store.cycleLineNumbers();
    expect(store.settings().lineNumbers).toBe('off');
    store.cycleLineNumbers();
    expect(store.settings().lineNumbers).toBe('absolute');
  });
});
