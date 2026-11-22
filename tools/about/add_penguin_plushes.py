"""Append editable plush companions without re-exporting any original geometry.

python tools/about/add_penguin_plushes.py --base-dir /tmp/about-plush-baselines

Requires numpy. Inputs are hash-checked against the placement manifest. Originals
remain byte-for-byte intact within the binary buffer; new materials/assemblies
are appended. Re-run from the same clean input directory, never the output.
"""
import argparse
import hashlib
import json
import math
import struct
from pathlib import Path
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
LIBRARY = ROOT / 'assets/models/about-penguin-plushes'

def read_glb(path):
    raw = Path(path).read_bytes()
    assert struct.unpack_from('<III', raw) == (0x46546c67,2,len(raw))
    length = struct.unpack_from('<I',raw,12)[0]
    return json.loads(raw[20:20+length]),raw[28+length:]

def accessor(doc, binary, index):
    a=doc['accessors'][index];v=doc['bufferViews'][a['bufferView']]
    width={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4}[a['type']]
    dtype={5121:'u1',5122:'<i2',5123:'<u2',5125:'<u4',5126:'<f4'}[a['componentType']]
    assert 'byteStride' not in v, 'Expected packed exporter attributes'
    values=np.frombuffer(binary,dtype=dtype,count=a['count']*width,
        offset=v.get('byteOffset',0)+a.get('byteOffset',0)).reshape(-1,width)
    if a.get('normalized'):
        values=values.astype(float)/{5121:255,5122:32767,5123:65535}[a['componentType']]
    return values

def write_glb(path, doc, binary):
    doc['buffers']=[dict(byteLength=len(binary))]
    js=json.dumps(doc,separators=(',',':')).encode();js+=b' '*((-len(js))%4)
    binary+=b'\0'*((-len(binary))%4)
    Path(path).write_bytes(struct.pack('<III',0x46546c67,2,28+len(js)+len(binary))+
        struct.pack('<II',len(js),0x4e4f534a)+js+struct.pack('<II',len(binary),0x004e4942)+binary)

def append_plush(doc, binary, library, library_binary, placement, number):
    mesh=next(m for m in library['meshes'] if m['name']==placement['variant'])
    p=mesh['primitives'][0];attrs=p['attributes']
    pos=accessor(library,library_binary,attrs['POSITION']).astype(float)
    normals=accessor(library,library_binary,attrs['NORMAL']).astype(float)
    colors=accessor(library,library_binary,attrs['COLOR_0'])
    indices=accessor(library,library_binary,p['indices']).flatten()
    angle=math.radians(placement['yaw_degrees']);c,s=math.cos(angle),math.sin(angle)
    rotation=np.array([[c,0,s],[0,1,0],[-s,0,c]])
    pos[:,1]-=pos[:,1].min()
    pos=pos@rotation.T*(placement['height']/pos[:,1].max())+placement['position']
    normals=normals@rotation.T
    node=doc['nodes'][0];scale=np.array(node['scale']);origin=np.array(node['translation'])
    normalized=(pos-origin)/scale
    assert normalized.min()>=0 and normalized.max()<=1, 'Plush outside existing quantization volume'
    # Welding reduces a companion to one small draw call, with no new texture.
    values=np.column_stack((np.round(normalized*65535),np.round(normals*32767),np.round(colors*255)))
    unique,inverse=np.unique(values,axis=0,return_inverse=True)
    def attr(data,kind,dtype,normalized=False,index=False):
        while len(binary)%4: binary.append(0)
        raw=np.asarray(data,dtype=dtype).tobytes()
        doc['bufferViews'].append(dict(buffer=0,byteOffset=len(binary),byteLength=len(raw),target=34963 if index else 34962))
        binary.extend(raw)
        a=dict(bufferView=len(doc['bufferViews'])-1,componentType=kind,count=len(data),type='SCALAR' if index else 'VEC3')
        if normalized:a['normalized']=True
        if kind==5123 and not index:
            a['min']=data.min(0).astype(int).tolist();a['max']=data.max(0).astype(int).tolist()
        doc['accessors'].append(a)
        return len(doc['accessors'])-1
    attributes={'POSITION':attr(unique[:,:3],5123,'<u2',True),
        'NORMAL':attr(unique[:,3:6],5122,'<i2',True),
        'COLOR_0':attr(unique[:,6:9],5121,'u1',True)}
    name=f"Penguin plush {number:02} - {placement['label']}"
    assembly=dict(name=name,pivot=placement['position'],offset=[.15,.55,.4],start=110,end=148,texture=0,opacity=1,masterLighting=False,surfaceMaterials=False)
    doc['materials'].append(dict(name=name,extras=dict(assembly=assembly),
        pbrMetallicRoughness=dict(metallicFactor=0,roughnessFactor=.95),doubleSided=True))
    doc['meshes'][0]['primitives'].append(dict(attributes=attributes,
        indices=attr(inverse[indices],5125,'<u4',index=True),material=len(doc['materials'])-1,mode=4))
    return len(indices)//3

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base-dir',type=Path,required=True)
    parser.add_argument('--output',type=Path,default=ROOT/'assets/about')
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    manifest=json.loads((LIBRARY/'placements.json').read_text())
    library,lb=read_glb(LIBRARY/'models/penguini-plush.glb')
    character=json.loads((LIBRARY/'library-metadata.json').read_text())
    reports=[]
    for row in manifest['models']:
        source=args.base_dir/row['file'];raw=source.read_bytes()
        assert hashlib.sha256(raw).hexdigest()==row['base_sha256'], f"Base changed: {row['model']}; review placements before applying"
        doc,base=read_glb(source);binary=bytearray(base)
        original_counts={key:len(doc[key]) for key in ('materials','accessors','bufferViews')}
        original_counts['primitives']=len(doc['meshes'][0]['primitives'])
        added=sum(append_plush(doc,binary,library,lb,p,i+1)for i,p in enumerate(row['plushes']))
        assert bytes(binary[:len(base)])==base
        budget=json.loads((args.base_dir/f"{row['model']}-budget.json").read_text())
        assert budget['triangles']+added<=400_000
        assert len(doc['meshes'][0]['primitives'])<=64
        doc.setdefault('extras',{})['penguin_plushes']=dict(count=len(row['plushes']),base_sha256=row['base_sha256'],
            base_binary_bytes=len(base),base_counts=original_counts,placements=row['plushes'],
            character_source=character['source'],character_source_sha256=character['source_sha256'])
        path=args.output/row['file'];write_glb(path,doc,binary)
        budget.update(triangles=budget['triangles']+added,glb_bytes=path.stat().st_size,
            materials=len(doc['materials']),penguin_plushes=len(row['plushes']),plush_triangles=added,
            plush_source='assets/models/about-penguin-plushes',base_glb_sha256=row['base_sha256'],
            character_source=character['source'],character_source_sha256=character['source_sha256'])
        for key in ('assemblies','assembly_groups'):
            if key in budget:budget[key]+=len(row['plushes'])
        if 'draw_calls' in budget:budget['draw_calls']+=len(row['plushes'])
        (args.output/f"{row['model']}-budget.json").write_text(json.dumps(budget,indent=2)+'\n')
        reports.append(dict(model=row['model'],plushes=len(row['plushes']),added_triangles=added,
            total_triangles=budget['triangles'],added_bytes=path.stat().st_size-len(raw),
            original_geometry_sha256=hashlib.sha256(base).hexdigest(),original_buffer_preserved=True))
    (LIBRARY/'integration-audit.json').write_text(json.dumps(reports,indent=2)+'\n')
    print(json.dumps(reports,indent=2))

if __name__=='__main__':main()
