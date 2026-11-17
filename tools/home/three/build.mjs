import { build } from 'esbuild';
import { writeFile } from 'node:fs/promises';
const result = await build({ entryPoints: ['tools/home/three/kitchen.js'], bundle: true, format: 'esm', target: 'es2022', minify: true, legalComments: 'eof', write: false });
await writeFile('assets/home-kitchen.js', result.outputFiles[0].text.replace(/[\t ]+$/gm, ''));
