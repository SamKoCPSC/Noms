#!/usr/bin/env python3
"""Bake selected tour scenes in isolated Blender processes, then publish verified outputs."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import shutil
import subprocess
from PIL import Image

REPO=Path(__file__).resolve().parents[3]
CONFIG=json.loads((Path(__file__).parent/'models.json').read_text())

def publish(model, directory):
    meta=json.loads((directory/f'{model}-lighting.json').read_text())
    glb=directory/f'{model}-baked.glb'
    assert glb.stat().st_size==meta['glb_bytes']
    packed=directory/f'{model}-packed.glb'
    subprocess.run(['node',str(REPO/'tools/about/baked/pack.mjs'),str(glb),str(packed),str(directory/f'{model}-lighting.json')],cwd=REPO,check=True)
    packed.replace(glb)
    meta=json.loads((directory/f'{model}-lighting.json').read_text())
    assert glb.stat().st_size<100_000_000
    for label in ['architecture','details']:
        with Image.open(directory/f'{label}-baked.png') as image:
            assert max(high for low,high in image.convert('RGB').getextrema())>20, f'{model}: empty {label} bake'
    for file in [glb,directory/f'{model}-lighting.json']:
        target=REPO/'assets/about'/file.name
        shutil.copy2(file,target.with_suffix(target.suffix+'.tmp'));target.with_suffix(target.suffix+'.tmp').replace(target)
    preview=REPO/'assets/models/about-baked-tour/previews';preview.mkdir(parents=True,exist_ok=True)
    with Image.open(directory/'master-reference.png') as image:
        image.convert('RGB').save(preview/f'{model}-reference.webp',quality=88)
    return meta

def main():
    parser=argparse.ArgumentParser();parser.add_argument('models',nargs='+',choices=list(CONFIG));parser.add_argument('--output',type=Path,default=Path('/tmp/about-bakes'));parser.add_argument('--workers',type=int,default=2);parser.add_argument('--threads',type=int,default=2);parser.add_argument('--samples',type=int,default=128);parser.add_argument('--publish-only',action='store_true')
    args=parser.parse_args()
    def work(model):
        directory=args.output/model;directory.mkdir(parents=True,exist_ok=True)
        if not args.publish_only:
            with (directory/'bake.log').open('w') as log:
                subprocess.run(['blender','-b','--threads',str(args.threads),'--python',str(Path(__file__).parent/'bake_scene.py'),'--',model,str(directory),str(args.samples)],cwd=REPO,stdout=log,stderr=subprocess.STDOUT,check=True)
            assert 'BAKE_COMPLETE' in (directory/'bake.log').read_text(), f'{model}: see {directory}/bake.log'
        meta=publish(model,directory);print(f"PUBLISHED {model}: {meta['triangles']:,} triangles, {meta['glb_bytes']:,} bytes",flush=True)
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        list(pool.map(work,args.models))
if __name__=='__main__':main()
