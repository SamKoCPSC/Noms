"""Draw the four village foliage cutouts. Run with Python + Pillow to regenerate the atlas."""
import math
import random
from pathlib import Path
from PIL import Image, ImageDraw

SIZE = 512
rng = random.Random(70251)
atlas = Image.new('RGBA', (2*SIZE, 2*SIZE))
palettes = {
    'blossom': ['#ae627c', '#d8819a', '#eea0b0', '#ffbac9', '#ffdbd7'],
    'fir': ['#244d41', '#326449', '#548258', '#75a55d', '#a3bb6a'],
    'maple': ['#ab5133', '#ca682f', '#e48b32', '#f5ae40', '#ffcf62'],
    'oak': ['#345b33', '#4b773c', '#689147', '#84aa4d', '#aac564'],
}

def shade(hex_color, factor):
    return tuple(min(255, round(int(hex_color[i:i+2],16)*factor)) for i in (1,3,5))

def leaf(base, angle, length, width, kind, tint):
    # Draw both sides of a folded, slightly asymmetric blade.
    dx, dy = math.cos(angle), math.sin(angle)
    def at(x,y):
        return (base[0]+dx*x*length-dy*y*width, base[1]+dy*x*length+dx*y*width)
    if kind == 'maple':
        edge = [(0,0),(.15,.28),(.12,.66),(.37,.55),(.53,1),(.57,.47),(.75,.6),(.73,.25),(1,0)]
    else:
        edge = [(0,0),(.14,.38),(.25,.73),(.32,.67),(.46,1),(.57,.9),(.62,.91),(.77,.56),(1,0)]
    for sign,light in [(-1,1.05),(1,.89)]:
        pts = [at(x,y*sign) for x,y in edge]
        draw.polygon(pts+[at(.48,.04)], fill=shade(tint,light))
    if length>19:
        draw.line([at(.08,0),at(.5,.03),at(.81,0)], fill=shade(tint,1.13),width=1)

def flower(x,y,r,tint):
    phase = rng.random()*math.tau
    for i in range(5):
        a = phase+i*math.tau/5
        dx,dy = math.cos(a)*r,math.sin(a)*r
        pts = [(x+dx*u-dy*v,y+dy*u+dx*v) for u,v in
               [(0,0),(.26,-.26),(.64,-.38),(.92,-.26),(1,-.09),(.88,.01),(1,.12),(.84,.32),(.52,.34),(.22,.17)]]
        draw.polygon(pts, fill=shade(tint,1.06-.07*math.sin(a)))
    draw.ellipse((x-r*.14,y-r*.14,x+r*.14,y+r*.14), fill='#efc982')

for idx,(kind,palette) in enumerate(palettes.items()):
    img = Image.new('RGBA',(SIZE,SIZE))
    draw = ImageDraw.Draw(img)
    shoots = []
    def shoot(start,end,bend):
        x,y = start
        dx,dy = end[0]-x,end[1]-y
        def point(t):
            return (x+dx*t+bend*math.sin(t*math.pi), y+dy*t)
        pts = [point(i/14) for i in range(15)]
        draw.line(pts, fill='#726246' if kind!='fir' else '#546547',width=3)
        shoots.append((point,math.atan2(dy,dx)))
    # Fan of twiglets, with air between the outermost tips.
    stem = [(248,500),(251,420),(241,340),(254,248),(249,165),(270,59)]
    draw.line(stem,fill='#685440',width=6)
    for j in range(9):
        t = j/9
        y = 450-t*340
        reach = (170+40*math.sin(t*math.pi))*(1-.64*t*t)
        for side in [-1,1]:
            shoot((249,y+rng.uniform(-8,8)),(250+side*reach*rng.uniform(.84,1.02), y-rng.uniform(64,105)),side*9)
    shoot((250,198),(267,29),-11)
    for point,angle in shoots:
        count = 14 if kind=='fir' else 7
        for n in range(count):
            t = (n+.45)/count
            x,y = point(t)
            for side in [-1,1]:
                a = angle+side*rng.uniform(.40,1.05)
                length = rng.uniform(21,46)*(1-.40*t)
                tint = rng.choice(palette[1:] if y<210 else palette)
                if kind=='blossom':
                    leaf((x,y),a,length*.85,8,'oak',tint)
                    for _ in range(3):
                        d=rng.random()*length
                        flower(x+math.cos(a)*d+rng.uniform(-8,8),y+math.sin(a)*d+rng.uniform(-8,8),rng.uniform(6,12),tint)
                elif kind=='fir':
                    # Every lateral spray has its own pairing of tapered needles.
                    tip=(x+math.cos(a)*length,y+math.sin(a)*length)
                    draw.line([(x,y),tip],fill=shade(tint,.87),width=3)
                    for k in range(7):
                        f=k/7
                        for sign in [-1,1]:
                            leaf((x+(tip[0]-x)*f,y+(tip[1]-y)*f),a+sign*.60,
                                 (13-6*f)*rng.uniform(.8,1.25),2.3,'needle',tint)
                else:
                    leaf((x,y),a,length*(1.16 if kind=='maple' else 1.03),length*(.42 if kind=='maple' else .27),kind,tint)
    atlas.paste(img,((idx%2)*SIZE,(idx//2)*SIZE))
atlas.save(Path(__file__).with_name('sprigs.png'), optimize=True)
