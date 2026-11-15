"""Check actual accessory triangles against the surrounding room surfaces.

python tools/about/check_plush_clearance.py
Small contacts within 8 mm of the feet are allowed for curved cushions/seats.
All other triangle intersections are reported, including window sills above a
support surface; checking foot height alone would miss those collisions.
"""
import json
import numpy as np
from add_penguin_plushes import ROOT, LIBRARY, read_glb, accessor

def triangles(doc, binary, primitive):
    node = doc['nodes'][0]
    points = accessor(doc, binary, primitive['attributes']['POSITION']) * node['scale'] + node['translation']
    return points[accessor(doc, binary, primitive['indices']).ravel()].reshape(-1, 3, 3)

def intersects(a, b):
    """Triangle/triangle separating-axis test, including coplanar edge axes."""
    ea = np.roll(a, -1, axis=0) - a
    eb = np.roll(b, -1, axis=1) - b
    na = np.cross(ea[0], ea[1])
    nb = np.cross(eb[:, 0], eb[:, 1])
    axes = [np.broadcast_to(na, (len(b), 3)), nb]
    axes += [np.cross(edge, eb[:, j]) for edge in ea for j in range(3)]
    axes += [np.broadcast_to(np.cross(na, edge), (len(b), 3)) for edge in ea]
    axes += [np.cross(nb, eb[:, j]) for j in range(3)]
    hit = np.ones(len(b), dtype=bool)
    for axis in axes:
        norm = np.linalg.norm(axis, axis=1)
        usable = norm > 1e-12
        unit = axis / np.maximum(norm[:, None], 1e-20)
        pa = np.einsum('ij,kj->ik', unit, a)
        pb = np.einsum('ij,ikj->ik', unit, b)
        separated = (pa.max(1) < pb.min(1) - 1e-7) | (pb.max(1) < pa.min(1) - 1e-7)
        hit &= ~(usable & separated)
        if not hit.any():break
    return hit

def main():
    manifest = json.loads((LIBRARY / 'placements.json').read_text())
    failures = []
    for row in manifest['models']:
        doc, binary = read_glb(ROOT / 'assets/about' / row['file'])
        originals, companions = [], []
        names = []
        for p in doc['meshes'][0]['primitives']:
            name = doc['materials'][p['material']]['name']
            t = triangles(doc, binary, p)
            if name.startswith('Penguin plush '):companions.append(t)
            else:
                originals.append(t)
                names.extend([name] * len(t))
        room = np.concatenate(originals)
        rlo, rhi = room.min(1), room.max(1)
        names = np.array(names)
        for plush, placement in zip(companions, row['plushes']):
            lo, hi = plush.min((0, 1)), plush.max((0, 1))
            near = np.all(rhi >= lo, axis=1) & np.all(rlo <= hi, axis=1)
            # Ignore only actual sole contact; keep chair backs, sills and props.
            near &= rhi[:, 1] > placement['position'][1] + .008
            rt, low, high, labels = room[near], rlo[near], rhi[near], names[near]
            clashes = set()
            contacts = []
            for tri in plush:
                candidates = np.all(high >= tri.min(0), axis=1) & np.all(low <= tri.max(0), axis=1)
                if candidates.any():
                    hits = intersects(tri, rt[candidates])
                    clashes.update(labels[candidates][hits])
                    if hits.any():contacts.extend(rt[candidates][hits].reshape(-1,3))
            print(row['model'], placement['label'], sorted(clashes) or 'clear', flush=True)
            if contacts:print('  collision area:', np.min(contacts,0).round(4), np.max(contacts,0).round(4), flush=True)
            if clashes:failures.append((row['model'], placement['label'], sorted(clashes)))
    assert not failures, failures
    print('PASS: every Penguini is clear of surrounding geometry above its feet')

if __name__ == '__main__':main()
