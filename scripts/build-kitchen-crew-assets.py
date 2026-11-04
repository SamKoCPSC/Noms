#!/usr/bin/env python3
"""Create script-free, pure-vector excited SVGs from the selected mascots."""
from pathlib import Path
import argparse
import sys
import copy
import json
import re
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1] / "assets/brand/kitchen-crew"
OUTPUTS = {}
NS = 'http://www.w3.org/2000/svg'
ET.register_namespace('', NS)
DURATION = 6600
CHARACTERS = [('marmalade', 'c1', 'Marmalade', 'cat'),
              ('nutmeg', 'o5', 'Nutmeg', 'owl'),
              ('bao', 'p1', 'Bao', 'panda')]


def node(name, attrs=None, text=None):
    e = ET.Element(f'{{{NS}}}{name}', attrs or {})
    e.text = text
    return e


def wrap(parent, child, id_, class_):
    index = list(parent).index(child)
    parent.remove(child)
    wrapper = node('g', {'id': id_, 'class': class_})
    wrapper.append(child)
    parent.insert(index, wrapper)
    return wrapper


def find(root, id_):
    return next(x for x in root.iter() if x.get('id') == id_)


def motion_css(name, frames):
    out = f'@keyframes {name} {{\n'
    for f in frames:
        props = []
        for key, val in f.items():
            if key == 'offset': continue
            props.append(f'{"animation-timing-function" if key == "easing" else key}: {val};')
        out += f'  {f["offset"] * 100:.8f}% {{ {" ".join(props)} }}\n'
    return out + '}\n'


def rotations(pairs, ease='ease-in-out'):
    return [{'offset': t, 'transform': f'rotate({deg}deg)', 'easing': ease} for t, deg in pairs]


