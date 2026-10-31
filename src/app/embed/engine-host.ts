/**
 * The engine side of the embedded editor (design 025 E4): one WebAssembly module, all
 * sessions of the page. A plain function over the module – the worker
 * (`engine.worker.ts`) and the main-thread fallback run the same code; tests give a fake module.
 * Frames and infos stay JSON strings here: the page parses them (cheap to post).
 */

/** What the WebAssembly binding (`ed-wasm`, built to `ed_wasm.js`) offers – see its `lib.rs`. */
export interface WasmEditor {
  command(cmd: string): string;
  setLanguage(spec: string): void;
  language(): string | undefined;
  info(): string;
  setText(text: string): void;
  clipboard(): string | undefined;
  clipboardHtml(): string | undefined;
  setClipboard(text: string): void;
  text(): string;
  slice(from: number, to: number): string;
  countMatches(query: string): string;
  countMarker(): number | undefined;
  overview(buckets: number): Uint8Array;
  free(): void;
}

export interface WasmModule {
  default(input?: { module_or_path: string | URL } | string | URL): Promise<unknown>;
  Editor: new (text: string, language?: string | null) => WasmEditor;
  languages(): string;
  encodings(): string;
}

/** One request to the engine. */
export interface EngineRequest {
  id: number;
  op: string;
  args: unknown[];
}

/** Its answer: a value, or an error (an `FsError` object when the engine sent one). */
export type EngineResponse =
  { id: number; ok: true; value: unknown } | { id: number; ok: false; error: unknown };

/** Turns what the binding throws (`FsError` as JSON, or anything) into a plain object. */
export function engineError(e: unknown): unknown {
  if (typeof e === 'string') {
    try {
      const v = JSON.parse(e) as unknown;
      if (v && typeof v === 'object' && 'code' in v) return v;
    } catch {
      // not JSON
    }
    return { code: 'unknown', message: e, path: null };
  }
  return { code: 'unknown', message: String(e), path: null };
}

export type EngineHandler = (op: string, args: unknown[]) => unknown;

/** The engine operations over a loaded module (the protocol of design 025 „Verträge“). */
export function createEngineHost(mod: WasmModule): EngineHandler {
  const sessions = new Map<number, WasmEditor>();
  let next = 1;
  const get = (id: unknown): WasmEditor => {
    const s = sessions.get(id as number);
    if (!s)
      throw JSON.stringify({ code: 'unknown_document', message: `no session ${id}`, path: null });
    return s;
  };
  return (op, a) => {
    switch (op) {
      case 'open': {
        const id = next++;
        sessions.set(id, new mod.Editor(String(a[0] ?? ''), (a[1] as string | null) ?? null));
        return id;
      }
      case 'close': {
        sessions.get(a[0] as number)?.free();
        sessions.delete(a[0] as number);
        return null;
      }
      case 'command':
        return get(a[0]).command(a[1] as string);
      case 'info':
        return get(a[0]).info();
      case 'setText':
        get(a[0]).setText(a[1] as string);
        return null;
      case 'setLanguage':
        get(a[0]).setLanguage(String(a[1] ?? ''));
        return get(a[0]).language() ?? null;
      case 'text':
        return get(a[0]).text();
      case 'slice':
        return get(a[0]).slice(a[1] as number, a[2] as number);
      case 'countMatches':
        return get(a[0]).countMatches(a[1] as string);
      case 'countMarker':
        return get(a[0]).countMarker() ?? null;
      case 'overview':
        return Array.from(get(a[0]).overview(a[1] as number));
      case 'clipboard': {
        const s = get(a[0]);
        return { text: s.clipboard() ?? null, html: s.clipboardHtml() ?? null };
      }
      case 'setClipboard':
        get(a[0]).setClipboard(a[1] as string);
        return null;
      case 'languages':
        return mod.languages();
      case 'encodings':
        return mod.encodings();
      default:
        throw JSON.stringify({ code: 'invalid_request', message: `unknown op ${op}`, path: null });
    }
  };
}

/** Runs one request against a handler – errors become `{ ok: false }` answers. */
export function answer(handler: EngineHandler, req: EngineRequest): EngineResponse {
  try {
    return { id: req.id, ok: true, value: handler(req.op, req.args) };
  } catch (e) {
    return { id: req.id, ok: false, error: engineError(e) };
  }
}

/** Loads the binding's glue code and the module (`ed_wasm.js` + `ed_wasm_bg.wasm`). */
export async function loadWasm(glueUrl: string, wasmUrl: string): Promise<WasmModule> {
  const mod = (await import(/* @vite-ignore */ glueUrl)) as WasmModule;
  await mod.default({ module_or_path: wasmUrl });
  return mod;
}
