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

/** User-facing text for an error (English; German follows in prompt 009). */
export function describeFsError(error: FsError): string {
  switch (error.code) {
    case 'not_found':
      return 'The file does not exist.';
    case 'permission_denied':
      return 'Access denied.';
    case 'not_a_directory':
      return 'This is not a folder.';
    case 'is_a_directory':
      return 'This is a folder, not a file.';
    case 'invalid_path':
      return 'The path is not valid.';
    case 'io':
      return 'The file could not be read or written.';
    case 'backend_unavailable':
      return 'The desktop backend is not available (running in a browser?).';
    case 'unknown':
      return 'Something went wrong.';
  }
}
