"""Build an offline review gallery, contact sheet and verified download archive."""
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
import json,zipfile,html,sys
R=Path(__file__).resolve().parent.parent
scenes=[('cozy-campsite', 'Cozy campsite', 'A canvas tent, fireside chairs, camp kitchen and soft string lights beneath the pines.'), ('night-market-lantern-courtyard', 'Night market courtyard', 'Noodles, bao, skewers and milk tea around shared tables and glowing paper lanterns.'), ('neighborhood-grocery-store', 'Neighborhood grocery', 'Seasonal produce, pantry staples, chilled drinks, fresh bread and a friendly corner checkout.')]
for key,_,_ in scenes:
 p=R/'previews'/f'{key}.png';assert Image.open(p).size==(1600,1400),p
 for ext in ['blend','glb']:assert (R/'models'/f'{key}.{ext}').stat().st_size>10000
font='/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
bold='/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
W,H=2200,1020;im=Image.new('RGB',(W,H),'#f1eee7');draw=ImageDraw.Draw(im)
draw.text((75,42),'NOMS / CAMP, GATHER, STOCK UP',font=ImageFont.truetype(bold,45),fill='#3f5147')
draw.text((77,105),'Three matte miniature scenes • Blender master models',font=ImageFont.truetype(font,25),fill='#746c60')
# Three scenes at equal scale for comparison.
positions=[(45,175,685,600),(755,175,685,600),(1465,175,685,600)]
for i,((key,title,desc),(x,y,w,h)) in enumerate(zip(scenes,positions),1):
 pic=Image.open(R/'previews'/f'{key}.png').convert('RGB');pic.thumbnail((w,h),Image.Resampling.LANCZOS);im.paste(pic,(x+(w-pic.width)//2,y))
 draw.text((x+12,y+h+16),f'{i:02}  {title}',font=ImageFont.truetype(bold,26 if i<=3 else 33),fill='#3f5147')
draw.text((65,925),'Editable .blend masters  /  Portable .glb exports  /  Named component groups',font=ImageFont.truetype(font,25),fill='#746c60')
im.save(R/'previews'/'collection.jpg',quality=94,optimize=True)
cards=[]
for i,(key,title,desc) in enumerate(scenes,1):
 cards.append(f'''<article><a href="previews/{key}.png"><img src="previews/{key}.png" alt="{html.escape(title)} — rendered 3D diorama" loading="lazy"></a><div class="content"><span class="number">{i:02}</span><h2>{html.escape(title)}</h2><p>{html.escape(desc)}</p><div class="links"><a download href="models/{key}.blend">Blender master</a><a download href="models/{key}.glb">GLB model</a><a href="previews/{key}.png">Full-size render ↗</a></div></div></article>''')
page='''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Noms — Camp, Gather, Stock Up</title><style>*{box-sizing:border-box}body{margin:0;background:#f1eee7;color:#343c35;font-family:system-ui,sans-serif}header,main,footer{max-width:1740px;margin:auto;padding:36px}header{padding-top:64px}small{font-weight:700;letter-spacing:.18em;color:#8b6951}h1{font-size:clamp(2.2rem,6vw,4.8rem);letter-spacing:-.045em;line-height:1.05;margin:20px 0}header p{max-width:650px;font-size:1.1rem;line-height:1.65;color:#706b61}main{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:30px;padding-top:12px}article{background:#faf8f3;border:1px solid #ddd8cc;border-radius:20px;overflow:hidden}img{display:block;width:100%;height:auto}.content{padding:24px 28px 30px}.number{color:#9e8069;font-size:.8rem;letter-spacing:.15em}h2{margin:8px 0 12px;font-size:1.6rem;letter-spacing:-.025em}p{line-height:1.6;margin:10px 0 20px;color:#746c60}.links{display:flex;gap:10px;flex-wrap:wrap}a{color:#476350}.links a{padding:9px 13px;border:1px solid #c5cebf;border-radius:8px;text-decoration:none;font-size:.9rem}.links a:hover{background:#e6ecdf}footer{font-size:.85rem;line-height:1.7;color:#746c60;padding-bottom:64px}@media(max-width:900px){header,main,footer{padding:24px}main{display:block}article{margin-bottom:24px}h1{margin-top:16px}.content{padding:20px}}</style><header><small>NOMS / MODEL COLLECTION</small><h1>Camp, gather,<br>stock up.</h1><p>A cozy campsite, a lantern-lit food courtyard and a neighborhood grocery store, with the softened forms, matte materials and natural lighting of the Noms dioramas.</p></header><main>'''+''.join(cards)+'''</main><footer>Static 3D review models with named assembly groups for future animation. The Blender masters include editable geometry and native lighting; GLB files include geometry and PBR materials. Click any image for the full-resolution render. This gallery works offline after unzipping the collection.<br><a href="README.md">Model notes and rebuild instructions</a></footer></html>'''
(R/'gallery.html').write_text(page)
if '--gallery-only' in sys.argv:
 print('Gallery and contact sheet generated');sys.exit(0)
dest=R/'dist'/'Noms-Campsite-Night-Market-Grocery.zip'
dest.parent.mkdir(exist_ok=True)
files=[R/'.gitignore',R/'README.md',R/'gallery.html',R/'validation.json',R/'native-validation.json',R/'import-validation.json']
for folder in ['models','previews','source']:files.extend(p for p in (R/folder).rglob('*')if p.is_file() and '__pycache__' not in str(p))
with zipfile.ZipFile(dest,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as z:
 for p in sorted(files):z.write(p,'Noms-Campsite-Night-Market-Grocery/'+str(p.relative_to(R)))
with zipfile.ZipFile(dest) as z:assert z.testzip() is None
print(json.dumps({'zip':str(dest),'bytes':dest.stat().st_size,'files':len(files),'gallery':str(R/'gallery.html'),'preview':str(R/'previews/collection.jpg')}))
