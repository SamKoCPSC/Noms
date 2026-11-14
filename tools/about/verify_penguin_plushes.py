"""Verify original scene preservation, budgets, and plush-to-furniture contact.

python tools/about/verify_penguin_plushes.py --base-dir /tmp/about-plush-baselines
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import numpy as np
from add_penguin_plushes import ROOT, LIBRARY, read_glb, accessor

def support_heights(doc, binary, support, x, z):
    node=doc['nodes'][0];found=[]
    for p in doc['meshes'][0]['primitives']:
        if doc['materials'][p['material']]['name']!=support:continue
        v=accessor(doc,binary,p['attributes']['POSITION'])*node['scale']+node['translation']
        triangles=v[accessor(doc,binary,p['indices']).flatten()].reshape(-1,3,3)
        a=triangles[:,0];ab=triangles[:,1]-a;ac=triangles[:,2]-a
        determinant=ab[:,0]*ac[:,2]-ab[:,2]*ac[:,0]
        mask=abs(determinant)>1e-8
        a,ab,ac,determinant=a[mask],ab[mask],ac[mask],determinant[mask]
        dx=x-a[:,0];dz=z-a[:,2]
        u=(dx*ac[:,2]-dz*ac[:,0])/determinant
        w=(ab[:,0]*dz-ab[:,2]*dx)/determinant
        hit=(u>=-1e-6)&(w>=-1e-6)&(u+w<=1+1e-6)
        found.extend((a[:,1]+u*ab[:,1]+w*ac[:,1])[hit].tolist())
    return found

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base-dir',required=True,type=Path)
    args=parser.parse_args()
    manifest=json.loads((LIBRARY/'placements.json').read_text())
    total=0
    for row in manifest['models']:
        before,bb=read_glb(args.base_dir/row['file'])
        after,ab=read_glb(ROOT/'assets/about'/row['file'])
        assert hashlib.sha256((args.base_dir/row['file']).read_bytes()).hexdigest()==row['base_sha256']
        assert ab[:len(bb)]==bb, 'Existing binary geometry/UV/color data changed'
        for key in ('materials','accessors','bufferViews'):
            assert before[key]==after[key][:len(before[key])], key
        assert before['nodes']==after['nodes']
        bp=before['meshes'][0]['primitives'];ap=after['meshes'][0]['primitives']
        assert bp==ap[:len(bp)]
        assert 1<=len(ap)-len(bp)==len(row['plushes'])<=2
        triangles=sum(after['accessors'][p['indices']]['count']//3 for p in ap)
        budget=json.loads((ROOT/f"assets/about/{row['model']}-budget.json").read_text())
        assert triangles==budget['triangles']<=400_000
        assert len(ap)<=64
        gaps=[]
        for p in row['plushes']:
            x,y,z=p['position'];angle=math.radians(p['yaw_degrees']);scale=p['height']/.98
            for side in (-1,1):
                dx=side*.139*scale;dz=.129*scale
                xx=x+math.cos(angle)*dx+math.sin(angle)*dz
                zz=z-math.sin(angle)*dx+math.cos(angle)*dz
                hits=support_heights(before,bb,p['support'],xx,zz)
                assert hits, f"No support under {row['model']} / {p['label']}"
                gap=y-min(hits,key=lambda h:abs(y-h))
                assert -.008<gap<.02, f"Foot contact: {row['model']} / {p['label']}: {gap}"
                gaps.append(round(gap,4))
        total+=len(row['plushes'])
        print(row['model'],f"{len(row['plushes'])} plushes; {triangles:,} triangles; foot gaps {gaps}; original data identical")
    assert total==15
    print('PASS: all 12 original scenes preserved; all 15 plushes supported and within budgets')

if __name__=='__main__':main()
