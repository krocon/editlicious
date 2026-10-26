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
