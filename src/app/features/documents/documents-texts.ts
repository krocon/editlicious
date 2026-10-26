import { translatable } from '../../core/i18n/i18n';

/** Texts of files and tabs (English = source, German below; design 009). */
const EN = {
  untitled: 'Untitled',
  appName: 'editlicious',
  tabs: 'Open files',
  close: 'Close',
  unsavedTitle: 'Unsaved changes',
  unsaved: (name: string) => `Save the changes to “${name}” before closing?`,
  save: 'Save',
  dontSave: 'Don’t save',
  cancel: 'Cancel',
  unmappableTitle: 'Character cannot be saved',
  unmappable: (name: string, encoding: string, where: string) =>
    `“${name}” contains a character (line ${where}) that ${encoding} cannot store. Save as UTF-8 instead?`,
  saveUtf8: 'Save as UTF-8',
  changedTitle: 'File changed on disk',
  changed: (name: string) =>
    `“${name}” was changed by another program. Reload it and lose your unsaved changes?`,
  reload: 'Reload',
  keepMine: 'Keep my version',
  reloaded: (name: string) => `“${name}” was changed on disk and reloaded.`,
  missing: (name: string) => `“${name}” no longer exists on disk.`,
  opening: (name: string) => `Opening ${name}…`,
  openFailed: (name: string, why: string) => `“${name}” could not be opened: ${why}`,
  saveFailed: (name: string, why: string) => `“${name}” could not be saved: ${why}`,
  saved: (name: string) => `Saved ${name}.`,
  encodingTitle: (enc: string) => `Encoding ${enc}`,
  encodingQuestion: (name: string, enc: string) =>
    `Read “${name}” again as ${enc} (if it shows wrong characters), or save it in ${enc} (convert)?`,
  encodingDirty: (name: string, enc: string) =>
    `Read “${name}” again as ${enc} – your unsaved changes are lost –, or save it in ${enc} (convert)?`,
  reopenWith: 'Read again',
  convertTo: 'Save converted',
  reopened: (name: string, enc: string) => `“${name}” read again as ${enc}.`,
  overwrite: (name: string) =>
    `“${name}” was changed by another program since you opened it. Overwrite those changes with yours?`,
  overwriteButton: 'Overwrite',
  lossyTitle: 'Unreadable characters',
  lossy: (name: string, enc: string) =>
    `Some bytes of “${name}” are not valid ${enc} and are shown as “�”. Saving writes “�” in their place – the original bytes are lost. Try “Encoding…” to read the file in another encoding first.`,
  saveAnyway: 'Save anyway',
};

export const DOC_TEXTS = translatable('documents', EN, {
  de: {
    untitled: 'Unbenannt',
    appName: 'editlicious',
    tabs: 'Offene Dateien',
    close: 'Schließen',
    unsavedTitle: 'Ungespeicherte Änderungen',
    unsaved: (name: string) => `Änderungen an „${name}“ vor dem Schließen speichern?`,
    save: 'Speichern',
    dontSave: 'Nicht speichern',
    cancel: 'Abbrechen',
    unmappableTitle: 'Zeichen kann nicht gespeichert werden',
    unmappable: (name: string, encoding: string, where: string) =>
      `„${name}“ enthält ein Zeichen (Zeile ${where}), das ${encoding} nicht speichern kann. Stattdessen als UTF-8 speichern?`,
    saveUtf8: 'Als UTF-8 speichern',
    changedTitle: 'Datei wurde geändert',
    changed: (name: string) =>
      `„${name}“ wurde von einem anderen Programm geändert. Neu laden und die ungespeicherten Änderungen verwerfen?`,
    reload: 'Neu laden',
    keepMine: 'Meine Fassung behalten',
    reloaded: (name: string) => `„${name}“ wurde geändert und neu geladen.`,
    missing: (name: string) => `„${name}“ gibt es nicht mehr.`,
    opening: (name: string) => `${name} wird geöffnet …`,
    openFailed: (name: string, why: string) => `„${name}“ konnte nicht geöffnet werden: ${why}`,
    saveFailed: (name: string, why: string) => `„${name}“ konnte nicht gespeichert werden: ${why}`,
    saved: (name: string) => `${name} gespeichert.`,
    encodingTitle: (enc: string) => `Zeichenkodierung ${enc}`,
    encodingQuestion: (name: string, enc: string) =>
      `„${name}“ neu als ${enc} lesen (wenn Zeichen falsch aussehen) oder in ${enc} speichern (umwandeln)?`,
    encodingDirty: (name: string, enc: string) =>
      `„${name}“ neu als ${enc} lesen – die ungespeicherten Änderungen gehen verloren – oder in ${enc} speichern (umwandeln)?`,
    reopenWith: 'Neu lesen',
    convertTo: 'Umgewandelt speichern',
    reopened: (name: string, enc: string) => `„${name}“ neu als ${enc} gelesen.`,
    overwrite: (name: string) =>
      `„${name}“ wurde seit dem Öffnen von einem anderen Programm geändert. Diese Änderungen mit deinen überschreiben?`,
    overwriteButton: 'Überschreiben',
    lossyTitle: 'Unlesbare Zeichen',
    lossy: (name: string, enc: string) =>
      `Einige Bytes von „${name}“ sind kein gültiges ${enc} und werden als „�“ gezeigt. Beim Speichern steht dort „�“ – die ursprünglichen Bytes gehen verloren. Mit „Zeichenkodierung …“ lässt sich die Datei vorher in einer anderen Kodierung lesen.`,
    saveAnyway: 'Trotzdem speichern',
  },
});
