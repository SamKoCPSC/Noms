#!/usr/bin/env python3
"""Validate the source-to-web bake contract without requiring Blender/WebGL."""
import hashlib
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[2]
source = ROOT / 'assets/models/terracotta-recipe-kitchen'
receipt = json.loads((source / 'source-receipt.json').read_text())
lighting = json.loads((ROOT / 'assets/about/kitchen-lighting.json').read_text())
master = source / 'models/sams-kitchen-recipe-manager.blend'
assert hashlib.sha256(master.read_bytes()).hexdigest() == lighting['source_sha256'] == receipt['sha256']
data = (ROOT / 'assets/about/kitchen-baked.glb').read_bytes()
magic, version, length = struct.unpack_from('<4sII', data)
assert (magic, version, length) == (b'glTF', 2, len(data))
size, kind = struct.unpack_from('<I4s', data, 12)
assert kind == b'JSON'
gltf = json.loads(data[20:20 + size])
assert not gltf.get('animations'), 'Baked shadows require a static model'
assert len(gltf['images']) == 2
assert all('bufferView' in image and 'uri' not in image for image in gltf['images'])
assert len(gltf['buffers']) == 1 and 'uri' not in gltf['buffers'][0]
baked = {i: m for i, m in enumerate(gltf['materials']) if m.get('name', '').startswith('Baked master lighting ')}
assert len(baked) == 2
for index, material in baked.items():
    assert 'KHR_materials_unlit' in material.get('extensions', {}), 'Do not light baked colors twice'
    assert 'emissiveTexture' not in material
    texture = material['pbrMetallicRoughness']['baseColorTexture']
    uv = 'TEXCOORD_' + str(texture.get('texCoord', 0))
    for mesh in gltf['meshes']:
        for primitive in mesh['primitives']:
            if primitive['material'] == index:
                assert uv in primitive['attributes'], (material['name'], uv)
assert any('KHR_materials_transmission' in m.get('extensions', {}) for m in gltf['materials'])
assert lighting['frame'] == 160 and lighting['animations'] is False
assert len(data) == lighting['glb_bytes']
print(f'PASS: unchanged master, embedded atlases/UVs, static geometry, unlit bake, live glass ({len(data):,} bytes)')
