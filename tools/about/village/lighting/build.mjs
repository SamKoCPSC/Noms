// Produce a self-contained before/after review, preserving PNG resolution.
import {readFile,writeFile,mkdir,copyFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {build} from 'esbuild';
import {NodeIO} from '@gltf-transform/core';
import {ALL_EXTENSIONS} from '@gltf-transform/extensions';
import {dedup,meshopt,mergeDocuments,prune,unpartition} from '@gltf-transform/functions';
import {MeshoptDecoder,MeshoptEncoder} from 'meshoptimizer';
import sharp from 'sharp';
const [source,out]=process.argv.slice(2);if(!out)throw Error('Usage: build.mjs BAKE_DIRECTORY OUTPUT');
await mkdir(out,{recursive:true});await MeshoptDecoder.ready;await MeshoptEncoder.ready;
const io=new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({'meshopt.decoder':MeshoptDecoder,'meshopt.encoder':MeshoptEncoder});
const receipts=[];
for(const [id,path] of [['before',source+'/district-before.glb'],['after',source+'/district-baked.glb'],['kitchen','assets/about/kitchen-baked.glb']]){
 const original=await readFile(path),doc=await io.readBinary(new Uint8Array(original));
 if(id==='after'){
  for(const node of doc.getRoot().listNodes())if(node.getName()==='Baked district landscape')node.dispose();
  mergeDocuments(doc,await io.read(source+'/landscape-vertex.glb'));
  const [scene,...extra]=doc.getRoot().listScenes();for(const other of extra){for(const node of other.listChildren())scene.addChild(node);other.dispose();}
  await doc.transform(prune(),unpartition());
 }
 const count=()=>doc.getRoot().listMeshes().flatMap(m=>m.listPrimitives()).reduce((n,p)=>n+(p.getIndices()?.getCount()||p.getAttribute('POSITION').getCount())/3,0),triangles=count();
 if(id==='kitchen')await copyFile(path,out+'/'+id+'.glb');
 else{
  for(const texture of doc.getRoot().listTextures())if(texture.getMimeType()==='image/png')texture.setImage(new Uint8Array(await sharp(Buffer.from(texture.getImage())).removeAlpha().png({compressionLevel:9}).toBuffer()));
  await doc.transform(dedup(),meshopt({encoder:MeshoptEncoder,level:'medium',quantizePosition:16,quantizeNormal:12,quantizeTexcoord:16}));
  if(count()!==triangles)throw Error('Triangle count changed');await io.write(out+'/'+id+'.glb',doc);
 }
 const data=await readFile(out+'/'+id+'.glb');receipts.push({id,source_sha256:createHash('sha256').update(original).digest('hex'),...(id==='after'?{vertex_source_sha256:createHash('sha256').update(await readFile(source+'/landscape-vertex.glb')).digest('hex')}:{}),bytes:data.length,triangles,sha256:createHash('sha256').update(data).digest('hex')});
}
if(receipts.find(r=>r.id==='after').triangles!==receipts.filter(r=>r.id!=='after').reduce((n,r)=>n+r.triangles,0))throw Error('Before/after geometry totals differ');
const meta=JSON.parse(await readFile(source+'/study.json'));meta.walk=JSON.parse(await readFile('assets/models/terraced-village/source/walking-camera.json'))[0].slice(0,41);
await writeFile(out+'/study.json',JSON.stringify(meta));
const template=await readFile('tools/about/village/lighting/index.html','utf8');await writeFile(out+'/index.html',template);
const bundle=await build({entryPoints:['tools/about/village/lighting/viewer.js'],bundle:true,format:'esm',target:'es2022',minify:true,write:false,legalComments:'eof'});await writeFile(out+'/viewer.js',bundle.outputFiles[0].text);
let html=template.replace('<script type="module" src="viewer.js"></script></html>','');
html+='<script>globalThis.__STUDY__='+JSON.stringify(meta).replaceAll('</','<\\/')+';</script>';
for(const {id} of receipts)html+=`<script type="application/octet-stream" id="asset-${id}">${(await readFile(out+'/'+id+'.glb')).toString('base64')}</script>`;
for(const [name,path]of [['THREE-LICENSE.txt','tools/about/three/THREE-LICENSE.txt'],['MESHOPT-LICENSE.txt','tools/about/three/MESHOPT-LICENSE.txt'],['DROID-FONT-NOTICE.txt','tools/about/village/DROID-FONT-NOTICE.txt']]){
 await copyFile(path,out+'/'+name);html+='<script type="text/plain">'+(await readFile(path,'utf8')).replaceAll('</','<\\/')+'</script>';
}
html+='<script type="module">'+bundle.outputFiles[0].text.replaceAll('</script','<\\/script')+'</script></html>';
await writeFile(out+'/OPEN-KITCHEN-LIGHTING.html',html);await writeFile(out+'/package-receipts.json',JSON.stringify(receipts,null,2)+'\n');
console.log('LIGHTING_PREVIEW_READY',html.length,receipts);
