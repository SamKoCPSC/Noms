"""Recover static geometry/prints from the legacy GLB when no master is archived.

This does not claim to recover Blender shaders or original lighting. All vertex
colors, transforms and print UVs are retained; a documented softbox rig provides
new baked lighting. Never writes over the input GLB.
"""
import bpy, json, struct, math
import numpy as np
from pathlib import Path
from mathutils import Vector


def reconstruct(path, prints):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    data = path.read_bytes()
    size = struct.unpack_from('<I', data, 12)[0]
    doc = json.loads(data[20:20+size]); binary = data[28+size:]
    def accessor(index):
        a = doc['accessors'][index]; v = doc['bufferViews'][a['bufferView']]
        dtype = {5121:'u1',5122:'<i2',5123:'<u2',5125:'<u4',5126:'<f4'}[a['componentType']]
        width = {'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4}[a['type']]
        item = np.dtype(dtype).itemsize
        array = np.ndarray((a['count'],width), dtype=dtype, buffer=binary,
            offset=v.get('byteOffset',0)+a.get('byteOffset',0), strides=(v.get('byteStride',width*item),item)).copy()
        if a.get('normalized'):
            maximum = {5121:255,5122:32767,5123:65535}[a['componentType']]
            array = np.maximum(-1,array.astype(np.float32)/maximum)
        return array
    scene = bpy.context.scene
    group = bpy.data.objects.new('Recovered scene ROOT',None);scene.collection.objects.link(group);group['assembly']='Recovered geometry'
    node = doc['nodes'][0]; objects=[]
    for i,primitive in enumerate(doc['meshes'][0]['primitives']):
        attrs=primitive['attributes']; positions=accessor(attrs['POSITION']).astype(np.float32)
        positions=positions*np.array(node.get('scale',[1,1,1]))+np.array(node.get('translation',[0,0,0]))
        positions=positions[:,[0,2,1]]*np.array([1,-1,1])
        indices=accessor(primitive['indices']).reshape(-1,3)
        material=doc['materials'][primitive['material']];meta=material['extras']['assembly']
        mesh=bpy.data.meshes.new(meta['name']);mesh.from_pydata(positions.tolist(),[],indices.tolist());mesh.update()
        obj=bpy.data.objects.new(meta['name'],mesh);scene.collection.objects.link(obj);obj.parent=group;objects.append(obj)
        normals=accessor(attrs['NORMAL'])[:,[0,2,1]]*np.array([1,-1,1])
        mesh.normals_split_custom_set_from_vertices(normals.tolist())
        colors=accessor(attrs['COLOR_0'])
        if colors.shape[1]==3:colors=np.column_stack((colors,np.ones(len(colors))))
        color=mesh.color_attributes.new(name='SourceColor',type='FLOAT_COLOR',domain='CORNER')
        color.data.foreach_set('color',colors[indices.flatten()].flatten())
        mat=bpy.data.materials.new('Recovered '+str(i));mat.use_nodes=True;mesh.materials.append(mat)
        nodes=mat.node_tree.nodes;links=mat.node_tree.links;p=nodes.get('Principled BSDF')
        vc=nodes.new('ShaderNodeVertexColor');vc.layer_name='SourceColor';links.new(vc.outputs['Color'],p.inputs['Base Color'])
        p.inputs['Roughness'].default_value=.78
        if meta.get('opacity',1)<1:
            p.inputs['Transmission Weight'].default_value=1.;p.inputs['Roughness'].default_value=.12
            p.inputs['IOR'].default_value=1.45
        if meta['texture']:
            uv=accessor(attrs['TEXCOORD_0']);uv[:,1]=1-uv[:,1]
            layer=mesh.uv_layers.new(name='SourceUV')
            layer.data.foreach_set('uv',uv[indices.flatten()].flatten())
            texture=nodes.new('ShaderNodeTexImage');texture.image=bpy.data.images.load(str(path.parent/prints[meta['texture']-1]));texture.image.pack()
            links.new(texture.outputs['Color'],p.inputs['Base Color'])
    # The packed legacy export duplicates vertices at color/normal seams.
    # Weld those seams for UV unwrapping; corner colors and print UVs stay intact.
    for obj in objects:
        bpy.context.view_layer.objects.active=obj
        modifier=obj.modifiers.new('Restore connected topology','WELD');modifier.merge_threshold=1e-6
        bpy.ops.object.modifier_apply(modifier=modifier.name)
    bpy.context.view_layer.update()
    lo=Vector(tuple(min((o.matrix_world@Vector(v))[a] for o in objects for v in o.bound_box) for a in range(3)))
    hi=Vector(tuple(max((o.matrix_world@Vector(v))[a] for o in objects for v in o.bound_box) for a in range(3)))
    center=(lo+hi)/2;extent=max(hi.x-lo.x,hi.y-lo.y);unit=extent/8
    for name,loc,energy,size in [('Key',(-5,-6,10),1200,7),('Fill',(6,-2,7),650,6),('Rim',(1,5,9),900,5)]:
        light=bpy.data.lights.new(name,'AREA');light.energy=energy*unit**2;light.shape='DISK';light.size=size*unit
        obj=bpy.data.objects.new(name,light);scene.collection.objects.link(obj);obj.location=center+Vector(loc)*unit
        obj.rotation_euler=(center-obj.location).to_track_quat('-Z','Y').to_euler()
    scene.world=bpy.data.worlds.new('Recovered studio');scene.world.use_nodes=True
    scene.world.node_tree.nodes['Background'].inputs['Color'].default_value=(.72,.78,.85,1)
    scene.world.node_tree.nodes['Background'].inputs['Strength'].default_value=.35
    camera=bpy.data.objects.new('Review camera',bpy.data.cameras.new('Review camera'));scene.collection.objects.link(camera)
    camera.location=center+Vector((-10,-15,12) if 'office' in path.name else (10,-15,12))*unit;camera.rotation_euler=(center-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.type='ORTHO';camera.data.ortho_scale=extent*1.45;scene.camera=camera
    scene.view_settings.view_transform='AgX';scene.view_settings.look='AgX - Medium High Contrast';scene.view_settings.exposure=-.18
    return scene
