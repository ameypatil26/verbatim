import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import ts from 'typescript';

const root = fileURLToPath(new URL('.', import.meta.url));

// The web build uses the same in-process TypeScript transform as the desktop
// app so it also works in restricted Windows environments where esbuild's
// helper process is unavailable.
function inProcessTypeScript() {
  return {
    name: 'verbatim-web-typescript',
    enforce: 'pre',
    transform(code, id) {
      if (!id.endsWith('.ts') || id.endsWith('.d.ts')) return null;
      const result = ts.transpileModule(code, {
        compilerOptions: {
          target: ts.ScriptTarget.ES2022,
          module: ts.ModuleKind.ESNext,
          isolatedModules: true,
          sourceMap: false
        },
        fileName: id
      });
      return { code: result.outputText, map: null };
    }
  };
}

export default defineConfig({
  root,
  base: './',
  publicDir: 'public',
  plugins: [inProcessTypeScript(), svelte()],
  esbuild: false,
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'esnext',
    minify: false,
    sourcemap: false,
    assetsDir: 'assets'
  }
});
