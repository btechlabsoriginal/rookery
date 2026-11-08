import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  // wasm-pack emits a JS module and a .wasm file in src/wasm-bindings.
  worker: { format: 'es' },
});
