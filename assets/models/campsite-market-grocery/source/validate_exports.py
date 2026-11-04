"""Check GLB structure, numeric geometry, indices, materials and assembly hierarchy."""
from pathlib import Path
import struct,json,math,hashlib
import numpy as np
R=Path(__file__).resolve().parent.parent
types={5120:('i1',1),5121:('u1',1),5122:('<i2',2),5123:('<u2',2),5125:('<u4',4),5126:('<f4',4)}
counts={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4,'MAT2':4,'MAT3':9,'MAT4':16}
results=[]
for p in sorted((R/'models').glob('*.glb')):
 data=p.read_bytes();magic,version,size=struct.unpack_from('<4sII',data);assert (magic,version,size)==(b'glTF',2,len(data)),p
 jlen,jtype=struct.unpack_from('<I4s',data,12);assert jtype==b'JSON';doc=json.loads(data[20:20+jlen]);off=20+jlen
 blen,btype=struct.unpack_from('<I4s',data,off);assert btype==b'BIN\0';blob=data[off+8:off+8+blen];assert len(blob)==blen
 assert doc['buffers'][0]['byteLength']<=len(blob)
 def read_accessor(idx):
  a=doc['accessors'][idx];v=doc['bufferViews'][a['bufferView']];dt,sz=types[a['componentType']];n=counts[a['type']];stride=v.get('byteStride',sz*n);start=v.get('byteOffset',0)+a.get('byteOffset',0)
  end=start+(a['count']-1)*stride+n*sz
  assert end<=v.get('byteOffset',0)+v['byteLength']<=len(blob),(p,idx)
  return np.ndarray((a['count'],n),dtype=dt,buffer=blob,offset=start,strides=(stride,sz))
 vertices=triangles=primitives=0
 for mesh in doc['meshes']:
  for prim in mesh['primitives']:
   primitives+=1;pos=read_accessor(prim['attributes']['POSITION']);assert np.isfinite(pos).all();vertices+=len(pos)
   assert 0<=prim['material']<len(doc['materials'])
   for attr,idx in prim['attributes'].items():
    a=read_accessor(idx);assert len(a)==len(pos) and np.isfinite(a).all(),(p,attr)
   indices=read_accessor(prim['indices']).reshape(-1);assert len(indices)%3==0 and indices.min()>=0 and indices.max()<len(pos);triangles+=len(indices)//3
 assert vertices>1000 and triangles>1000
 roots=doc['scenes'][doc.get('scene',0)]['nodes'];assert len(roots)>=7
 visited=set()
 def walk(idx,anc):
  assert idx not in anc and 0<=idx<len(doc['nodes']);n=doc['nodes'][idx];visited.add(idx)
  for k in ['translation','rotation','scale','matrix']:
   if k in n:assert all(math.isfinite(v)for v in n[k])
  if 'mesh' in n:assert 0<=n['mesh']<len(doc['meshes'])
  for ch in n.get('children',[]):walk(ch,anc|{idx})
 for i in roots:
  assert 'assembly' in doc['nodes'][i]['extras'];walk(i,set())
 for image in doc.get('images',[]):assert 'bufferView' in image,'GLB must be self-contained'
 results.append(dict(file=p.name,bytes=len(data),assemblies=len(roots),nodes=len(visited),primitives=primitives,vertices=vertices,triangles=triangles,materials=len(doc['materials']),embedded_images=len(doc.get('images',[])),sha256=hashlib.sha256(data).hexdigest(),validation='PASS'))
 print(p.name,triangles,'triangles',len(roots),'assemblies: PASS',flush=True)
(R/'validation.json').write_text(json.dumps({'checks':'GLB headers, buffer bounds, finite attributes/transforms, triangle indices, PBR material references, acyclic hierarchy, embedded images','models':results},indent=2))
assert len(results)==3
