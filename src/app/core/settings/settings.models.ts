/** Mirror of Rust `settings::Settings` (design 009). */
export interface Settings {
  version: number;
  language: 'auto' | 'en' | 'de';
  scheme: 'auto' | 'paper' | 'ink' | 'sand' | 'night';
  fontFamily: string;
  fontSize: number;
  tabWidth: number;
  indentWith: 'auto' | 'spaces' | 'tabs';
  lineNumbers: 'absolute' | 'relative' | 'off';
  indentGuides: boolean;
  highlightCurrentLine: boolean;
  showTabs: boolean;
  newLineEnding: 'auto' | 'lf' | 'crlf';
  restoreSession: boolean;
  menuBar: boolean;
  statusBar: boolean;
  wordWrap: boolean;
  overview: boolean;
  /** Colour values as swatches in the gutter (design 018). */
  colorSwatches: boolean;
  /** Word boundaries of repeated double clicks (prompt 018a); empty = built-in. */
  expandWeights: Record<string, number>;
  /** An AI may drive the editor over MCP (design 016 E9). */
  mcp: boolean;
  /** ffmpeg for the video export (design 017); empty = the one on the PATH. */
  ffmpegPath: string;
  /** Results of arithmetic below the expression (prompt 020). */
  mathHints: boolean;
  /** Results as suggestions after `=` or Ctrl+Space. */
  mathSuggest: boolean;
  /** Fixed decimal places; -1 = as needed. */
  mathDecimals: number;
  /** `auto` = by the UI language. */
  decimalSeparator: 'auto' | 'point' | 'comma';
}

export const DEFAULT_SETTINGS: Settings = {
  version: 1,
  language: 'auto',
  scheme: 'auto',
  fontFamily: '',
  fontSize: 14,
  tabWidth: 4,
  indentWith: 'auto',
  lineNumbers: 'absolute',
  indentGuides: true,
  highlightCurrentLine: true,
  showTabs: true,
  newLineEnding: 'auto',
  restoreSession: true,
  menuBar: false,
  statusBar: true,
  wordWrap: false,
  overview: true,
  colorSwatches: true,
  expandWeights: {},
  mcp: false,
  ffmpegPath: '',
  mathHints: true,
  mathSuggest: true,
  mathDecimals: -1,
  decimalSeparator: 'auto',
};

export interface LoadedSettings {
  value: Settings;
  warning: { code: string; path: string | null } | null;
}
