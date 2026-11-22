import { build } from 'esbuild';
import { writeFile } from 'node:fs/promises';
const outfile = 'assets/about-three-tour.js';
const result = await build({ entryPoints: ['tools/about/three/tour.js'], bundle: true, format: 'esm', target: 'es2022', minify: true, legalComments: 'eof', outfile, write: false });
// Three's embedded GLSL includes trailing whitespace; keep generated diffs clean.
await writeFile(outfile, result.outputFiles[0].text.replace(/[\t ]+$/gm, ''));

const village=await build({entryPoints:['tools/about/three/village.js'],bundle:true,format:'esm',target:'es2022',minify:true,legalComments:'eof',write:false});
await writeFile('assets/about-three-village.js',village.outputFiles[0].text.replace(/[\t ]+$/gm,''));
