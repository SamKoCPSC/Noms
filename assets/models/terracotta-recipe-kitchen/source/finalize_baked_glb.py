"""Mark display-baked materials unlit, preserving texture UVs and binary data."""
import json
from pathlib import Path
import struct
import sys


def finalize(path, expected_count=2):
    path = Path(path)
    data = path.read_bytes()
    magic, version, length = struct.unpack_from('<4sII', data)
    assert magic == b'glTF' and version == 2 and length == len(data)
    size, kind = struct.unpack_from('<I4s', data, 12)
    assert kind == b'JSON'
    document = json.loads(data[20:20 + size])
    count = 0
    for material in document['materials']:
        if not material.get('name', '').startswith('Baked master lighting '):
            continue
        # Blender exports emission-only nodes as emissive PBR. These atlases
        # already contain the source view transform: never light them again.
        texture = material.pop('emissiveTexture')
        material.pop('emissiveFactor', None)
        material['pbrMetallicRoughness'] = {
            'baseColorTexture': texture, 'metallicFactor': 0, 'roughnessFactor': 1,
        }
        material.setdefault('extensions', {})['KHR_materials_unlit'] = {}
        count += 1
    assert count == expected_count, f'Expected {expected_count} baked atlases, got {count}'
    used = document.setdefault('extensionsUsed', [])
    if 'KHR_materials_unlit' not in used:
        used.append('KHR_materials_unlit')
    encoded = json.dumps(document, separators=(',', ':')).encode()
    encoded += b' ' * (-len(encoded) % 4)
    chunks = struct.pack('<I4s', len(encoded), b'JSON') + encoded + data[20 + size:]
    path.write_bytes(struct.pack('<4sII', b'glTF', 2, 12 + len(chunks)) + chunks)


if __name__ == '__main__':
    finalize(sys.argv[1])
