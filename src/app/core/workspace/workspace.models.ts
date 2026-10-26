/** Mirror of Rust `session_file.rs` (design 004 E5). */
export type TabsMode = 'auto' | 'show' | 'hide';

export interface SessionEntry {
  path: string;
  line: number;
  ch: number;
  top: number;
}

export interface SessionFile {
  version: number;
  files: SessionEntry[];
  active: number;
  recent: string[];
  tabs: TabsMode;
}
