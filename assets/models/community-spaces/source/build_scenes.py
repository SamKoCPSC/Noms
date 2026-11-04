"""Run: blender -b --threads 2 --python build_scenes.py -- coffee_shop [--proof]
Other scene keys: farmers_market, artisan_bakery, night_market, recipe_library.
Output lives in ../models and ../previews. All assets are local and packed.
"""
from pathlib import Path
import sys
base=Path(__file__).resolve().parent
for folder in ['models','previews']:(base.parent/folder).mkdir(exist_ok=True)
SCENE_KEY=sys.argv[sys.argv.index('--')+1]
if SCENE_KEY not in ['coffee_shop','farmers_market','artisan_bakery','night_market','recipe_library']:raise ValueError('Unknown scene key')
exec(compile((base/'diorama_common.py').read_text(),str(base/'diorama_common.py'),'exec'))
exec(compile((base/'scene_details.py').read_text(),str(base/'scene_details.py'),'exec'))
exec(compile((base/f'{SCENE_KEY}.py').read_text(),str(base/f'{SCENE_KEY}.py'),'exec'))
