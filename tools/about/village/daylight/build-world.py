"""Save a portable native world with the same bright daylight used for every bake."""
import bpy,json,sys,hashlib
from pathlib import Path
from mathutils import Matrix
sys.path.insert(0,str(Path(__file__).resolve().parent))
from common import *
out=Path(sys.argv[sys.argv.index('--')+1]);out.parent.mkdir(parents=True,exist_ok=True)
hashes=originals()
bpy.ops.wm.open_mainfile(filepath=str(ASSET/'models/terraced-village-blockout.blend'),load_ui=False,use_scripts=False)
scene=bpy.context.scene
if LAYOUT.get('stone_paths') or LAYOUT.get('vegetation'):
    previous=bpy.data.collections['World terrain — blockout']
    for obj in list(previous.objects):bpy.data.objects.remove(obj,do_unlink=True)
    bpy.data.collections.remove(previous)
    terrain=environment(scene);terrain.name='World terrain — blockout'
for obj in scene.objects:
    if obj.instance_type=='COLLECTION' and 'stop' in obj:
        key=LAYOUT['stops'][obj['stop']-1]['id'];obj.matrix_world=anchor(key)@Matrix.Translation(obj.instance_collection.instance_offset)
lighting(scene);scene.frame_set(1);scene['stage']=LAYOUT['stage']
text=bpy.data.texts.get('Shared daylight settings') or bpy.data.texts.new('Shared daylight settings');text.clear();text.write(json.dumps(CONFIG,indent=2))
bpy.context.preferences.filepaths.save_version=0;bpy.ops.file.pack_all();bpy.ops.wm.save_as_mainfile(filepath=str(out),compress=True)
for path,sha in hashes.items():assert hashlib.sha256((REPO/path).read_bytes()).hexdigest()==sha
print('DAYLIGHT_WORLD_SAVED',out,flush=True)
