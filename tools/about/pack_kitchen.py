"""Quantize the grouped, vertex-colored Blender GLB (standard KHR_mesh_quantization)."""
import json,struct,sys
from pathlib import Path
out=Path(sys.argv[1]);path=out/'sams-kitchen.glb';raw=path.read_bytes();jl=struct.unpack_from('<I',raw,12)[0];d=json.loads(raw[20:20+jl]);binary=raw[28+jl:]
if d.get('extensionsRequired'):raise SystemExit('Expected unquantized Blender export')
def accessor(index):
 a=d['accessors'][index];v=d['bufferViews'][a['bufferView']];off=v.get('byteOffset',0)+a.get('byteOffset',0);w={'VEC3':3,'VEC4':4,'SCALAR':1,'VEC2':2}[a['type']];fmt={5126:'f',5125:'I',5123:'H',5121:'B'}[a['componentType']];values=list(struct.unpack_from('<'+fmt*(a['count']*w),binary,off));return [v/65535 for v in values]if a.get('normalized')and fmt=='H' else [v/255 for v in values]if a.get('normalized')and fmt=='B' else values
buf=bytearray();views=[];accessors=[];primitives=[];materials=[]
def append(data,target):
 while len(buf)%4:buf.append(0)
 views.append({'buffer':0,'byteOffset':len(buf),'byteLength':len(data),'target':target});buf.extend(data);return len(views)-1
def attr(values,width,kind,fmt,normalized=False):
 vi=append(struct.pack('<'+fmt*len(values),*values),34963 if width==1 else 34962)
 a={'bufferView':vi,'componentType':kind,'count':len(values)//width,'type':{1:'SCALAR',2:'VEC2',3:'VEC3',4:'VEC4'}[width]}
 if normalized:a['normalized']=True
 accessors.append(a);return len(accessors)-1
ps=d['meshes'][0]['primitives'];allpos=[accessor(p['attributes']['POSITION']) for p in ps]
mins=[min(min(p[k::3]) for p in allpos) for k in range(3)];maxs=[max(max(p[k::3]) for p in allpos) for k in range(3)];extent=max(b-a for a,b in zip(mins,maxs))
for p,positions in zip(ps,allpos):
 normals=accessor(p['attributes']['NORMAL']);colors=accessor(p['attributes']['COLOR_0']);indices=accessor(p['indices']);count=len(positions)//3
 attributes={'POSITION':attr([round((v-mins[i%3])/extent*65535) for i,v in enumerate(positions)],3,5123,'H',True),'NORMAL':attr([max(-32767,min(32767,round(v*32767))) for v in normals],3,5122,'h',True),'COLOR_0':attr([max(0,min(255,round(v*255))) for v in colors],len(colors)//count,5121,'B',True)}
 metadata=d['materials'][p['material']]['extras']['assembly']
 if metadata['texture']:
  uv=accessor(p['attributes']['TEXCOORD_0']);attributes['TEXCOORD_0']=attr([max(0,min(65535,round(v*65535)))for v in uv],2,5123,'H',True)
 primitives.append({'attributes':attributes,'indices':attr(indices,1,5125,'I'),'mode':4,'material':len(materials)})
 materials.append({'name':metadata['name'],'extras':{'assembly':metadata},'pbrMetallicRoughness':{'metallicFactor':0,'roughnessFactor':.65},'doubleSided':True})
doc={'asset':{'version':'2.0','generator':'Noms Blender web exporter revision 13'},'extensionsUsed':['KHR_mesh_quantization'],'extensionsRequired':['KHR_mesh_quantization'],'scene':0,'scenes':[{'nodes':[0]}],'nodes':[{'mesh':0,'name':"Sam's recipe manager kitchen",'translation':mins,'scale':[extent]*3}],'meshes':[{'primitives':primitives}],'materials':materials,'accessors':accessors,'bufferViews':views,'buffers':[{'byteLength':len(buf)}]}
js=json.dumps(doc,separators=(',',':')).encode();js+=b' '*((-len(js))%4);buf+=b'\0'*((-len(buf))%4);packed=struct.pack('<III',0x46546c67,2,12+8+len(js)+8+len(buf))+struct.pack('<II',len(js),0x4e4f534a)+js+struct.pack('<II',len(buf),0x004e4942)+buf;path.write_bytes(packed)
budget=json.loads((out/'kitchen-budget.json').read_text());budget.update(glb_bytes=len(packed),position_quantum=extent/65535,quantization='KHR_mesh_quantization',materials=len(materials));(out/'kitchen-budget.json').write_text(json.dumps(budget,indent=2));print(budget)