def build(name, old, display, species):
    raw = (ROOT/'source'/f'{name}-still.svg').read_text()
    prefix = f'{name}-excited'
    raw = raw.replace(old+'-', prefix+'-')
    svg = ET.fromstring(raw)
    svg.set('viewBox', '0 0 512 672')
    svg.set('height', '672')
    svg.set('id', prefix)
    svg.set('data-duration-ms', str(DURATION))
    svg.find(f'{{{NS}}}title').text = f'{display} — excited jump and wave'
    svg.find(f'{{{NS}}}desc').text = (
        f'{display} does two excited hops, waves, and settles into the original resting pose. '
        'The animation plays once in 6.6 seconds. Reduced motion shows the resting pose. '
        'Transparent, editable vector artwork with no scripts or external resources.')
    art = next(e for e in svg if e.tag == f'{{{NS}}}g')
    art.set('transform', 'translate(0 106)')
    art.set('id', prefix+'-stage')
    shadow = find(art, prefix+'-ground-shadow')
    shadow.set('class', prefix+'-shadow')
    character = node('g', {'id': prefix+'-character', 'class': prefix+'-jump'})
    for el in list(art):
        if el is not shadow:
            art.remove(el)
            character.append(el)
    art.append(character)

    # Extra transform wrappers preserve all selected artwork's existing transforms.
    head = find(character, prefix+'-head')
    wrap(character, head, prefix+'-head-motion', prefix+'-head-motion')
    hat = find(head, prefix+'-hat')
    wrap(head, hat, prefix+'-hat-motion', prefix+'-hat-motion')
    left = find(character, prefix+'-left-arm')
    wrap(character, left, prefix+'-left-motion', prefix+'-left-motion')
    right = find(character, prefix+'-right-arm')
    wrap(character, right, prefix+'-right-motion', prefix+'-right-motion')
    if species == 'cat':
        tail = find(character, prefix+'-tail')
        wrap(character, tail, prefix+'-tail-motion', prefix+'-tail-motion')

    if species == 'owl':
        # Preserve painting order, but give wing, utensil, grip and finger marks
        # the same outer transform and shoulder pivot so nothing drifts apart.
        for suffix in ('whisk', 'tool-grip'):
            el = find(character, prefix+'-'+suffix)
            wrap(character, el, prefix+'-'+suffix+'-motion', prefix+'-right-motion')
        fingers = next(e for e in character if e.tag == f'{{{NS}}}path' and
                       e.get('d', '').startswith('M405 361'))
        wrap(character, fingers, prefix+'-finger-motion', prefix+'-right-motion')

    eyes = find(head, prefix+'-eyes')
    eye_wrap = wrap(head, eyes, prefix+'-blink', prefix+'-blink')
    eyes.set('class', prefix+'-eyes-rest')
    happy = node('g', {'id': prefix+'-eyes-happy', 'class': prefix+'-eyes-happy', 'opacity': '0'})
    y = 269 if species == 'owl' else 271
    space = 45 if species == 'owl' else 44
    for x in (256-space, 256+space):
        happy.append(node('path', {'d': f'M{x-10} {y+3} Q{x} {y-10} {x+10} {y+3}',
                                  'fill': 'none', 'stroke': '#fff1d9' if species=='panda' else '#493428',
                                  'stroke-width': '5'}))
    eye_wrap.append(happy)

    reference = json.loads((ROOT/'source'/'penguini-excited-reference.json').read_text())
    reference = {x['selector']: x['keyframes'] for x in reference['tracks']}
    jump = copy.deepcopy(reference['.penguin'])
    head_frames = copy.deepcopy(reference['.head'])
    # Scale Penguini's travel to these smaller vector rigs, retaining its timing.
    for frames in (jump, head_frames):
        for f in frames:
            f['transform'] = re.sub(r'(-?[\d.]+)px', lambda m: f'{float(m[1])*.46:g}px', f['transform'])
    shadow_frames = copy.deepcopy(reference['.shadow'])
    for f in shadow_frames:
        f['opacity'] = f'{float(f["opacity"])/.13:.6f}'

    tracks = {
        'jump': ((256,512),jump),
        'shadow': ((256,516),shadow_frames),
        'head-motion': ((256,349),head_frames),
        'left-motion': ((176,355),rotations([
            (0,0),(.04545,0),(.15909,22),(.24242,-7),(.29545,23),(.36364,-6),
            (.42424,25),(.47727,0),(.5303,25),(.58333,0),(.63636,24),
            (.68939,0),(.72727,8),(.90909,0),(1,0)])),
        'right-motion': ((336,355),rotations([
            (0,0),(.05,0),(.09,5),(.15545,-20 if species!='owl' else -13),
            (.23727,3),(.29455,-16 if species!='owl' else -10),(.36818,4),
            (.44,0),(.53,-5),(.62,2),(.72,0),(.90909,0),(1,0)])),
        'hat-motion': ((256,176),[
            {'offset':t,'transform':f'translateY({y}px) rotate({r}deg)','easing':'ease-in-out'}
            for t,y,r in [(0,0,0),(.09,1,-1),(.15545,-3,1),(.215,0,-1.8),
                         (.23727,1,1),(.29455,-2,-1),(.34364,0,1.5),(.4,1,-1),
                         (.49,0,0),(.90909,0,0),(1,0,0)]]),
        'tail-motion': ((344,439),rotations([(0,0),(.09,6),(.15545,-9),(.23727,7),
                                           (.29455,-8),(.37,5),(.46,-5),(.56,4),
                                           (.7,-3),(.90909,0),(1,0)])),
        'eyes-rest': (None,reference['.eyes-open']),
        'eyes-happy': (None,reference['.eyes-happy']),
        'blink': ((256,y),reference['.blink']),
    }
    if species!='cat': del tracks['tail-motion']
    if species=='owl':
        # Swing the utensil-bearing wing outwards, away from the tilting head.
        tracks['right-motion'] = ((336,355), rotations([
            (0,0),(.05,0),(.09,4),(.15545,18),(.23727,3),(.29455,14),
            (.36818,7),(.41727,5),(.48273,17),(.54818,5),(.61364,16),
            (.70364,9),(.90909,0),(1,0)]))
    css = f'/* {display}: 6.6-second two-hop celebration; plays once and returns to rest. */\n'
    for key, (origin, frames) in tracks.items():
        cls = prefix+'-'+key
        css += f'.{cls} {{ animation: {cls}-keys 6.6s linear 1 both;'
        if origin:
            css += f' transform-box: view-box; transform-origin: {origin[0]}px {origin[1]}px;'
        css += ' }\n' + motion_css(cls+'-keys', frames)
    css += '@media (prefers-reduced-motion: reduce) {\n'
    css += ',\n'.join('.'+prefix+'-'+k for k in tracks) + ' { animation: none !important; }\n'
    css += f'.{prefix}-eyes-rest {{ opacity: 1; }}\n.{prefix}-eyes-happy {{ opacity: 0; }}\n}}\n'
    style = node('style', text=css)
    svg.insert(3,style)
    OUTPUTS[ROOT/f'{name}-excited.svg'] = ET.tostring(svg,encoding='unicode')+'\n'
    return svg


