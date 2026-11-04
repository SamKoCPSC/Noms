"""Bake unlit surface data for the kitchen's live lighting (called by export_kitchen).
No lamps or other assemblies contribute to these maps. Local cavity AO is confined
to a rigid assembly; the independently unfolding room walls get no baked AO.
"""
import bpy, math
import numpy as np


def bake_surfaces(objects, scene, output):
    atlas_name = 'KitchenSurfaceAtlas'
    prints = {}
    for obj in objects:
        uv = obj.data.uv_layers.get('UVMap')
        if uv:
            prints[obj.name] = [tuple(v.uv) for v in uv.data]
        obj.data.uv_layers.active = obj.data.uv_layers.new(name=atlas_name)
    bpy.context.view_layer.objects.active = objects[0]
    bpy.ops.object.mode_set(mode='EDIT')
    bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.uv.smart_project(angle_limit=math.radians(66), island_margin=.004, margin_method='FRACTION')
    bpy.ops.object.mode_set(mode='OBJECT')
    scene.render.engine = 'CYCLES'
    scene.cycles.samples = 16
    scene.render.bake.target = 'IMAGE_TEXTURES'
    scene.render.bake.margin = 8
    scene.render.bake.margin_type = 'EXTEND'
    scene.render.bake.normal_space = 'OBJECT'
    materials = {m for o in objects for m in o.data.materials if m}
    targets = {}
    outputs = {}
    shaders = {}
    for mat in materials:
        tree = mat.node_tree
        # Tangent-space normal maps must keep their source UV basis after atlas unwrap.
        for node in tree.nodes:
            if node.type == 'NORMAL_MAP' and node.space == 'TANGENT' and not node.uv_map:
                color_links = node.inputs['Color'].links
                if color_links and color_links[0].from_node.type == 'TEX_IMAGE':
                    links = color_links[0].from_node.inputs['Vector'].links
                    if links and links[0].from_node.type == 'UVMAP':
                        node.uv_map = links[0].from_node.uv_map
        targets[mat] = tree.nodes.new('ShaderNodeTexImage')
        outputs[mat] = next(n for n in tree.nodes if n.type == 'OUTPUT_MATERIAL' and n.is_active_output)
        shaders[mat] = tree.nodes.get('Principled BSDF')

    def bake(name, kind='EMIT', skip_room=False):
        image = bpy.data.images.new(name, width=2048, height=2048, alpha=True, float_buffer=True)
        image.colorspace_settings.name = 'Non-Color'
        if skip_room:
            image.pixels.foreach_set(np.ones(2048*2048*4, dtype=np.float32))
        for mat, target in targets.items():
            target.image = image
            mat.node_tree.nodes.active = target
        for obj in objects:
            obj.select_set(not skip_room or obj.data.attributes['Assembly'].data[0].value != 1)
        bpy.context.view_layer.objects.active = next(o for o in objects if o.select_get())
        print('SURFACE_BAKE', name, flush=True)
        bpy.ops.object.bake(type=kind, uv_layer=atlas_name, use_clear=not skip_room)
        pixels = np.empty(2048*2048*4, dtype=np.float32)
        image.pixels.foreach_get(pixels)
        for obj in objects:
            obj.select_set(True)
        return pixels.reshape((-1, 4))

    # Keep the original normal/bump networks active for the object-space normal pass.
    normal = bake('Kitchen object normals', 'NORMAL')
    for mat in materials:
        tree = mat.node_tree
        emission = tree.nodes.new('ShaderNodeEmission')
        tree.links.new(emission.outputs[0], outputs[mat].inputs['Surface'])
        outputs[mat] = emission

    def channel(name):
        for mat in materials:
            tree = mat.node_tree
            socket = outputs[mat].inputs['Color']
            for link in list(socket.links):
                tree.links.remove(link)
            source = shaders[mat].inputs[name]
            if source.is_linked:
                tree.links.new(source.links[0].from_socket, socket)
            else:
                value = source.default_value
                socket.default_value = value if name == 'Base Color' else (value, value, value, 1.)
            # A conservative tinted-glass appearance for this opaque web derivative.
            if name == 'Base Color' and shaders[mat].inputs['Transmission Weight'].default_value > .1:
                socket.default_value = (.12, .20, .16, 1.)

    channel('Base Color')
    albedo = bake('Kitchen unlit material color')
    channel('Roughness')
    roughness = bake('Kitchen material roughness')
    for mat in materials:
        tree = mat.node_tree
        socket = outputs[mat].inputs['Color']
        for link in list(socket.links):
            tree.links.remove(link)
        ao = tree.nodes.new('ShaderNodeAmbientOcclusion')
        ao.only_local = True
        ao.samples = 16
        ao.inputs['Distance'].default_value = .045
        tree.links.new(ao.outputs['AO'], socket)
    cavity = bake('Kitchen assembly-local cavities', skip_room=True)
    # Save numerical data without a display transform. Normals remain in Blender
    # object coordinates; the renderer converts Z-up to glTF Y-up before lighting.
    rgb = np.maximum(albedo[:, :3], 0.)
    albedo[:, :3] = np.where(rgb <= .0031308, rgb*12.92, 1.055*np.power(rgb, 1./2.4)-.055)
    albedo[:, 3] = np.clip(cavity[:, 0], .25, 1.)
    normal[:, 3] = np.clip(roughness[:, 0], .08, 1.)
    scene.render.image_settings.file_format = 'PNG'
    scene.render.image_settings.color_mode = 'RGBA'
    scene.render.image_settings.color_depth = '8'
    for name, pixels in [('kitchen-surface-color', albedo), ('kitchen-surface-normal', normal)]:
        image = bpy.data.images.new(name, width=2048, height=2048, alpha=True, float_buffer=False)
        image.colorspace_settings.name = 'Non-Color'
        image.pixels.foreach_set(np.clip(pixels, 0., 1.).ravel())
        image.filepath_raw = str(output / (name+'.png'))
        image.file_format = 'PNG'
        image.save()
    for obj in objects:
        atlas = obj.data.uv_layers[atlas_name]
        for poly in obj.data.polygons:
            ti = obj.data.attributes['RecipeTexture'].data[poly.index].value
            if ti:
                for li in poly.loop_indices:
                    atlas.data[li].uv = prints[obj.name][li]
            else:
                obj.data.attributes['RecipeTexture'].data[poly.index].value = 3
        final_uv = [c for v in atlas.data for c in v.uv]
        while obj.data.uv_layers:
            obj.data.uv_layers.remove(obj.data.uv_layers[0])
        atlas = obj.data.uv_layers.new(name=atlas_name)
        atlas.data.foreach_set('uv', final_uv)
        obj.data.uv_layers.active = atlas
        atlas.active_render = True
    print('SURFACE_MATERIALS_BAKED', flush=True)
