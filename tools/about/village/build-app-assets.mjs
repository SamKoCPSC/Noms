// Ship the reviewed WebP derivatives without the base64 overhead of the offline HTML.
// Geometry, dimensions and clear-glass materials are checked by the preview packager.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
const normal=join(tmpdir(),'noms-village-app-source'), mobile=join(tmpdir(),'noms-village-app-encoded');
execFileSync(process.execPath,['tools/about/village/build-preview.mjs',normal],{stdio:'inherit'});
execFileSync(process.execPath,['tools/about/village/build-mobile-preview.mjs',normal,mobile,'76'],{stdio:'inherit'});
const html=await readFile(join(mobile,'OPEN-VILLAGE-MOBILE.html'),'utf8');
const out='assets/about/village';await mkdir(out,{recursive:true});
const receipts=JSON.parse(await readFile(join(mobile,'mobile-package-receipts.json'),'utf8'));
for(const receipt of receipts){
 const match=html.match(new RegExp(`<script type="application/octet-stream" id="asset-${receipt.id}">([^<]+)</script>`));
 if(!match)throw Error(`Missing embedded asset ${receipt.id}`);
 const bytes=Buffer.from(match[1],'base64');
 if(bytes.length!==receipt.preview_bytes)throw Error('Incomplete GLB');
 await writeFile(`${out}/${receipt.id}.glb`,bytes);
 receipt.sha256=createHash('sha256').update(bytes).digest('hex');
}
await writeFile(`${out}/receipts.json`,JSON.stringify(receipts,null,2)+'\n');
console.log('VILLAGE_APP_ASSETS_READY',receipts.reduce((n,r)=>n+r.preview_bytes,0));

await import("./build-camera.mjs");
