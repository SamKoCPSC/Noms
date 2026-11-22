"""blender -b --threads 2 --python source/build_scenes.py -- campsite [--proof]
Other scenes: night_market, grocery_store. All geometry is authored locally.
"""
from pathlib import Path
import sys
base=Path(__file__).resolve().parent
for folder in ['models','previews']:(base.parent/folder).mkdir(exist_ok=True)
SCENE_KEY=sys.argv[sys.argv.index('--')+1]
if SCENE_KEY not in ['campsite','night_market','grocery_store']:raise ValueError('Unknown scene')
for script in ['diorama_common','scene_details','new_details',SCENE_KEY]:
 exec(compile((base/(script+'.py')).read_text(),str(base/(script+'.py')),'exec'))
