/** Mirror of Rust `session_file.rs` (design 004 E5). */
export type TabsMode = 'auto' | 'show' | 'hide';

export interface SessionEntry {
  path: string;
  line: number;
  ch: number;
  top: number;
  /** Marked words by colour (design 019). */
  markers?: (string | null)[];
}

export interface SessionFile {
  version: number;
  files: SessionEntry[];
  active: number;
  recent: string[];
  tabs: TabsMode;
  findHistory: string[];
  replaceHistory: string[];
}

/** Mirror of Rust `path_input.rs` (prompt 024): one folder entry matching the typed path. */
export interface PathEntry {
  /** The name as shown (folders end with a separator). */
  label: string;
  /** The whole input text after choosing this entry. */
  value: string;
  dir: boolean;
}

/** Completions of a typed path (Rust `PathCompletion`). */
export interface PathCompletion {
  entries: PathEntry[];
  /** The input extended by what all matches share (Tab); `null` = nothing to add. */
  common: string | null;
  /** More entries matched than were sent. */
  more: boolean;
}
