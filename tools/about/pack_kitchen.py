"""Quantize the single-mesh, vertex-colored Blender GLB (standard KHR_mesh_quantization)."""
import json,struct,sys
from pathlib import Path
out=Path(sys.argv[1]);path=out/'sams-kitchen.glb';raw=path.read_bytes();jl=struct.unpack_from('<I',raw,12)[0];d=json.loads(raw[20:20+jl]);binary=raw[28+jl:]
if d.get('extensionsRequired'):raise SystemExit('Expected unquantized Blender export')
def accessor(index):
 a=d['accessors'][index];v=d['bufferViews'][a['bufferView']];off=v.get('byteOffset',0)+a.get('byteOffset',0);w={'VEC3':3,'VEC4':4,'SCALAR':1}[a['type']];fmt={5126:'f',5125:'I',5123:'H',5121:'B'}[a['componentType']];values=list(struct.unpack_from('<'+fmt*(a['count']*w),binary,off));return [v/65535 for v in values]if a.get('normalized')and fmt=='H' else [v/255 for v in values]if a.get('normalized')and fmt=='B' else values
p=d['meshes'][0]['primitives'][0];positions=accessor(p['attributes']['POSITION']);normals=accessor(p['attributes']['NORMAL']);colors=accessor(p['attributes']['COLOR_0']);indices=accessor(p['indices']);mins=[min(positions[k::3])for k in range(3)];maxs=[max(positions[k::3])for k in range(3)];extent=max(b-a for a,b in zip(mins,maxs));count=len(positions)//3
qpos=[round((v-mins[i%3])/extent*65535)for i,v in enumerate(positions)];qnorm=[max(-32767,min(32767,round(v*32767)))for v in normals];qcolor=[max(0,min(255,round(v*255)))for v in colors]
buf=bytearray();views=[]
def append(data,target):
 while len(buf)%4:buf.append(0)
 views.append({'buffer':0,'byteOffset':len(buf),'byteLength':len(data),'target':target});buf.extend(data);return len(views)-1
pv=append(struct.pack('<'+'H'*len(qpos),*qpos),34962);nv=append(struct.pack('<'+'h'*len(qnorm),*qnorm),34962);cv=append(bytes(qcolor),34962);iv=append(struct.pack('<'+'I'*len(indices),*indices),34963)
accessors=[{'bufferView':pv,'componentType':5123,'count':count,'type':'VEC3','normalized':True,'min':[min(qpos[k::3])for k in range(3)],'max':[max(qpos[k::3])for k in range(3)]},{'bufferView':nv,'componentType':5122,'count':count,'type':'VEC3','normalized':True},{'bufferView':cv,'componentType':5121,'count':count,'type':'VEC3'if len(colors)==count*3 else 'VEC4','normalized':True},{'bufferView':iv,'componentType':5125,'count':len(indices),'type':'SCALAR'}]
doc={'asset':{'version':'2.0','generator':'Noms Blender web exporter'},'extensionsUsed':['KHR_mesh_quantization'],'extensionsRequired':['KHR_mesh_quantization'],'scene':0,'scenes':[{'nodes':[0]}],'nodes':[{'mesh':0,'name':"Sam's kitchen",'translation':mins,'scale':[extent]*3}],'meshes':[{'primitives':[{'attributes':{'POSITION':0,'NORMAL':1,'COLOR_0':2},'indices':3,'material':0,'mode':4}]}],'materials':[{'name':'Kitchen colors and soft contact shading','pbrMetallicRoughness':{'metallicFactor':0,'roughnessFactor':.65},'doubleSided':True}],'accessors':accessors,'bufferViews':views,'buffers':[{'byteLength':len(buf)}]}
js=json.dumps(doc,separators=(',',':')).encode();js+=b' '*((-len(js))%4);buf+=b'\0'*((-len(buf))%4);packed=struct.pack('<III',0x46546c67,2,12+8+len(js)+8+len(buf))+struct.pack('<II',len(js),0x4e4f534a)+js+struct.pack('<II',len(buf),0x004e4942)+buf;path.write_bytes(packed)
budget=json.loads((out/'kitchen-budget.json').read_text());budget.update(glb_bytes=len(packed),position_quantum=extent/65535,quantization='KHR_mesh_quantization',materials=1);(out/'kitchen-budget.json').write_text(json.dumps(budget,indent=2));print(budget)
