#!/usr/bin/env node
/**
 * Build the MV3 extension without esbuild's helper process.
 *
 * TypeScript's compiler API transpiles each source file in-process. This is
 * intentionally dependency-free at runtime and keeps the extension bundle tiny.
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';

const here = path.dirname(fileURLToPath(import.meta.url));
const srcDir = path.join(here, 'src');
const files = ['background.ts', 'content.ts'];
let total = 0;

for (const file of files) {
  const input = path.join(srcDir, file);
  const output = path.join(here, file.replace(/\.ts$/, '.js'));
  const source = fs.readFileSync(input, 'utf8');
  const result = ts.transpileModule(source, {
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ESNext,
      isolatedModules: true,
      removeComments: false,
      sourceMap: false
    },
    fileName: file
  });
  fs.writeFileSync(output, result.outputText, 'utf8');
  total += Buffer.byteLength(result.outputText);
  console.log(`built ${path.relative(here, output)} · ${Buffer.byteLength(result.outputText)} bytes`);
}

if (total > 500 * 1024) {
  console.error('Extension JS exceeds the 500 KB budget.');
  process.exit(1);
}
console.log(`extension bundle ${(total / 1024).toFixed(1)} KB (under 500 KB)`);
