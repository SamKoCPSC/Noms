"""Cache the corrected native room geometry once before the twelve room bakes."""
import bpy,json,sys,hashlib
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
from common import *
from glass import apply_room_glass,glass_revision
out=Path(sys.argv[sys.argv.index('--')+1]);out.parent.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(ASSET/'models/terraced-village-blockout.blend'),load_ui=False,use_scripts=False)
report=apply_room_glass();report['source_hashes']=originals()
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(out),compress=True)
report['cached_blend_sha256']=hashlib.sha256(out.read_bytes()).hexdigest()
out.with_suffix('.json').write_text(json.dumps(report,indent=2)+'\n')
print('CLEAR_ROOM_SOURCE_READY',flush=True)
