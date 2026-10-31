import { answer, createEngineHost, EngineHandler, EngineResponse, loadWasm } from './engine-host';

/** Options of {@link initEditlicious}. */
export interface EditliciousInit {
  /** Where `ed_wasm_bg.wasm` is; default: next to this bundle. */
  wasmUrl?: string | URL;
  /** Where the binding's glue code `ed_wasm.js` is; default: next to this bundle. */
  glueUrl?: string | URL;
  /**
   * Run the engine in a web worker (default) – big texts never block the page. `false`: on the
   * main thread (a CSP without `worker-src`, tests).
   */
  worker?: boolean;
}

/** Sends one request to the engine and waits for its answer. */
export type EngineCall = (op: string, ...args: unknown[]) => Promise<unknown>;

let started: { options: EditliciousInit; call: Promise<EngineCall> } | null = null;

/**
 * Loads the engine once for the whole page (design 025 E5): every editor shares the one
 * WebAssembly module; later calls get the same promise.
 */
export function initEditlicious(options: EditliciousInit = {}): Promise<void> {
  return engine(options).then(() => undefined);
}

/** The shared engine (starts it with `options` on first use). */
export function engine(options: EditliciousInit = {}): Promise<EngineCall> {
  if (started) {
    if (Object.keys(options).length && options !== started.options) {
      console.warn('editlicious: the engine is already started – new options are ignored');
    }
    return started.call;
  }
  const base = import.meta.url;
  const wasmUrl = String(options.wasmUrl ?? new URL('./ed_wasm_bg.wasm', base));
  const glueUrl = String(options.glueUrl ?? new URL('./ed_wasm.js', base));
  const useWorker = options.worker ?? typeof Worker !== 'undefined';
  const call = useWorker ? startWorker(glueUrl, wasmUrl) : startInline(glueUrl, wasmUrl);
  started = { options, call };
  return call;
}

/** Tests: forget the started engine, or use a handler of their own. */
export function resetEngine(handler?: EngineHandler): void {
  started = handler ? { options: {}, call: Promise.resolve(inlineCall(handler)) } : null;
}

function inlineCall(handler: EngineHandler): EngineCall {
  return (op, ...args) => {
    const r = answer(handler, { id: 0, op, args });
    return r.ok ? Promise.resolve(r.value) : Promise.reject(r.error);
  };
}

async function startInline(glueUrl: string, wasmUrl: string): Promise<EngineCall> {
  const mod = await loadWasm(glueUrl, wasmUrl);
  return inlineCall(createEngineHost(mod));
}

function startWorker(glueUrl: string, wasmUrl: string): Promise<EngineCall> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL('./engine.worker', import.meta.url), { type: 'module' });
    const pending = new Map<number, { ok: (v: unknown) => void; fail: (e: unknown) => void }>();
    let next = 1;
    const call: EngineCall = (op, ...args) =>
      new Promise((ok, fail) => {
        const id = next++;
        pending.set(id, { ok, fail });
        worker.postMessage({ id, op, args });
      });
    worker.onmessage = (e: MessageEvent) => {
      const d = e.data as EngineResponse | { ready: boolean; error?: string };
      if ('ready' in d) {
        if (d.ready) resolve(call);
        else reject(new Error(d.error));
        return;
      }
      const p = pending.get(d.id);
      pending.delete(d.id);
      if (!p) return;
      if (d.ok) p.ok(d.value);
      else p.fail(d.error);
    };
    worker.onerror = (e) => reject(new Error(e.message));
    worker.postMessage({ init: { glueUrl, wasmUrl } });
  });
}
