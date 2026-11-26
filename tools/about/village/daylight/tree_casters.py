"""Replace the original outdoor trees with the tour's exact shadow casters for baking."""
import hashlib
import subprocess
import tempfile
from pathlib import Path

import bpy
from common import REPO


def install_tree_casters():
    terrain = bpy.data.collections['World terrain — blockout']
    retired = []
    for obj in terrain.objects:
        if obj.name.startswith(('Layered broadleaf foliage ', 'Ridged evergreen foliage ', 'Faceted tree trunk')):
            retired.append(obj)
        elif (obj.name.startswith('Landscape detail') and len(obj.data.vertices) == 26
              and abs(obj.active_material.diffuse_color[0] - 0.21586) < 1e-5):
            retired.append(obj)
    # 50 individual trunks, one combined trunk mesh and 20 foliage batches.
    assert len(retired) == 71, f'Unexpected original tree mesh count: {len(retired)}'
    for obj in retired:
        bpy.data.objects.remove(obj, do_unlink=True)
    with tempfile.TemporaryDirectory() as folder:
        path = Path(folder) / 'tree-casters.glb'
        subprocess.run(['node', 'tools/about/village/trees/export-casters.mjs', str(path)], cwd=REPO, check=True)
        revision = hashlib.sha256(path.read_bytes()).hexdigest()
        bpy.ops.import_scene.gltf(filepath=str(path))
    crowns = [obj for obj in bpy.context.selected_objects if obj.type == 'MESH' and obj.name.startswith('Village tree foliage')]
    assert len(crowns) == 116, f'Expected 116 planted canopies, got {len(crowns)}'
    print('TREE_CASTERS_READY', len(crowns), revision, flush=True)
    return revision
