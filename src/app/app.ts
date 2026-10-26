import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { AboutDialog } from './features/about/about-dialog';
import { DialogService } from './shared/dialog/dialog.service';

/**
 * Root: the editor area (prompt 003 onwards) and the one modal dialog of the moment.
 * Nothing but the text should catch the eye (prompt 000-start: "wenig Ablenkung").
 */
@Component({
  selector: 'app-root',
  imports: [AboutDialog],
  template: `
    <main class="welcome">
      <img class="logo" src="logo.svg" alt="editlicious" />
      <p class="hint">A calm text editor for huge files.</p>
      <button type="button" class="btn quiet about" (click)="dialogs.show('about')">About</button>
    </main>
    @switch (dialogs.open()) {
      @case ('about') {
        <app-about-dialog />
      }
    }
  `,
  styleUrl: './app.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class App {
  protected readonly dialogs = inject(DialogService);
}
