import ts from 'typescript';

/**
 * Svelte 5 preprocessing without invoking Vite's esbuild helper process.
 * Restricted Windows environments block piped child processes, so TypeScript
 * is stripped in-process here. Plain CSS needs no preprocessing.
 */
function typescriptPreprocess() {
  return {
    script: ({ content, attributes }) => {
      if (attributes.lang !== 'ts' && attributes.lang !== 'typescript') return undefined;
      const result = ts.transpileModule(content, {
        compilerOptions: {
          target: ts.ScriptTarget.ES2022,
          module: ts.ModuleKind.ESNext,
          isolatedModules: true,
          sourceMap: false
        },
        fileName: 'component.ts'
      });
      return { code: result.outputText, map: null };
    }
  };
}

export default {
  preprocess: typescriptPreprocess()
};
