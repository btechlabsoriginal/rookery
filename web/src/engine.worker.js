import init, { analyse } from './wasm-bindings/rookery_wasm.js';

let ready;

self.onmessage = async ({ data }) => {
  const { id, fen, depth, play } = data;
  try {
    // Initialization is shared by jobs in the same worker.
    ready ??= init();
    await ready;
    const report = JSON.parse(analyse(fen, depth));
    self.postMessage({ id, fen, play, report });
  } catch (error) {
    self.postMessage({ id, fen, play, error: String(error) });
  }
};
