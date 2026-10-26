import { translatable } from '../i18n/i18n';

/**
 * Errors from the backend: `FsError { code, message, path }` (Rust `error.rs`). New codes go
 * into both files and into `describeFsError()`. The UI never shows Rust's `message`.
 */
export type FsErrorCode =
  | 'not_found'
  | 'permission_denied'
  | 'not_a_directory'
  | 'is_a_directory'
  | 'invalid_path'
  | 'io'
  | 'unknown_document'
  | 'binary_file'
  | 'file_too_large'
  | 'unsupported_encoding'
  | 'unmappable_character'
  | 'selection_too_large'
  | 'invalid_edit'
  | 'cancelled'
  | 'invalid_menu'
  | 'invalid_search'
  /** Frontend only: no Tauri backend (e.g. `pnpm start` in a browser). */
  | 'backend_unavailable'
  /** Frontend only: something that is no `FsError` was thrown. */
  | 'unknown';

export interface FsError {
  code: FsErrorCode;
  message: string;
  path?: string | null;
}

const KNOWN: readonly FsErrorCode[] = [
  'not_found',
  'permission_denied',
  'not_a_directory',
  'is_a_directory',
  'invalid_path',
  'io',
  'unknown_document',
  'binary_file',
  'file_too_large',
  'unsupported_encoding',
  'unmappable_character',
  'selection_too_large',
  'invalid_edit',
  'cancelled',
  'invalid_menu',
  'invalid_search',
  'backend_unavailable',
  'unknown',
];

export function isFsError(value: unknown): value is FsError {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Partial<FsError>;
  return typeof v.code === 'string' && typeof v.message === 'string';
}

/** Whatever was thrown → an `FsError` (unknown codes become `unknown`). */
export function toFsError(value: unknown): FsError {
  if (isFsError(value)) {
    return KNOWN.includes(value.code) ? value : { ...value, code: 'unknown' };
  }
  const message = value instanceof Error ? value.message : String(value);
  return { code: 'unknown', message };
}

const EN: Record<FsErrorCode, string> = {
  not_found: 'The file does not exist.',
  permission_denied: 'Access denied.',
  not_a_directory: 'This is not a folder.',
  is_a_directory: 'This is a folder, not a file.',
  invalid_path: 'The path is not valid.',
  io: 'The file could not be read or written.',
  unknown_document: 'This file is no longer open.',
  binary_file: 'This is not a text file.',
  file_too_large: 'The file is too large (more than 4 GB).',
  unsupported_encoding: 'This encoding is not supported.',
  unmappable_character: 'A character cannot be stored in this encoding.',
  selection_too_large: 'The selection is too large for this operation.',
  invalid_edit: 'The change could not be applied.',
  cancelled: 'Cancelled.',
  invalid_menu: 'The menu could not be built.',
  invalid_search: 'The search pattern is not valid.',
  backend_unavailable: 'The desktop backend is not available (running in a browser?).',
  unknown: 'Something went wrong.',
};

const TEXTS = translatable('fsErrors', EN, {
  de: {
    not_found: 'Die Datei gibt es nicht.',
    permission_denied: 'Zugriff verweigert.',
    not_a_directory: 'Das ist kein Ordner.',
    is_a_directory: 'Das ist ein Ordner, keine Datei.',
    invalid_path: 'Der Pfad ist ungültig.',
    io: 'Die Datei konnte nicht gelesen oder geschrieben werden.',
    unknown_document: 'Diese Datei ist nicht mehr geöffnet.',
    binary_file: 'Das ist keine Textdatei.',
    file_too_large: 'Die Datei ist zu groß (mehr als 4 GB).',
    unsupported_encoding: 'Diese Kodierung wird nicht unterstützt.',
    unmappable_character: 'Ein Zeichen lässt sich in dieser Kodierung nicht speichern.',
    selection_too_large: 'Die Auswahl ist für diese Aktion zu groß.',
    invalid_edit: 'Die Änderung konnte nicht ausgeführt werden.',
    cancelled: 'Abgebrochen.',
    invalid_menu: 'Das Menü konnte nicht aufgebaut werden.',
    invalid_search: 'Das Suchmuster ist ungültig.',
    backend_unavailable: 'Das Desktop-Backend ist nicht verfügbar (läuft im Browser?).',
    unknown: 'Etwas ist schiefgegangen.',
  },
});

/** User-facing text for an error, in the UI language (design 009). */
export function describeFsError(error: FsError): string {
  return TEXTS[error.code] ?? TEXTS.unknown;
}
