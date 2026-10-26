import { translatable } from '../../core/i18n/i18n';

/** Texts of the About dialog and the start screen (design 009). */
const EN = {
  title: 'About editlicious',
  tagline: 'A calm text editor for huge files.',
  version: (v: string, os: string, arch: string) => `Version ${v} · ${os} / ${arch}`,
  noBackend: 'Version: desktop backend not available',
  copyright: '© 2026 Marc Kronberg · MIT License',
  close: 'Close',
  about: 'About',
  desktopOnly: 'The editor runs in the desktop app (pnpm tauri dev).',
};

export const ABOUT_TEXTS = translatable('about', EN, {
  de: {
    title: 'Über editlicious',
    tagline: 'Der ruhige Texteditor für riesige Dateien.',
    version: (v: string, os: string, arch: string) => `Version ${v} · ${os} / ${arch}`,
    noBackend: 'Version: Desktop-Backend nicht verfügbar',
    copyright: '© 2026 Marc Kronberg · MIT-Lizenz',
    close: 'Schließen',
    about: 'Über',
    desktopOnly: 'Der Editor läuft in der Desktop-App (pnpm tauri dev).',
  },
});
