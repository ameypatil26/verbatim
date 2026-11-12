import ts from 'typescript';

/**
 * TypeScript preprocessing for the landing page.
 *
 * `verbatimModuleSyntax` is essential: Svelte component imports are used in
 * markup, so TypeScript must not elide them as "unused" when it strips types
 * from the <script> block.
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
          verbatimModuleSyntax: true,
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
