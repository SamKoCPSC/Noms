#!/usr/bin/env python3
"""Check every active baked asset against its preserved source and UV contract."""
import hashlib
import json
import struct
from pathlib import Path
from io import BytesIO
from PIL import Image

ROOT=Path(__file__).resolve().parents[2]
config=json.loads((ROOT/'tools/about/baked/models.json').read_text())
assert len(config)==12
for model,entry in config.items():
    lighting=json.loads((ROOT/f'assets/about/{model}-lighting.json').read_text())
    source=ROOT/entry.get('master',entry.get('export'))
    assert hashlib.sha256(source.read_bytes()).hexdigest()==lighting['source_sha256'],model
    data=(ROOT/f'assets/about/{model}-baked.glb').read_bytes()
    assert struct.unpack_from('<4sII',data)==(b'glTF',2,len(data))
    size,kind=struct.unpack_from('<I4s',data,12);assert kind==b'JSON'
    gltf=json.loads(data[20:20+size]);binary=data[28+size:]
    assert not gltf.get('animations') and lighting['animations'] is False
    assert lighting['frame']==160 and lighting['glb_bytes']==len(data)
    assert len(data)<100_000_000 and all('uri' not in b for b in gltf['buffers'])
    baked={i:m for i,m in enumerate(gltf['materials']) if m.get('name','').startswith('Baked master lighting ')}
    assert len(baked)==2,(model,len(baked))
    for index,material in baked.items():
        assert 'KHR_materials_unlit' in material.get('extensions',{}) and 'emissiveTexture' not in material
        texture=material['pbrMetallicRoughness']['baseColorTexture'];uv='TEXCOORD_'+str(texture.get('texCoord',0))
        image=gltf['images'][gltf['textures'][texture['index']]['source']]
        view=gltf['bufferViews'][image['bufferView']];start=view.get('byteOffset',0)
        with Image.open(BytesIO(binary[start:start+view['byteLength']])) as image:
            assert image.size==(2048,2048)
            assert max(high for low,high in image.convert('RGB').getextrema())>20,(model,'empty bake')
        for mesh in gltf['meshes']:
            for primitive in mesh['primitives']:
                if primitive['material']==index:assert uv in primitive['attributes']
    triangles=sum(gltf['accessors'][p['indices']]['count']//3 for m in gltf['meshes'] for p in m['primitives'])
    assert triangles==lighting['triangles'],(model,triangles,lighting['triangles'])
    assert lighting['geometry_compression']['triangles_removed']==0
    assert lighting['penguini'] is True
    if 'export' in entry:
        assert lighting['source_kind']=='reconstructed-export'
        live=[p for m in gltf['meshes'] for p in m['primitives'] if p['material'] not in baked]
        assert live and all('COLOR_0' in p['attributes'] for p in live),(model,'lost live material tints')
    print(f'PASS {model}: preserved source, static geometry, embedded unlit lighting, {len(data):,} bytes')
