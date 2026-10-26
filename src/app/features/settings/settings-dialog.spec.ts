import { TestBed } from '@angular/core/testing';
import { OS } from '../../core/actions/platform';
import { setLanguage } from '../../core/i18n/i18n';
import { SettingsApi } from '../../core/settings/settings-api';
import { FakeSettingsApi } from '../../../testing/fake-settings-api';
import { SettingsDialog } from './settings-dialog';
import { SettingsStore } from './settings.store';

describe('SettingsDialog', () => {
  afterEach(() => setLanguage('en'));

  it('changes apply at once – the dialog follows a language switch', async () => {
    TestBed.configureTestingModule({
      imports: [SettingsDialog],
      providers: [
        { provide: SettingsApi, useClass: FakeSettingsApi },
        { provide: OS, useValue: 'other' },
      ],
    });
    const store = TestBed.inject(SettingsStore);
    await store.load();
    const fixture = TestBed.createComponent(SettingsDialog);
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('h2')?.textContent).toBe('Settings');

    const lang = el.querySelector<HTMLSelectElement>('#set-lang')!;
    lang.value = 'de';
    lang.dispatchEvent(new Event('change'));
    await fixture.whenStable();
    expect(store.settings().language).toBe('de');
    expect(el.querySelector('h2')?.textContent).toBe('Einstellungen');

    const size = el.querySelector<HTMLInputElement>('#set-size')!;
    size.value = '20';
    size.dispatchEvent(new Event('change'));
    expect(store.settings().fontSize).toBe(20);

    const guides = [...el.querySelectorAll<HTMLInputElement>('input[type=checkbox]')].find((c) =>
      c.parentElement?.textContent?.includes('Einrückungslinien'),
    )!;
    guides.click();
    expect(store.settings().indentGuides).toBe(false);
  });
});
