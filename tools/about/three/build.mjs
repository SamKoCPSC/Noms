import { build } from 'esbuild';
import { writeFile } from 'node:fs/promises';
const outfile = 'assets/about-three-kitchen.js';
const result = await build({ entryPoints: ['tools/about/three/kitchen.js'], bundle: true, format: 'esm', target: 'es2022', minify: true, legalComments: 'eof', outfile, write: false });
// Three's embedded GLSL includes trailing whitespace; keep generated diffs clean.
await writeFile(outfile, result.outputFiles[0].text.replace(/[\t ]+$/gm, ''));
