import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import ts from 'typescript';

// Tauri expects a fixed port. `esbuild: false` keeps the build usable in
// restricted Windows environments that disallow esbuild's helper process.
// TypeScript is therefore stripped in-process by this small plugin.
function inProcessTypeScript() {
  return {
    name: 'verbatim-in-process-typescript',
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
  plugins: [inProcessTypeScript(), svelte()],
  clearScreen: false,
  esbuild: false,
  server: {
    port: 1420,
    strictPort: true,
    host: '127.0.0.1',
    watch: {
      ignored: ['**/src-tauri/**', '**/services/**', '**/mcp/**', '**/web/**', '**/extension/**']
    }
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'esnext',
    minify: false,
    sourcemap: false
  }
});
