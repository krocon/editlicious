/// <reference lib="webworker" />
/**
 * The engine in a worker (design 025 E4): long work on big texts (counting, replace all,
 * sorting, `setText`) never blocks the page. First message: `{ init: { glueUrl, wasmUrl } }`.
 */
import { answer, createEngineHost, EngineHandler, EngineRequest, loadWasm } from './engine-host';

let handler: Promise<EngineHandler> | null = null;

addEventListener('message', (e: MessageEvent) => {
  const data = e.data as { init?: { glueUrl: string; wasmUrl: string } } & Partial<EngineRequest>;
  if (data.init) {
    const { glueUrl, wasmUrl } = data.init;
    handler = loadWasm(glueUrl, wasmUrl).then(createEngineHost);
    handler.then(
      () => postMessage({ ready: true }),
      (err: unknown) => postMessage({ ready: false, error: String(err) }),
    );
    return;
  }
  const req = data as EngineRequest;
  if (!handler) {
    postMessage({
      id: req.id,
      ok: false,
      error: { code: 'unknown', message: 'not initialised', path: null },
    });
    return;
  }
  void handler.then((h) => postMessage(answer(h, req)));
});
