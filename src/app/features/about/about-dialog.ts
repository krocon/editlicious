import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { AboutApi } from '../../core/about/about-api';
import { AppInfo } from '../../core/about/about.models';
import { Modal } from '../../shared/dialog/modal';
import { DialogService } from '../../shared/dialog/dialog.service';

/** About: wordmark, version and platform from the backend, link to the repository. */
@Component({
  selector: 'app-about-dialog',
  imports: [Modal],
  template: `
    <app-modal heading="About editlicious" width="26rem" (closed)="dialogs.close()">
      <div class="about">
        <img class="logo" src="logo.svg" alt="" />
        <p class="tagline">A calm text editor for huge files.</p>
        @if (info(); as i) {
          <p class="meta">Version {{ i.version }} · {{ i.os }} / {{ i.arch }}</p>
        } @else {
          <p class="meta">Version: desktop backend not available</p>
        }
        <p class="meta">© 2026 Marc Kronberg · MIT License</p>
      </div>
      <div class="actions">
        <button type="button" class="btn primary" data-autofocus (click)="dialogs.close()">
          Close
        </button>
      </div>
    </app-modal>
  `,
  styles: `
    .about {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: var(--space-1);
      text-align: center;
    }
    .logo {
      width: 220px;
      height: auto;
      margin: var(--space-2) 0;
      filter: var(--logo-filter, none);
    }
    :host-context([data-theme='dark']) .logo {
      filter: invert(1);
    }
    @media (prefers-color-scheme: dark) {
      :host-context(:root:not([data-theme='light'])) .logo {
        filter: invert(1);
      }
    }
    .tagline {
      margin: 0 0 var(--space-2);
    }
    .meta {
      margin: 0;
      font-size: var(--font-size-small);
      color: var(--color-text-muted);
    }
    .actions {
      display: flex;
      justify-content: flex-end;
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AboutDialog {
  protected readonly dialogs = inject(DialogService);
  protected readonly info = signal<AppInfo | null>(null);

  constructor() {
    inject(AboutApi)
      .appInfo()
      .then((i) => this.info.set(i))
      .catch(() => this.info.set(null));
  }
}