def gallery(roots):
    cards=[]
    for root,(name,old,display,species) in zip(roots,CHARACTERS):
        cards.append(f'<article><div class="stage">{ET.tostring(root,encoding="unicode")}</div><div class="caption"><h2>{display}</h2><a href="{name}-excited.svg" download>Download SVG</a></div></article>')
    body='''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Marmalade, Nutmeg &amp; Bao — excited jump</title><style>
*{box-sizing:border-box}body{margin:0;background:#f7f2e9;color:#493428;font:14px/1.5 system-ui,sans-serif}main{max-width:1240px;margin:auto;padding:35px 34px 28px}.eyebrow{text-transform:uppercase;letter-spacing:.18em;font-size:10px;color:#967254;font-weight:700}header{display:flex;justify-content:space-between;align-items:center;gap:24px;margin-bottom:20px}h1{font:normal 38px/1.2 Georgia,serif;letter-spacing:-.025em;margin:8px 0 6px}header p{color:#8c7661;margin:0;font-size:12px}.controls{display:flex;gap:8px;flex-wrap:wrap;align-items:center}button{font:inherit;cursor:pointer;border-radius:9px;padding:9px 14px;color:#664631;border:1px solid #cbb59b;background:#fffaf2}button.primary{background:#a7613f;color:#fff8e9;border-color:#a7613f}button:disabled{opacity:.5;cursor:default}button:focus-visible,a:focus-visible,input:focus-visible{outline:3px solid #a7613f;outline-offset:3px}label{display:flex;gap:6px;align-items:center;font-size:12px;padding:0 4px}.grid{display:grid;grid-template-columns:repeat(3,1fr);gap:18px}article{border:1px solid #e2d5c4;border-radius:20px;background:#fffbf4;padding:9px 16px 15px}.stage svg{display:block;width:100%;height:auto}.caption{display:flex;justify-content:space-between;align-items:center;border-top:1px solid #ecdfce;padding-top:12px}h2{font:normal 25px Georgia,serif;margin:0}.caption a{font-size:11px;color:#925a3d;text-decoration:none}footer{display:flex;justify-content:space-between;gap:16px;font-size:11px;color:#8a725d;margin-top:18px}.note{font-size:12px;color:#8a725d}.note[hidden]{display:none}@media(max-width:750px){main{padding:22px 18px}header{align-items:start;flex-direction:column}.grid{grid-template-columns:1fr}h1{font-size:32px}article{max-width:440px;width:100%;margin:auto}footer{flex-direction:column}}
</style><main><header><div><div class="eyebrow">The little kitchen crew / selected mascots</div><h1>A little jump for joy.</h1><p>Two excited hops, a cheerful wave, and a soft landing.</p></div><div class="controls"><button class="primary" id="replay">Replay all</button><button id="pause" aria-pressed="false">Pause</button><label><input id="loop" type="checkbox"> Loop</label></div></header><p class="note" id="reduced" hidden>Reduced motion is on. Showing the resting poses.</p><div class="grid">'''+''.join(cards)+'''</div><footer><span>6.6 seconds · Transparent SVGs · Original selected designs</span><span>Motion inspired by Chef Penguini</span></footer></main><script>
const replay=document.querySelector('#replay'),pause=document.querySelector('#pause'),loop=document.querySelector('#loop');
const reduce=matchMedia('(prefers-reduced-motion: reduce)');
let all=[],paused=false;
const getAnimations=()=>Array.from(document.querySelectorAll('.stage svg')).flatMap(svg=>svg.getAnimations({subtree:true}));
function refresh(){all=getAnimations();document.querySelector('#reduced').hidden=!reduce.matches;replay.disabled=pause.disabled=loop.disabled=reduce.matches;}
function reset(){refresh();if(reduce.matches)return;paused=false;pause.textContent='Pause';pause.setAttribute('aria-pressed','false');for(const a of all){a.effect.updateTiming({iterations:loop.checked?Infinity:1});a.currentTime=0;a.play();}}
replay.addEventListener('click',reset);
pause.addEventListener('click',()=>{paused=!paused;for(const a of all)paused?a.pause():a.play();pause.textContent=paused?'Resume':'Pause';pause.setAttribute('aria-pressed',String(paused));});
loop.addEventListener('change',reset);
reduce.addEventListener('change',()=>{refresh();if(!reduce.matches)reset();});
refresh();
</script></html>'''
    OUTPUTS[ROOT/'preview.html'] = body


def combined(roots):
    svg=node('svg',{'viewBox':'0 0 1536 672','width':'1536','height':'672',
                    'role':'img','aria-labelledby':'crew-title crew-desc'})
    svg.append(node('title',{'id':'crew-title'},'Marmalade, Nutmeg and Bao — excited jumps'))
    svg.append(node('desc',{'id':'crew-desc'},'All three selected chef mascots do two hops and wave together. Plays once in 6.6 seconds and respects reduced motion.'))
    for i,root in enumerate(roots):
        # Nested SVG viewports give every rig its own coordinate system.
        item=copy.deepcopy(root)
        item.set('x',str(i*512))
        item.set('y','0')
        item.set('width','512')
        item.set('height','672')
        svg.append(item)
    text=ET.tostring(svg,encoding='unicode')
    OUTPUTS[ROOT/'kitchen-crew-excited.svg'] = text+'\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Fail if generated assets are out of date.")
    args = parser.parse_args()
    roots = [build(*character) for character in CHARACTERS]
    gallery(roots)
    combined(roots)
    stale = []
    for path, content in OUTPUTS.items():
        if args.check:
            if not path.exists() or path.read_text() != content:
                stale.append(str(path.relative_to(ROOT)))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
    if stale:
        print("Out-of-date kitchen crew assets: " + ", ".join(stale), file=sys.stderr)
        return 1
    print(f"{'Checked' if args.check else 'Built'} {len(OUTPUTS)} kitchen crew assets.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
