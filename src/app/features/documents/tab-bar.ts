import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  inject,
  output,
  signal,
} from '@angular/core';
import { DocumentsStore } from './documents.store';
import { DOC_TEXTS } from './documents-texts';

/**
 * Quiet tabs of the open files (design 004 E2, legacy: auto/show/hide). Tab list after APG:
 * arrows move between tabs, Enter/Space activates, Delete closes; drag to reorder, middle
 * click closes. A dot marks unsaved changes.
 */
@Component({
  selector: 'app-tab-bar',
  template: `
    <div class="tabs" role="tablist" [attr.aria-label]="t.tabs">
      @for (d of store.docs(); track d.id; let i = $index) {
        <div
          class="tab"
          role="tab"
          [class.active]="d.id === store.activeId()"
          [class.dragover]="dragOver() === i"
          [attr.aria-selected]="d.id === store.activeId()"
          [attr.tabindex]="d.id === store.activeId() ? 0 : -1"
          [attr.title]="d.info.path ?? d.info.name"
          [attr.data-id]="d.id"
          draggable="true"
          (keydown)="onKeydown($event)"
          (mousedown)="onMousedown($event, d.id)"
          (auxclick)="onAux($event, d.id)"
          (dragstart)="onDragStart($event, d.id)"
          (dragover)="onDragOver($event, i)"
          (dragleave)="dragOver.set(null)"
          (drop)="onDrop($event, i)"
        >
          <span class="name">{{ d.info.name }}</span>
          <button
            type="button"
            class="close"
            [class.dirty]="d.info.dirty"
            tabindex="-1"
            [attr.aria-label]="t.close + ' ' + d.info.name"
            (mousedown)="$event.stopPropagation()"
            (click)="closeTab.emit(d.id)"
          ></button>
        </div>
      }
    </div>
  `,
  styleUrl: './tab-bar.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TabBar {
  protected readonly store = inject(DocumentsStore);
  protected readonly t = DOC_TEXTS;
  protected readonly dragOver = signal<number | null>(null);
  /** Close asks about unsaved changes – the app shell decides. */
  readonly closeTab = output<number>();
  /** After a tab was chosen the editor gets the focus back. */
  readonly chosen = output<void>();
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  protected onMousedown(e: MouseEvent, id: number): void {
    if (e.button !== 0) return;
    this.store.activate(id);
    this.chosen.emit();
  }

  protected onAux(e: MouseEvent, id: number): void {
    if (e.button === 1) {
      e.preventDefault();
      this.closeTab.emit(id);
    }
  }

  protected onDragStart(e: DragEvent, id: number): void {
    e.dataTransfer?.setData('application/x-editlicious-tab', String(id));
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }

  protected onDragOver(e: DragEvent, i: number): void {
    if (!e.dataTransfer?.types.includes('application/x-editlicious-tab')) return;
    e.preventDefault();
    this.dragOver.set(i);
  }

  protected onDrop(e: DragEvent, i: number): void {
    const id = Number(e.dataTransfer?.getData('application/x-editlicious-tab'));
    this.dragOver.set(null);
    if (!id) return;
    e.preventDefault();
    this.store.move(id, i);
  }

  protected onKeydown(e: KeyboardEvent): void {
    const docs = this.store.docs();
    const i = docs.findIndex((d) => d.id === this.store.activeId());
    let next: number;
    switch (e.key) {
      case 'ArrowLeft':
        next = (i - 1 + docs.length) % docs.length;
        break;
      case 'ArrowRight':
        next = (i + 1) % docs.length;
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = docs.length - 1;
        break;
      case 'Delete':
        e.preventDefault();
        this.closeTab.emit(docs[i].id);
        return;
      case 'Enter':
      case ' ':
        e.preventDefault();
        this.chosen.emit();
        return;
      default:
        return;
    }
    e.preventDefault();
    this.store.activate(docs[next].id);
    setTimeout(() => this.host.nativeElement.querySelector<HTMLElement>('.tab.active')?.focus());
  }
}
