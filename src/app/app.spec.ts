import { TestBed } from '@angular/core/testing';
import { App } from './app';
import { AboutApi } from './core/about/about-api';
import { FakeAboutApi } from '../testing/fake-about-api';

describe('App', () => {
  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [App],
      providers: [{ provide: AboutApi, useClass: FakeAboutApi }],
    });
  });

  it('opens the About dialog with the version from the backend', async () => {
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    el.querySelector<HTMLButtonElement>('button.about')!.click();
    await fixture.whenStable();
    expect(el.querySelector('[role="dialog"]')?.textContent).toContain('0.0.0-test');
  });
});
