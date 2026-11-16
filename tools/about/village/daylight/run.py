"""Run resumable room bakes with bounded parallelism; never publish partial assets."""
import argparse, concurrent.futures, hashlib, json, subprocess, time
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[4]
parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);parser.add_argument('--workers',type=int,default=2);parser.add_argument('--threads',type=int,default=2);args=parser.parse_args()
config=json.loads((ROOT/'assets/models/terraced-village/daylight/lighting.json').read_text())
stops=json.loads((ROOT/'assets/models/terraced-village/layout.json').read_text())['stops']
anchors=json.loads((ROOT/'assets/models/terraced-village/daylight/source/anchors.json').read_text())
def inputs_match(metadata,key):
    return metadata.get('lighting')==config and metadata.get('anchor')==anchors[key] and all(
        hashlib.sha256((ROOT/path).read_bytes()).hexdigest()==sha
        for path,sha in metadata.get('source_hashes',{}).items()) and bool(metadata.get('source_hashes'))
def work(stop):
    key=stop['id'];out=args.directory/key;out.mkdir(parents=True,exist_ok=True)
    result=out/'bake.json'
    target=ROOT/'assets/models/terraced-village/daylight/models'/f'{key}.glb'
    if result.exists() and target.exists() and inputs_match(json.loads(result.read_text()),key) and json.loads(result.read_text()).get('packed_sha256')==hashlib.sha256(target.read_bytes()).hexdigest():
        print('CACHED',key,flush=True);return
    prepared=out/'metadata.json'
    commands=[]
    if not prepared.exists() or not (out/'scene.blend').exists() or not inputs_match(json.loads(prepared.read_text()),key):commands.append('prepare-room.py')
    commands.append('bake-room.py')
    for script in commands:
        print('START',key,script,flush=True)
        with (out/(script+'.log')).open('w') as log:
            subprocess.run(['blender','-b','-t',str(args.threads),'--disable-autoexec','--python-exit-code','1','--python',str(Path(__file__).parent/script),'--',key,str(out)],cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
    assert (out/(key+'-baked.glb')).stat().st_size>10000
    for label in ('architecture','details'):
        with Image.open(out/(label+'-baked.png')) as image:assert image.size==(2048,2048) and max(hi for lo,hi in image.convert('RGB').getextrema())>20
    packed=target.with_suffix('.tmp.glb')
    subprocess.run(['node','tools/about/baked/pack.mjs',str(out/(key+'-baked.glb')),str(packed),str(result)],cwd=ROOT,check=True)
    packed.replace(target);meta=json.loads(result.read_text());meta['packed_sha256']=hashlib.sha256(target.read_bytes()).hexdigest();result.write_text(json.dumps(meta,indent=2)+'\n')
    destination=ROOT/'assets/models/terraced-village/daylight/source'/f'{key}-bake.json';destination.write_text(result.read_text())
    # These evaluated intermediates are reproducible and consume tmpfs memory.
    for file in [out/'scene.blend',out/(key+'-baked.glb'),*out.glob('*-linear.exr')]:file.unlink(missing_ok=True)
    print('COMPLETE',key,flush=True)
failures=[]
with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
    tasks={pool.submit(work,s):s['id'] for s in stops}
    for task in concurrent.futures.as_completed(tasks):
        try:task.result()
        except Exception as e:failures.append(tasks[task]);print('FAILED',tasks[task],str(e),flush=True)
if failures:raise SystemExit('Bakes failed: '+', '.join(failures))
print('ALL_ROOM_BAKES_COMPLETE',flush=True)
