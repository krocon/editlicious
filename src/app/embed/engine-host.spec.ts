import { answer, createEngineHost, engineError, WasmEditor, WasmModule } from './engine-host';

class FakeEditor implements WasmEditor {
  static freed = 0;
  lang: string | undefined;
  constructor(
    public body: string,
    lang?: string | null,
  ) {
    this.lang = lang ?? undefined;
  }
  command(cmd: string): string {
    if (cmd.includes('"type"'))
      throw JSON.stringify({ code: 'read_only', message: 'ro', path: null });
    return `{"frame":${cmd}}`;
  }
  setLanguage(spec: string): void {
    this.lang = spec || undefined;
  }
  language(): string | undefined {
    return this.lang;
  }
  info(): string {
    return '{}';
  }
  setText(text: string): void {
    this.body = text;
  }
  clipboard(): string | undefined {
    return 'c';
  }
  clipboardHtml(): string | undefined {
    return undefined;
  }
  setClipboard(): void {
    // nothing to keep
  }
  text(): string {
    return this.body;
  }
  slice(from: number, to: number): string {
    return this.body.split('\n').slice(from, to).join('\n');
  }
  countMatches(): string {
    return '{"total":1,"current":1}';
  }
  countMarker(): number | undefined {
    return undefined;
  }
  overview(): Uint8Array {
    return new Uint8Array([0, 1]);
  }
  free(): void {
    FakeEditor.freed++;
  }
}

const mod: WasmModule = {
  default: () => Promise.resolve(),
  Editor: FakeEditor,
  languages: () => '["Rust"]',
  encodings: () => '["UTF-8"]',
};

describe('engine host (design 025 E4)', () => {
  it('opens sessions with their own ids and runs ops on them', () => {
    const h = createEngineHost(mod);
    const a = h('open', ['one', 'Rust']) as number;
    const b = h('open', ['two', null]) as number;
    expect(a).not.toBe(b);
    expect(h('text', [a])).toBe('one');
    h('setText', [b, 'x\ny']);
    expect(h('slice', [b, 1, 2])).toBe('y');
    expect(h('setLanguage', [b, 'TypeScript'])).toBe('TypeScript');
    expect(h('overview', [a, 2])).toEqual([0, 1]);
    expect(h('countMarker', [a])).toBeNull();
    expect(h('clipboard', [a])).toEqual({ text: 'c', html: null });
    expect(h('languages', [])).toBe('["Rust"]');
    h('close', [a]);
    expect(FakeEditor.freed).toBe(1);
  });

  it('answers errors as FsError objects', () => {
    const h = createEngineHost(mod);
    const a = h('open', ['', null]) as number;
    expect(answer(h, { id: 1, op: 'command', args: [a, '{"type":"type"}'] })).toEqual({
      id: 1,
      ok: false,
      error: { code: 'read_only', message: 'ro', path: null },
    });
    expect(answer(h, { id: 2, op: 'text', args: [99] })).toMatchObject({
      ok: false,
      error: { code: 'unknown_document' },
    });
    expect(answer(h, { id: 3, op: 'nope', args: [] })).toMatchObject({
      ok: false,
      error: { code: 'invalid_request' },
    });
    expect(engineError(new Error('boom'))).toMatchObject({ code: 'unknown' });
  });
});
