import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { ActionService } from '../../core/actions/action.service';
import { Settings } from '../../core/settings/settings.models';
import { DialogService } from '../../shared/dialog/dialog.service';
import { Modal } from '../../shared/dialog/modal';
import { SCHEMES } from '../menus/chrome.store';
import { SettingsStore } from './settings.store';
import { SETTINGS_TEXTS } from './settings-texts';

/** The settings dialog (Mod+,): every change applies at once and is saved (design 009). */
@Component({
  selector: 'app-settings-dialog',
  imports: [Modal],
  templateUrl: './settings-dialog.html',
  styleUrl: './settings-dialog.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SettingsDialog {
  protected readonly dialogs = inject(DialogService);
  protected readonly store = inject(SettingsStore);
  protected readonly t = SETTINGS_TEXTS;
  protected readonly mac = inject(ActionService).os === 'mac';
  protected readonly schemes = SCHEMES;
  private readonly actions = inject(ActionService);

  protected schemeLabel(s: string): string {
    const l = this.actions.label(`view.scheme.${s}`).replace(/^[^:]+:\s*/, '');
    return l.charAt(0).toUpperCase() + l.slice(1);
  }

  protected set<K extends keyof Settings>(key: K, value: Settings[K]): void {
    this.store.update({ [key]: value } as Partial<Settings>);
  }

  protected value(e: Event): string {
    return (e.target as HTMLInputElement | HTMLSelectElement).value;
  }

  protected checked(e: Event): boolean {
    return (e.target as HTMLInputElement).checked;
  }

  protected number(e: Event, fallback: number): number {
    const n = Number((e.target as HTMLInputElement).value);
    return Number.isFinite(n) && n > 0 ? Math.round(n) : fallback;
  }
}
