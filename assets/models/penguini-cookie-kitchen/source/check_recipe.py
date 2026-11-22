"""Validate recipe continuity in the open Blender master. Run with --python-exit-code 1."""
import math

import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree

scene = bpy.context.scene
objects = scene.objects


def at(seconds):
    scene.frame_set(round(seconds * scene.render.fps / scene.render.fps_base))


def visible(obj):
    return max(obj.matrix_world.to_scale()) > .005


tray = objects['Baking | parchment-lined tray']
cookies = [objects[f'Cookie {number} | portion'] for number in range(1, 4)]
book = objects['Reading | open recipe book']
mitt = objects['Oven | worn mitt']
marker_names = {marker.name for marker in scene.timeline_markers}
assert {'Welcome jump', 'Read recipe', 'Timer rings', 'Two cookies remain'} <= marker_names
assert any(strip.type == 'SOUND' and strip.sound.packed_file
           and strip.frame_start == scene.timeline_markers['Timer rings'].frame
           for strip in scene.sequence_editor.sequences)

# The glTF exporter can silently replace a non-finite bone rotation. Check the
# authored keys and exercise the actual rig against props on both sides.
rig = objects['Penguini rig']
for curve in rig.animation_data.action.fcurves:
    assert all(math.isfinite(key.co.y) for key in curve.keyframe_points), curve.data_path

for seconds, side, prop, offset in (
    (101.28, 'R', 'Cookie 3 | portion', (.10, .035, -.045)),
    (107.81, 'L', 'Baking | parchment-lined tray', (-.56, .41, .035)),
    (129.3, 'R', 'Cookie 1 | portion', (.035, 0, 0)),
):
    at(seconds)
    tip = rig.matrix_world @ rig.pose.bones['Foreflipper.' + side].tail
    target = objects[prop].matrix_world @ Vector(offset)
    assert (tip - target).length < .10, (seconds, prop)

# At each rear pickup the working shoulder is already the near one. Track the
# same palm through the moving door and the tray pickup.
hips = objects['Penguini | kitchen placement']
door = objects['Oven | hinged door']
handle = Vector((-.40, -.13, .81))
back_reaches = (
    ('L', door, handle, (*range(3093, 3115), *range(3252, 3273))),
    ('L', tray, (-.56, .41, .035), (3237,)),
)
for side, prop, offset, frames in back_reaches:
    opposite = 'L' if side == 'R' else 'R'
    for frame in frames:
        scene.frame_set(frame)
        target = prop.matrix_world @ Vector(offset)
        toward = (target - hips.matrix_world.translation).xy.normalized()
        forward = (hips.matrix_world.to_quaternion() @ Vector((0, -1, 0))).xy.normalized()
        near = rig.matrix_world @ rig.pose.bones['Flipper.' + side].head
        far = rig.matrix_world @ rig.pose.bones['Flipper.' + opposite].head
        palm = rig.matrix_world @ rig.pose.bones['Foreflipper.' + side].tail
        assert forward.dot(toward) > .60, (frame, prop.name, 'facing')
        assert (near - target).length + .05 < (far - target).length, (frame, prop.name, 'side')
        assert (palm - target).length < .14, (frame, prop.name, 'contact')

# During unloading the door, tray and footsteps retrace the loading action.
# Baking, the mitts and the timer advance on the normal clock.
mitts = {'L': objects['Oven | worn left mitt'], 'R': mitt}
for frame in range(3531, 3714):
    scene.frame_set(6804 - frame)
    loaded_tray = tray.matrix_world.copy()
    loaded_door = door.rotation_euler.x
    scene.frame_set(frame)
    assert max((tray.matrix_world[i] - loaded_tray[i]).length for i in range(4)) < .002, frame
    assert abs(door.rotation_euler.x - loaded_door) < .002, frame
    for side, glove in mitts.items():
        palm = rig.matrix_world @ rig.pose.bones['Foreflipper.' + side].tail
        elbow = rig.matrix_world @ rig.pose.bones['Foreflipper.' + side].head
        inside = glove.matrix_world.inverted() @ palm
        assert inside.xy.length < .025 and -.20 < inside.z < -.12, (side, frame, 'mitt fit')
        cuff_axis = glove.matrix_world.to_quaternion() @ Vector((0, 0, 1))
        assert cuff_axis.dot((elbow - palm).normalized()) > .98, (side, frame, 'cuff')
        if 3567 <= frame <= 3652:
            local = tray.matrix_world.inverted() @ palm
            # At the rack opening both mitts grasp the near short edge. Once
            # outside, the mitts move around the corners onto the long rims.
            near_front = abs(local.x + .83) < .09 and abs(local.y) < .67
            near_side = abs(abs(local.y) - .58) < .07 and -.95 < local.x < .2
            assert near_front or near_side, (side, frame, 'tray edge')
            assert .11 < local.z < .28, (side, frame, 'tray height')
    if 3532 <= frame <= 3553 or 3690 <= frame <= 3712:
        met = door.matrix_world @ handle
        # The insulated pad extends beyond the fingertips toward the handle.
        palm = rig.matrix_world @ rig.pose.bones['Foreflipper.L'].tail
        assert (palm - met).length < .20, (frame, 'oven handle')

# Check the skin at the tips in the padded palms, including the wrist turn when
# Penguini switches from carrying the tray to closing the oven.
scene.frame_set(0)
body = objects['Body • continuous shoulders and flippers']
depsgraph = bpy.context.evaluated_depsgraph_get()
bind = rig.matrix_world.inverted() @ body.matrix_world
for side, glove in mitts.items():
    bone = rig.data.bones['Foreflipper.' + side]
    axis = (bone.tail_local - bone.head_local).normalized()
    group = body.vertex_groups[bone.name].index
    reach = {v.index: (bind @ v.co - bone.head_local).dot(axis) for v in body.data.vertices
             if any(g.group == group and g.weight > .65 for g in v.groups)}
    limit = .91 * max(reach.values())
    tips = [index for index, distance in reach.items() if distance > limit]
    pad = next(part for part in glove.children if part.name.startswith('Mitt | palm'))
    surface = BVHTree.FromObject(pad, depsgraph)
    for frame in range(3531, 3714):
        scene.frame_set(frame)
        skin = body.evaluated_get(depsgraph)
        to_pad = pad.matrix_world.inverted() @ skin.matrix_world
        for index in tips:
            point = to_pad @ skin.data.vertices[index].co
            nearest, normal, _, _ = surface.find_nearest(point)
            assert (nearest - point).dot(normal) > .04, (side, frame, 'tip outside padding')

# Both flippers ease into their hanging cuffs before approaching the hot tray.
for side, pickup in (('L', 3509), ('R', 3512)):
    glove = mitts[side]
    scene.frame_set(pickup)
    hand = rig.pose.bones['Foreflipper.' + side]
    assert (rig.matrix_world @ hand.tail - glove.matrix_world @ Vector((0, .007, .118))).length < .035
    scene.frame_set(3474)
    previous = rig.matrix_world @ hand.tail
    for frame in range(3475, pickup + 1):
        scene.frame_set(frame)
        current = rig.matrix_world @ hand.tail
        assert (current - previous).length < .19, ('mitt pickup', side, frame)
        previous = current

# Check the deformed torso against the worktop during the stir and tray pickup. Flipper-to-prop contact is part of these gestures.
body = objects['Body • continuous shoulders and flippers']
torso_groups = {body.vertex_groups[name].index for name in ('Torso', 'Pelvis')}
torso_vertices = [v.index for v in body.data.vertices
                  if sum(g.weight for g in v.groups if g.group in torso_groups) > .9]
worktop = objects['Island oak worktop']
depsgraph = bpy.context.evaluated_depsgraph_get()
surface = BVHTree.FromObject(worktop, depsgraph)
for seconds in (86.533, 105.1):
    at(seconds)
    deformed = body.evaluated_get(depsgraph)
    to_worktop = worktop.matrix_world.inverted() @ deformed.matrix_world
    for index in torso_vertices:
        point = to_worktop @ deformed.data.vertices[index].co
        nearest, normal, _, _ = surface.find_nearest(point)
        assert (point - nearest).dot(normal) >= -.002, (seconds, index)

at(0)
assert not any(map(visible, cookies))
tray_home = tray.matrix_world.translation.copy()
book_home = book.matrix_world.translation.copy()
mitt_home = mitt.matrix_world.translation.copy()

# The whole book moves as one rigid assembly. Keep its collision mesh in book
# space so the raised lettering, pages and bookmark participate in the check.
book_vertices, book_faces = [], []
for part in book.children_recursive:
    if part.type != 'MESH':
        continue
    transform = book.matrix_world.inverted() @ part.matrix_world
    offset = len(book_vertices)
    book_vertices.extend(transform @ vertex.co for vertex in part.data.vertices)
    book_faces.extend(tuple(offset + i for i in face.vertices) for face in part.data.polygons)
book_surface = BVHTree.FromPolygons(book_vertices, book_faces)
book_bounds = [(min(v[i] for v in book_vertices), max(v[i] for v in book_vertices))
               for i in range(3)]
parts_to_clear = [objects[name] for name in (
    'Head • round charcoal plumage', 'Face • heart-shaped cream mask',
    'Beak • upper bill', 'Beak • lower bill', 'Apron bow loop.L', 'Apron bow loop.R',
)]


def assert_clear_of_book(parts, frame):
    to_book = book.matrix_world.inverted()
    for part in parts:
        mesh_object = part.evaluated_get(depsgraph)
        transform = to_book @ mesh_object.matrix_world
        corners = [transform @ Vector(corner) for corner in mesh_object.bound_box]
        if any(max(c[i] for c in corners) < lo or min(c[i] for c in corners) > hi
               for i, (lo, hi) in enumerate(book_bounds)):
            continue
        mesh = mesh_object.data
        surface = BVHTree.FromPolygons(
            [transform @ v.co for v in mesh.vertices],
            [face.vertices[:] for face in mesh.polygons],
        )
        assert not book_surface.overlap(surface), (frame, part.name)


read = scene.timeline_markers['Read recipe'].frame
espresso_start = scene.timeline_markers['Turn toward espresso powder'].frame
wet_stir = scene.timeline_markers['Stir the wet ingredients'].frame
wet_return = scene.timeline_markers['Return spoon after wet stir'].frame
final_mix = scene.timeline_markers['Finish mixing the dough'].frame
for frame in range(read - 12, read + 175):
    scene.frame_set(frame)
    assert_clear_of_book(parts_to_clear, frame)
    if 44 <= frame - read <= 123:
        for side, sign in (('L', 1), ('R', -1)):
            palm = rig.matrix_world @ rig.pose.bones['Foreflipper.' + side].tail
            edge = book.matrix_world @ Vector((sign * .704, -sign * .040, .028))
            assert (palm - edge).length < .025, (frame, side)


def collider(face_indices):
    vertices = sorted({i for face in face_indices for i in face})
    remap = {index: new for new, index in enumerate(vertices)}
    faces = [tuple(remap[i] for i in face) for face in face_indices]
    return vertices, faces


def transformed_surface(mesh, collider_data, transform):
    vertices, faces = collider_data
    return BVHTree.FromPolygons([transform @ mesh.vertices[i].co for i in vertices], faces)


bowl = objects['Terracotta mixing bowl']
bowl_geometry = collider([face.vertices[:] for face in bowl.data.polygons])
bowl_surface = transformed_surface(bowl.data, bowl_geometry, bowl.matrix_world)

chest = rig.pose.bones['Torso']
chest_bind_inverse = chest.bone.matrix_local.inverted()


def chest_world_transform():
    return rig.matrix_world @ chest.matrix @ chest_bind_inverse


def torso_lean_degrees():
    axis = chest_world_transform().to_3x3() @ Vector((0, 0, 1))
    return math.degrees(math.atan2(axis.xy.length, axis.z))


# From the landing camera, the pat should separate from the palm, cross the
# saucepan rim in view and come to rest inside it. Check across the range of
# camera distances used by the responsive fit.
pat = objects['Butter block']
pan = objects['Butter saucepan']
release = scene.timeline_markers['Release butter over the saucepan'].frame
land = scene.timeline_markers['Butter lands and starts melting'].frame
pickup = scene.timeline_markers['Pick up the butter'].frame
# The rig's L is Penguini's anatomical right, on the viewer's left when facing front.
right_palm = rig.pose.bones['Foreflipper.L']
left_palm = rig.pose.bones['Foreflipper.R']
for frame in range(pickup, release + 1):
    scene.frame_set(frame)
    assert torso_lean_degrees() < .01, (frame, 'upright butter pickup')
    palm = rig.matrix_world @ right_palm.tail
    free = rig.matrix_world @ left_palm.tail
    assert (palm - pat.matrix_world.translation).length < .14, (frame, 'right-hand butter grip')
    assert (free - pat.matrix_world.translation).length > .5, (frame, 'left flipper clear of pat')

azimuth, polar = math.radians(-10), math.radians(62)
direction = Vector((math.sin(azimuth) * math.sin(polar), -math.cos(azimuth) * math.sin(polar), math.cos(polar)))
heights = []
for frame in range(release, land + 1):
    scene.frame_set(frame)
    assert torso_lean_degrees() < .01, (frame, 'upright butter drop')
    center = pat.matrix_world.translation
    if frame == release:
        drop_xy = center.xy.copy()
    assert (center.xy - drop_xy).length < .012, (frame, 'sideways butter drop')
    heights.append(center.z)
    samples = [center, *[center.lerp(pat.matrix_world @ v.co, .86) for v in pat.data.vertices]]
    for distance in (12.5, 15.5):
        eye = Vector((0, 0, 2.2)) + direction * distance
        seen = 0
        for point in samples:
            ray = point - eye
            hit, _, _, _, obj, _ = scene.ray_cast(depsgraph, eye, ray.normalized(), distance=ray.length + .04)
            seen += int(hit and obj.name == pat.name)
        assert seen >= 3, (frame, 'pat obscured at saucepan rim', distance, seen)
rim_z = max((pan.matrix_world @ v.co).z for v in pan.data.vertices)
assert heights[0] > rim_z + .20 and heights[-1] < rim_z - .06
assert all(b <= a + .001 for a, b in zip(heights, heights[1:]))
assert (pat.matrix_world.translation - pan.matrix_world.translation).xy.length < .14
scene.frame_set(land + 58)
assert visible(objects['Butter | melting pool']) and pat.scale.z < .30

butter_pour = scene.timeline_markers['Pour melted butter into the bowl'].frame
scene.frame_set(butter_pour)
liquid = objects['Pan | toasted butter']
assert visible(liquid) and max(pat.scale) < .005
pan_lift = scene.timeline_markers['Lift the saucepan'].frame
pan_setdown = scene.timeline_markers['Set down the empty saucepan'].frame
for frame in range(pan_lift, pan_setdown + 1):
    scene.frame_set(frame)
    grip = objects['Saucepan | oak grip'].matrix_world.translation
    # The fingers ease away once the base has touched down.
    tolerance = .08 if frame >= pan_setdown - 2 else .025
    assert (rig.matrix_world @ right_palm.tail - grip).length < tolerance, (frame, 'right-hand saucepan grip')
    assert (rig.matrix_world @ left_palm.tail - grip).length > .5, (frame, 'left flipper clear of pan grip')
scene.frame_set(butter_pour + 12)
stream = [o for o in objects if o.name.startswith('Brown butter ribbon |') and visible(o)]
assert stream and all((o.matrix_world.translation - bowl.matrix_world.translation).xy.length < .46 for o in stream)
deposited_butter = objects['Bowl | melted butter on dough']
scene.frame_set(scene.timeline_markers['Set down the empty saucepan'].frame)
assert visible(deposited_butter) and not visible(liquid)
scene.frame_set(wet_return)
assert not visible(deposited_butter)

# The brown sugar cup is parked on the counter, then carried by the same
# flipper through the lift, tip and return. Falling clumps clear the rim.
cup = objects['Brown sugar | held measuring cup']
cup_meshes = [part for part in cup.children if part.type == 'MESH']
cup_geometry = [(part, collider([p.vertices[:] for p in part.data.polygons])) for part in cup_meshes]
scoop = objects['Brown sugar | measured contents']
deposit = objects['Brown sugar | sugar on dough']
pickup = scene.timeline_markers['Pick up brown sugar'].frame
pour = scene.timeline_markers['Pour brown sugar'].frame
returned = scene.timeline_markers['Set down brown sugar'].frame
assert land < read < pan_lift < butter_pour < pan_setdown < pickup < pour < returned
scene.frame_set(0)
cup_home = cup.matrix_world.copy()
worktop_z = max((worktop.matrix_world @ v.co).z for v in worktop.data.vertices)
shell = objects['Brown sugar | hollow cup']
shell_base = min((shell.matrix_world @ v.co).z for v in shell.data.vertices)
assert 0 <= shell_base - worktop_z < .003 and visible(scoop) and not visible(deposit)
flipper = rig.pose.bones[cup['working_flipper']]
group = body.vertex_groups[flipper.name].index
distal = {v.index for v in body.data.vertices if any(g.group == group and g.weight > .68 for g in v.groups)}
flipper_geometry = collider([p.vertices[:] for p in body.data.polygons if all(i in distal for i in p.vertices)])
face_parts = [(part, collider([p.vertices[:] for p in part.data.polygons])) for part in parts_to_clear[:4]]
left_upper = rig.pose.bones['Flipper.R']


def left_flipper_elevation():
    span = rig.matrix_world.to_3x3() @ (left_palm.tail - left_upper.head)
    return math.degrees(math.atan2(span.z, span.xy.length))


scene.frame_set(scene.timeline_markers['Add brown sugar'].frame)
rest_elevation = left_flipper_elevation()
for frame in range(pickup, returned + 1):
    scene.frame_set(frame)
    assert torso_lean_degrees() <= 10.01, (frame, 'sugar reach exceeds ten degrees')
    assert left_flipper_elevation() <= rest_elevation + 10, (frame, 'free flipper lifts too far')
    palm = rig.matrix_world @ flipper.tail
    skin = body.evaluated_get(depsgraph)
    hand_surface = transformed_surface(skin.data, flipper_geometry, skin.matrix_world)
    for part, geometry in face_parts:
        face = transformed_surface(part.data, geometry, part.matrix_world)
        assert not hand_surface.overlap(face), (frame, part.name, 'flipper crosses face')
    assert (palm - cup.matrix_world.translation).length < .015, (frame, 'sugar cup grip')
    for part, geometry in cup_geometry:
        surface = transformed_surface(part.data, geometry, part.matrix_world)
        assert not bowl_surface.overlap(surface), (frame, part.name, 'hits mixing bowl')
scene.frame_set(pour)
assert abs(torso_lean_degrees() - 10) < .01, 'ten-degree sugar pour'
sugar_lift = left_flipper_elevation() - rest_elevation
assert sugar_lift > 0, 'free flipper lifts as the sugar pours'
lip = cup.matrix_world @ Vector(cup['lip_local'])
assert (lip - bowl.matrix_world.translation).xy.length < .45 and lip.z > 2.20
clumps = [part for part in objects if part.name.startswith('Brown sugar | falling clump')]
assert clumps
for part in clumps:
    shown = [key.co.x for key in part.animation_data.action.fcurves.find('scale', index=0).keyframe_points
             if key.co.y > .005]
    positions = []
    for f in (min(shown), max(shown)):
        scene.frame_set(math.floor(f), subframe=f % 1)
        positions.append(part.matrix_world.translation.copy())
    assert (positions[1] - positions[0]).xy.length < .001 and positions[1].z < positions[0].z
for frame in (returned, espresso_start, scene.frame_end):
    scene.frame_set(frame)
    assert max((cup.matrix_world[i] - cup_home[i]).length for i in range(4)) < .002
    assert not visible(scoop)
# The parked cup sits outside the flippers' paths while all three cookies are shaped.
portion_start = scene.timeline_markers['Roll and place dough ball 1'].frame
portion_end = scene.timeline_markers['Open oven and load tray'].frame
for frame in range(portion_start, portion_end, 8):
    scene.frame_set(frame)
    moving_skin = BVHTree.FromObject(body, depsgraph)
    for part, geometry in cup_geometry:
        local = body.matrix_world.inverted() @ part.matrix_world
        assert not moving_skin.overlap(transformed_surface(part.data, geometry, local)), (frame, 'parked cup')

scene.frame_set(espresso_start)
assert visible(deposit)
scene.frame_set(wet_return)
assert not visible(deposit)

# The egg carries use the same shoulder-led, unbent reach as the sugar pour.
scene.frame_set(pour)
working_upper = rig.pose.bones['Flipper.L']
sugar_span = (rig.matrix_world.to_3x3() @ (right_palm.tail - working_upper.head)).length
sugar_elbow = right_palm.rotation_quaternion.copy()
previous = cup['setdown_frame']
for number in (1, 2):
    egg = objects[f'Whole egg {number}']
    halves = list(egg.children)
    yolk = objects[f'Bowl | egg yolk {number}']
    pickup, tap, opened, setdown = (egg[key + '_frame'] for key in ('pickup', 'tap', 'release', 'return'))
    assert previous < pickup < tap < opened < setdown < espresso_start
    previous = setdown
    shell_geometry = [(h, collider([p.vertices[:] for p in h.data.polygons])) for h in halves]
    scene.frame_set(0)
    assert len(halves) == 2 and all(abs(h.rotation_euler.y) < .001 for h in halves)
    assert not visible(yolk)
    initial_base = min((h.matrix_world @ v.co).z for h in halves for v in h.data.vertices)
    assert 0 <= initial_base - worktop_z < .008
    for frame in range(pickup, setdown + 1):
        scene.frame_set(frame)
        assert torso_lean_degrees() <= 10.01, (frame, 'egg reach exceeds sugar lean')
        span = (rig.matrix_world.to_3x3() @ (right_palm.tail - working_upper.head)).length
        assert abs(span - sugar_span) < .003, (frame, 'egg reach stretches flipper')
        assert abs(right_palm.rotation_quaternion.dot(sugar_elbow)) > .9999, (frame, 'egg reach bends elbow')
        grip = egg.matrix_world @ Vector(egg['grip_local'])
        assert (rig.matrix_world @ right_palm.tail - grip).length < .035, (frame, 'egg grip')
        assert (rig.matrix_world @ left_palm.tail - grip).length > .28
        skin = body.evaluated_get(depsgraph)
        hand = transformed_surface(skin.data, flipper_geometry, skin.matrix_world)
        assert not hand.overlap(bowl_surface), (frame, 'flipper inside bowl')
        for part, geometry in face_parts:
            assert not hand.overlap(transformed_surface(part.data, geometry, part.matrix_world)), (frame, part.name)
        assert_clear_of_book(halves, frame)
        for part, geometry in cup_geometry:
            assert not hand.overlap(transformed_surface(part.data, geometry, part.matrix_world)), (frame, 'sugar cup')
        if tap - 4 <= frame <= tap + 4:
            underside = [h.matrix_world @ v.co for h in halves for v in h.data.vertices if v.co.z < -.13]
            distance = min(bowl_surface.find_nearest(p)[3] for p in underside)
            if frame == tap:
                assert distance < .012, (frame, 'missed rim tap', distance)
                assert all(abs(h.rotation_euler.y) < .01 for h in halves)
            else:
                assert distance > .015, (frame, 'shell lingers at rim', distance)
    resting = min((h.matrix_world @ v.co).z for h in halves for v in h.data.vertices)
    assert 0 <= resting - worktop_z < .008
    # The empty shell and withdrawing hand both pass the parked sugar cup.
    for frame in range(setdown - 18, setdown + 15):
        scene.frame_set(frame)
        skin = body.evaluated_get(depsgraph)
        hand = transformed_surface(skin.data, flipper_geometry, skin.matrix_world)
        for part, geometry in cup_geometry:
            assert not hand.overlap(transformed_surface(part.data, geometry, part.matrix_world)), (frame, 'shell setdown')
        for half, geometry in shell_geometry:
            assert not bowl_surface.overlap(transformed_surface(half.data, geometry, half.matrix_world)), (frame, 'shell return')
    scene.frame_set(opened + 3)
    assert visible(yolk) and all(abs(h.rotation_euler.y) > .70 for h in halves)
    assert 4 < torso_lean_degrees() <= 10.01, (number, 'slight lean during release')
    falling = yolk.matrix_world.translation.copy()
    scene.frame_set(espresso_start)
    settled = yolk.matrix_world.translation
    assert visible(yolk) and falling.z > settled.z + .18
    assert (settled.xy - bowl.matrix_world.translation.xy).length < .26
    for distance in (12.5, 15.5):
        eye = Vector((0, 0, 2.2)) + direction * distance
        hit, _, _, _, obj, _ = scene.ray_cast(depsgraph, eye, (settled - eye).normalized())
        assert hit and obj == yolk, (number, 'settled yolk hidden from landing camera')
    scene.frame_set(wet_return)
    assert not visible(yolk)


# The ingredients on the tray side of the bowl are handled by the other flipper.
left_group = body.vertex_groups[left_palm.name].index
left_vertices = {v.index for v in body.data.vertices if any(g.group == left_group and g.weight > .68 for g in v.groups)}
left_geometry = collider([p.vertices[:] for p in body.data.polygons if all(i in left_vertices for i in p.vertices)])
previous = objects['Whole egg 2']['return_frame']
for label, name in (('Vanilla', 'Vanilla extract | bottle'), ('Baking soda', 'Baking soda | orange carton')):
    vessel = objects[name]
    pickup, pour_start, pour_end, setdown = (vessel[k + '_frame'] for k in ('pickup', 'pour', 'end_pour', 'setdown'))
    assert previous < pickup < pour_start < pour_end < setdown < espresso_start
    minimum_setdown = 54 if label == 'Vanilla' else 44
    assert setdown - pour_end >= minimum_setdown, (name, 'rushed set-down')
    previous = setdown
    scene.frame_set(0)
    home = vessel.matrix_world.copy()
    assert home.translation.x > bowl.matrix_world.translation.x
    vessel_geometry = collider([p.vertices[:] for p in vessel.data.polygons])
    for frame in range(pickup, setdown + 1):
        scene.frame_set(frame)
        grip = vessel.matrix_world @ Vector(vessel['grip_local'])
        assert (rig.matrix_world @ left_palm.tail - grip).length < .02, (name, frame, 'left grip')
        assert (rig.matrix_world @ right_palm.tail - grip).length > .45
        assert torso_lean_degrees() < 10.05
        assert abs(left_palm.rotation_quaternion.w) > .999, (name, frame, 'elbow twist')
        skin = body.evaluated_get(depsgraph)
        hand = transformed_surface(skin.data, left_geometry, skin.matrix_world)
        container = transformed_surface(vessel.data, vessel_geometry, vessel.matrix_world)
        assert not bowl_surface.overlap(container) and not bowl_surface.overlap(hand), (name, frame, 'bowl clearance')
        for part, geometry in face_parts:
            face = transformed_surface(part.data, geometry, part.matrix_world)
            assert not face.overlap(hand) and not face.overlap(container), (name, frame, 'face clearance')
    scene.frame_set(pour_start + 3)
    lip = vessel.matrix_world @ Vector(vessel['lip_local'])
    assert (lip.xy - bowl.matrix_world.translation.xy).length < .43 and lip.z > 2.22
    assert 4 < torso_lean_degrees() < 10.05
    stream = [o for o in objects if o.name.startswith(label + ' | falling ') and visible(o)]
    assert stream and all(o.matrix_world.translation.z <= lip.z + .06 for o in stream)
    scene.frame_set(espresso_start)
    assert max((vessel.matrix_world[i] - home[i]).length for i in range(4)) < .002
    assert visible(objects[label + ' | on dough'])
    scene.frame_set(wet_return)
    assert not visible(objects[label + ' | on dough'])


# The closed jar travels as one piece. The spare flipper twists and holds
# the lid while the working flipper shakes three pinches over the bowl.
jar = objects['Espresso | jar']
lid = objects['Espresso | screw lid']
stages = [scene.timeline_markers[name].frame for name in (
    'Pick up espresso jar', 'Bring espresso to the bowl', 'Unscrew espresso lid',
    'Lift espresso lid', 'Shake espresso powder', 'Finish shaking espresso',
    'Reseat espresso lid', 'Close espresso jar', 'Set down espresso jar',
)]
assert previous < espresso_start < stages[0] and stages == sorted(set(stages)) and stages[-1] < wet_stir
take, arrive, unscrew, lift, shake, stop, reseat, close, park = stages
scene.frame_set(0)
jar_home = jar.matrix_world.copy()
lid_home = jar_home.inverted() @ lid.matrix_world
counter = objects['Left cream stone countertop']
jar_center = counter.matrix_world.inverted() @ jar_home.translation
for axis in (0, 1):
    assert min(p[axis] for p in counter.bound_box) < jar_center[axis] < max(p[axis] for p in counter.bound_box)
assert abs(jar_center.z - max(p[2] for p in counter.bound_box)) * counter.matrix_world.to_scale().z < .01
jar_grip = Vector(jar['grip_local'])
lid_grip = Vector(lid['grip_local'])
powder = [part for part in objects if part.name.startswith('Espresso | falling grounds ')]
espresso_shell = objects['Espresso | amber jar']
espresso_geometry = collider([face.vertices[:] for face in espresso_shell.data.polygons])
# Include the upper flippers: the palms can reach the cap while the elbows cross the cotton straps.
espresso_wings = []
for side in 'LR':
    groups = {body.vertex_groups[name + '.' + side].index for name in ('Flipper', 'Foreflipper')}
    vertices = {v.index for v in body.data.vertices if sum(g.weight for g in v.groups if g.group in groups) > .70}
    espresso_wings.append(collider([p.vertices[:] for p in body.data.polygons if all(i in vertices for i in p.vertices)]))
straps = [objects['Apron strap.' + side] for side in 'LR']
head = objects['Head • rotate here']
up = Vector((0, 0, 1))
for frame in range(take, park + 1):
    scene.frame_set(frame)
    assert (jar.matrix_world @ jar_grip - rig.matrix_world @ right_palm.tail).length < .02, (frame, 'jar grip')
    vessel = transformed_surface(espresso_shell.data, espresso_geometry, espresso_shell.matrix_world)
    assert not vessel.overlap(bowl_surface), (frame, 'espresso jar hits bowl')
    skin = body.evaluated_get(depsgraph)
    hands = [transformed_surface(skin.data, mesh, skin.matrix_world) for mesh in (flipper_geometry, left_geometry)]
    for part, geometry in face_parts:
        face = transformed_surface(part.data, geometry, part.matrix_world)
        assert not any(hand.overlap(face) for hand in hands), (frame, 'espresso reach crosses face')
    if arrive <= frame <= close:
        assert (head.matrix_world.to_quaternion() @ up).dot(up) > math.cos(math.radians(.3)), (frame, 'tilted head at jar')
    for strap, wing in zip(straps, espresso_wings):
        to_strap = strap.matrix_world.inverted() @ skin.matrix_world
        flipper_surface = transformed_surface(skin.data, wing, to_strap)
        assert not BVHTree.FromObject(strap, depsgraph).overlap(flipper_surface), (frame, strap.name, 'flipper crosses strap')
    if unscrew <= frame <= close:
        assert (lid.matrix_world @ lid_grip - rig.matrix_world @ left_palm.tail).length < .02, (frame, 'lid grip')
    relative = jar.matrix_world.inverted() @ lid.matrix_world
    if frame <= unscrew or frame >= close:
        assert max((relative[i] - lid_home[i]).length for i in range(4)) < .003, (frame, 'loose lid')
    if frame == lift:
        assert relative.translation.z > lid_home.translation.z + .03
        assert relative.to_quaternion().rotation_difference(lid_home.to_quaternion()).angle > math.radians(100)
    if frame in (shake + 4, shake + 13, shake + 22):
        lip = jar.matrix_world @ Vector(jar['lip_local'])
        assert (lip.xy - bowl.matrix_world.translation.xy).length < .45
        assert (lid.matrix_world.translation - lip).length > .3
        assert any(visible(piece) and .02 < lip.z - piece.matrix_world.translation.z < .40
                   and (piece.matrix_world.translation.xy - lip.xy).length < .12 for piece in powder)
for frame in (park, wet_stir, scene.frame_end):
    scene.frame_set(frame)
    assert max((jar.matrix_world[i] - jar_home[i]).length for i in range(4)) < .003
scene.frame_set(wet_stir)
assert visible(objects['Espresso | on mixture'])
scene.frame_set(wet_return)
assert not visible(objects['Espresso | on mixture'])


# Each dry ingredient starts on the back counter and returns there after its pour.
rear_counter = objects['Left cream stone countertop']
counter_corners = [rear_counter.matrix_world @ Vector(p) for p in rear_counter.bound_box]
counter_top = max(p.z for p in counter_corners)
previous = wet_return
for name, mesh_name in (('Pantry | Flour', 'Soft flour sack'), ('Pantry | Chocolate chips', 'Chip crock')):
    vessel, shell = objects[name], objects[mesh_name]
    pickup, tipped, poured, setdown = (vessel[k + '_frame'] for k in ('pickup', 'pour', 'end_pour', 'setdown'))
    assert previous < pickup < tipped < poured < setdown < final_mix
    previous = setdown
    shell_geometry = collider([p.vertices[:] for p in shell.data.polygons])
    scene.frame_set(0)
    home = vessel.matrix_world.copy()
    corners = [shell.matrix_world @ Vector(corner) for corner in shell.bound_box]
    for axis in ('x', 'y'):
        limits = [getattr(p, axis) for p in counter_corners]
        assert all(min(limits) < getattr(p, axis) < max(limits) for p in corners), (name, 'counter footprint')
    assert abs(min(p.z for p in corners) - counter_top) < .008, (name, 'counter height')
    center = sum(corners, Vector()) / len(corners)
    for distance in (12.5, 15.5):
        eye = Vector((0, 0, 2.2)) + direction * distance
        hit, _, _, _, first, _ = scene.ray_cast(depsgraph, eye, (center - eye).normalized())
        assert hit and first in vessel.children, (name, 'hidden in opening shot')
    for frame in (pickup, setdown):
        scene.frame_set(frame)
        toward = (vessel.matrix_world.translation - hips.matrix_world.translation).xy.normalized()
        forward = (hips.matrix_world.to_quaternion() @ Vector((0, -1, 0))).xy.normalized()
        gaze = (objects['Beak • upper bill'].matrix_world.translation
                - objects['Head • round charcoal plumage'].matrix_world.translation).xy.normalized()
        assert forward.dot(toward) > .45 and gaze.dot(toward) > .80, (name, frame, 'facing ingredient')
    for frame in range(pickup, setdown + 1):
        scene.frame_set(frame)
        grip = vessel.matrix_world @ Vector(vessel['grip_local'])
        assert (rig.matrix_world @ right_palm.tail - grip).length < .02, (name, frame, 'right grip')
        assert (rig.matrix_world @ left_palm.tail - grip).length > .35
        assert torso_lean_degrees() < 10.05
        skin = body.evaluated_get(depsgraph)
        hand = transformed_surface(skin.data, flipper_geometry, skin.matrix_world)
        container = transformed_surface(shell.data, shell_geometry, shell.matrix_world)
        assert not bowl_surface.overlap(hand) and not bowl_surface.overlap(container), (name, frame, 'bowl clearance')
        for part, geometry in face_parts:
            face = transformed_surface(part.data, geometry, part.matrix_world)
            assert not face.overlap(hand) and not face.overlap(container), (name, frame, 'face clearance')
    # The free flipper rises from its carrying pose while the container tips.
    scene.frame_set(tipped - 17)
    carrying_elevation = left_flipper_elevation()
    assert torso_lean_degrees() < 4
    for frame in (tipped, poured):
        scene.frame_set(frame)
        assert abs(torso_lean_degrees() - 10) < .1, (name, 'pour lean')
        assert abs(left_flipper_elevation() - carrying_elevation - sugar_lift) < .5, (name, 'counterbalance')
        # The raised tip must remain visible past his face from the responsive landing camera.
        tip = rig.matrix_world @ left_palm.tail
        for distance in (12.5, 15.5):
            eye = Vector((0, 0, 2.2)) + direction * distance
            hit, point, _, _, first, _ = scene.ray_cast(depsgraph, eye, (tip - eye).normalized())
            assert hit and first == body and (point - tip).length < .15, (name, frame, 'hidden free flipper')
    scene.frame_set(tipped + 3)
    lip = vessel.matrix_world @ Vector(vessel['lip_local'])
    assert (lip.xy - bowl.matrix_world.translation.xy).length < .43 and lip.z > 2.22
    for frame in (setdown, final_mix, scene.frame_end):
        scene.frame_set(frame)
        assert max((vessel.matrix_world[i] - home[i]).length for i in range(4)) < .002, (name, frame, 'returned to counter')

# On each trip to the pantry, the torso rocks toward the supporting foot.
feet = [objects[f'Foot.{side} • integrated ankle'] for side in 'LR']
scene.frame_set(0)
floor = [foot.matrix_world.translation.z for foot in feet]
flour, chocolate = objects['Pantry | Flour'], objects['Pantry | Chocolate chips']
# Stop sampling when he returns to his starting spot. The next walk leads to the spoon.
scene.frame_set(espresso_start)
espresso_home = hips.matrix_world.translation.copy()
for back_at_bowl in range(park, wet_stir):
    scene.frame_set(back_at_bowl)
    if (hips.matrix_world.translation - espresso_home).length < .025:
        break
else:
    raise AssertionError('Penguini did not return from the espresso counter')
# From the book to the stove, one sole stays planted through each swing.
# Check both an ankle and a toe point to catch feet that spin in place.
departure = scene.timeline_markers['Turn to the saucepan'].frame
previous_contacts = None
swings = []
for frame in range(departure + 5, pan_lift - 9):
    scene.frame_set(frame)
    contacts = [(foot.matrix_world.translation.copy(),
                 foot.matrix_world @ Vector((0, -.25, -.10))) for foot in feet]
    if previous_contacts:
        slip = [max((a - b).length for a, b in zip(now, old))
                for now, old in zip(contacts, previous_contacts)]
        assert min(slip) < .003, (frame, 'saucepan walk foot slides')
    previous_contacts = contacts
    height = [foot.matrix_world.translation.z - ground for foot, ground in zip(feet, floor)]
    if max(height) > .095:
        swing = height.index(max(height))
        if not swings or swings[-1] != swing:
            swings.append(swing)
assert len(swings) >= 6, 'expected alternating strides between the book and stove'

for begin, end, min_sway, max_sway in (
    (scene.timeline_markers['Turn to the saucepan'].frame, pan_lift - 3, 5, 12),
    (pan_lift + 8, butter_pour - 22, 1.5, 6),
    (espresso_start, take, 1, 4), (take, arrive, 1, 4), (close, park, 1, 4), (park, back_at_bowl, 1, 4),
    (scene.timeline_markers['Turn toward flour and chocolate chips'].frame, flour['pickup_frame'], .7, 3),
    (flour['end_pour_frame'], chocolate['pickup_frame'], .7, 3),
):
    roll_at_midstep = [[], []]
    for frame in range(begin + 1, end):
        scene.frame_set(frame)
        lifted = [foot.matrix_world.translation.z - rest for foot, rest in zip(feet, floor)]
        if abs(lifted[0] - lifted[1]) < .065:
            continue
        axis = chest_world_transform().to_3x3() @ Vector((0, 0, 1))
        local = hips.matrix_world.to_quaternion().inverted() @ axis
        roll_at_midstep[lifted.index(max(lifted))].append(math.degrees(math.atan2(local.x, local.z)))
    toward_right, toward_left = roll_at_midstep
    assert toward_right and toward_left
    sway = sum(toward_right) / len(toward_right) - sum(toward_left) / len(toward_left)
    assert min_sway < sway < max_sway, (begin, end, 'turn waddle', sway)

chips = [o for o in objects if o.name == 'Chocolate chip' or o.name.startswith('Chocolate chip.')]
assert len(chips) >= 20
scene.frame_set(0)
crock = objects['Pantry | Chocolate chips']
assert all((c.matrix_world.translation.xy - crock.matrix_world.translation.xy).length < .24 for c in chips)
# Check the fall itself as well as the settled result.
for vessel, pieces in (
    (objects['Pantry | Flour'], [o for o in objects if o.name.startswith('Flour | falling ')]),
    (crock, chips),
):
    scene.frame_set((vessel['pour_frame'] + vessel['end_pour_frame']) // 2)
    mouth = vessel.matrix_world @ Vector(vessel['lip_local'])
    assert any(visible(piece)
               and .03 < mouth.z - piece.matrix_world.translation.z < .24
               and (piece.matrix_world.translation.xy - mouth.xy).length < .16
               for piece in pieces), (vessel.name, 'no ingredient falling from lip')

scene.frame_set(final_mix)
assert all(visible(c) and (c.matrix_world.translation.xy - bowl.matrix_world.translation.xy).length < .45 for c in chips)
assert visible(objects['Flour | mound on dough']) and not visible(objects['Flour in open sack'])
scene.frame_set(scene.timeline_markers['Set mixing spoon down'].frame)
assert not visible(objects['Flour | mound on dough'])
# Cast onto the dough between scoops. Pieces fully submerged in the mixture
# do not count, even if their mesh has a nonzero scale.
for frame in (scene.frame_current,
              scene.timeline_markers['Roll and place dough ball 2'].frame - 2,
              scene.timeline_markers['Roll and place dough ball 3'].frame - 2):
    scene.frame_set(frame)
    exposed = 0
    for chip in chips:
        point = chip.matrix_world.translation.copy()
        point.z = bowl.matrix_world.translation.z + .65
        hit, _, _, _, surface, _ = scene.ray_cast(depsgraph, point, Vector((0, 0, -1)))
        exposed += int(hit and surface == chip)
    assert exposed >= len(chips) // 3, (frame, 'chips at dough surface', exposed)
scene.frame_set(scene.timeline_markers['Roll and place dough ball 3'].frame + 50)
assert not any(visible(chip) for chip in chips)


# The spoon is a persistent prop. The hand meets it on the worktop and
# keeps the same grip through the lift, stir and return.
spoon = objects['Wooden stirring spoon']
grip = Vector(spoon['grip_local'])
pickups = list(spoon['pickup_frames'])
returns = list(spoon['setdown_frames'])
assert len(pickups) == len(returns) == 2
assert wet_stir < pickups[0] < returns[0] == wet_return
assert chocolate['setdown_frame'] < final_mix < pickups[1] < returns[1]
props = bpy.data.collections['Recipe ingredients | unanimated props']
assert {'Brown sugar ramekin'} <= set(props.objects.keys())
assert props not in list(scene.collection.children_recursive)
assert all(part.name not in objects and not part.animation_data for part in props.all_objects)

# Check the turns from the welcome pose to the book and from the book to the spoon.
for start, end in ((scene.timeline_markers['Turn toward the book'].frame, read + 20),
                   (pickups[0] - 44, pickups[0] - 8)):
    previous_turn = None
    for frame in range(start, end + 1):
        scene.frame_set(frame)
        turn = hips.matrix_world.to_quaternion()
        hands = [rig.matrix_world @ rig.pose.bones['Foreflipper.' + side].tail for side in 'LR']
        if previous_turn is not None:
            assert abs(previous_turn.dot(turn)) > math.cos(math.radians(12) / 2), (frame, 'abrupt turn')
            assert all((a - b).length < .36 for a, b in zip(hands, previous_hands)), (frame, 'abrupt release')
        previous_turn, previous_hands = turn, hands

scene.frame_set(0)
rest = spoon.matrix_world.copy()
worktop_z = max((worktop.matrix_world @ v.co).z for v in worktop.data.vertices)
spoon_base = min((part.matrix_world @ v.co).z for part in spoon.children for v in part.data.vertices)
assert 0 <= spoon_base - worktop_z < .01
tray_base = objects['Baking tray base']
tray_points = [tray_base.matrix_world @ v.co for v in tray_base.data.vertices]
rear_edge = max(p.y for p in tray_points)
counter_edge = max((worktop.matrix_world @ v.co).y for v in worktop.data.vertices)
spoon_points = [part.matrix_world @ v.co for part in spoon.children for v in part.data.vertices]
assert all(rear_edge < p.y < counter_edge for p in spoon_points), 'spoon behind tray'
tray_left, tray_right = min(v.x for v in tray_points), max(v.x for v in tray_points)
assert all(tray_left < p.x < tray_right for p in spoon_points), 'spoon aligned with tray'
spoon_parts = {part: collider([f.vertices[:] for f in part.data.polygons]) for part in spoon.children}
for start, end in zip([0, *returns], [*pickups, scene.frame_end]):
    assert start < end
    for frame in {start, end, (start + end) // 2}:
        scene.frame_set(frame)
        assert all(visible(part) for part in spoon_parts), (frame, 'hidden spoon')
        assert max((spoon.matrix_world[i] - rest[i]).length for i in range(4)) < .002
for pickup, returned in zip(pickups, returns):
    assert pickup < returned
    highest = rest.translation.z
    previous_grip = rest @ grip
    for frame in range(pickup, returned + 1):
        scene.frame_set(frame)
        palm = rig.matrix_world @ rig.pose.bones['Foreflipper.R'].tail
        handle = spoon.matrix_world @ grip
        assert (palm - handle).length < .02, (frame, 'spoon grip')
        assert (handle - previous_grip).length < .23, (frame, 'spoon jump')
        previous_grip = handle
        highest = max(highest, spoon.matrix_world.translation.z)
        for part, geometry in spoon_parts.items():
            surface = transformed_surface(part.data, geometry, part.matrix_world)
            assert not bowl_surface.overlap(surface), (frame, part.name, 'hits bowl')
    assert highest > rest.translation.z + .60

# The tray clears the resting spoon in both directions on its way to the oven.
scene.frame_set(0)
parked = [transformed_surface(part.data, geometry, part.matrix_world)
          for part, geometry in spoon_parts.items()]
tray_rims = [part for part in tray.children if part == tray_base or part.name.startswith('Raised tray rim')]
tray_shapes = {part: collider([f.vertices[:] for f in part.data.polygons]) for part in tray_rims}
for start, end in ((3159, 3239), (3565, 3645)):
    for frame in range(start, end + 1):
        scene.frame_set(frame)
        for part, geometry in tray_shapes.items():
            metal = transformed_surface(part.data, geometry, part.matrix_world)
            assert all(not metal.overlap(wood) for wood in parked), (frame, 'tray hits resting spoon')

# Measure the neck against the skinned chest in its own coordinate system so
# walking, leaning and looking at the bowl don't register as a separation.
scene.frame_set(0)
neck = objects['Head • rotate here']


def neck_in_chest_space():
    return chest_world_transform().inverted() @ neck.matrix_world.translation


collar = neck_in_chest_space()
pelvis = rig.pose.bones['Pelvis']
hem = objects['Apron • hem stitch']


def hem_in_pelvis_space():
    seam = hem.evaluated_get(depsgraph)
    return (rig.matrix_world @ pelvis.matrix).inverted() @ (seam.matrix_world @ seam.data.vertices[0].co)


rest_hem = hem_in_pelvis_space()

for frame in (*range(cup['pickup_frame'], cup['setdown_frame'] + 1),
              *range(objects['Whole egg 1']['pickup_frame'], objects['Whole egg 2']['return_frame'] + 1),
              *range(pickups[0], returns[0] + 1)):
    scene.frame_set(frame)
    assert (neck_in_chest_space() - collar).length < .003, (frame, 'head lifts off chest')
    assert (hem_in_pelvis_space() - rest_hem).length < .03, (frame, 'lean lifts the apron hem')

scene.frame_set(read + 78)
assert book.matrix_world.translation.z > book_home.z + .3
scene.frame_set(pickups[0])
assert (book.matrix_world.translation - book_home).length < .01

# The ceramic floor is exposed through the opening and book reading. Then each
# addition raises the surface during the actual fall, with a steady level between additions.
dough = objects['Bowl | cookie dough']
batter = objects['Bowl | mixed batter']
sole = objects['Scale | nonslip sole']
platform = objects['Scale | weighing platform']
sole_bottom = min((sole.matrix_world @ v.co).z for v in sole.data.vertices)
platform_top = max((platform.matrix_world @ v.co).z for v in platform.data.vertices)
bowl_bottom = min((bowl.matrix_world @ v.co).z for v in bowl.data.vertices)
assert abs(sole_bottom - worktop_z) < .001 and abs(bowl_bottom - platform_top) < .001
assert (platform.matrix_world.translation.xy - bowl.matrix_world.translation.xy).length < .001
scene.frame_set(wet_stir)
display = objects['Scale | recessed display']
toward_cook = (hips.matrix_world.translation - bowl.matrix_world.translation).xy.normalized()
face_normal = display.matrix_world.to_quaternion() @ Vector((0, 0, 1))
assert face_normal.xy.normalized().dot(toward_cook) > .7
assert (display.matrix_world.translation - bowl.matrix_world.translation).xy.dot(toward_cook) > 0
assert all(o.animation_data is None for o in bpy.data.collections['Kitchen scale'].objects)
for frame in (0, read, butter_pour):
    scene.frame_set(frame)
    assert not any(map(visible, (dough, batter, deposited_butter)))
    for dx, dy in ((.12, .04), (-.18, .03), (.04, -.19)):
        point = bowl.matrix_world.translation + Vector((dx, dy, .65))
        hit, _, _, _, first, _ = scene.ray_cast(depsgraph, point, Vector((0, 0, -1)))
        assert hit and first == bowl, (frame, 'bowl should be empty')

for name, surface, delay, duration in (
    ('Pour melted butter into the bowl', deposited_butter, 6, 21),
    ('Pour brown sugar', deposited_butter, 6, 30),
    ('Release egg 1', deposited_butter, 4, 14),
    ('Release egg 2', deposited_butter, 6, 13),
    ('Pour vanilla', deposited_butter, 7, 20),
    ('Shake baking soda', deposited_butter, 8, 27),
    ('Shake espresso powder', deposited_butter, 6, 24),
    ('Pour flour', batter, 5, 23),
    ('Pour chocolate chips', batter, 6, 28),
):
    start = scene.timeline_markers[name].frame + delay
    levels = []
    for frame in range(start, start + duration + 1):
        scene.frame_set(frame)
        levels.append(surface.matrix_world.translation.z)
    assert all(a < b for a, b in zip(levels, levels[1:])), (name, 'fill should rise with the pour')
    scene.frame_set(start + duration + 6)
    assert visible(surface) and abs(surface.matrix_world.translation.z - levels[-1]) < .0001

scene.frame_set(wet_return)
assert visible(batter) and not visible(dough)
scene.frame_set(returns[-1])
assert visible(dough) and not visible(batter)
amount = dough.matrix_world.to_scale().z
for index in range(1, 4):
    scene.frame_set(scene.timeline_markers[f'Roll and place dough ball {index}'].frame + 45)
    remaining = dough.matrix_world.to_scale().z
    assert remaining < amount, ('dough portion', index)
    amount = remaining
assert not visible(dough)

for count, seconds in enumerate((93.15, 97.57, 101.94), start=1):
    at(seconds)
    assert sum(map(visible, cookies)) == count
    assert all(cookie.parent == tray for cookie in cookies)
at(101.94)
raw_heights = [cookie.matrix_world.to_scale().z for cookie in cookies]
slots = [tray.matrix_world.inverted() @ c.matrix_world.translation for c in cookies]
assert min((slots[i] - slots[j]).length for i in range(3) for j in range(i)) > .44

# The camera-facing cookie sits halfway between the rear pair. Leave enough
# parchment around it for the dough to spread in the oven.
left, right, front = slots
assert min(left.y, right.y) - front.y > .42
assert abs(front.x - (left.x + right.x) / 2) < .05
# The rig labels left and right as seen from the front. Rig R is his left flipper.
for frame in range(3020, 3046):
    scene.frame_set(frame)
    position = cookies[2].matrix_world.translation
    hand = rig.matrix_world @ rig.pose.bones['Foreflipper.R'].tail
    free_hand = rig.matrix_world @ rig.pose.bones['Foreflipper.L'].tail
    assert (hand - position - Vector((.10, .035, -.045))).length < .04, ('left placement', frame)
    assert (free_hand - position).length > .28, ('right hand released', frame)
at(115.72)
paper = objects['Parchment paper on tray']
to_paper = paper.matrix_world.inverted()
third = objects['Cookie 3 | golden dough']
points = [to_paper @ third.matrix_world @ v.co for v in third.data.vertices]
paper_front = min(v.co.y for v in paper.data.vertices)
assert min(v.y for v in points) >= paper_front
at(109.21)
assert (tray.matrix_world.translation - Vector((1.93, 2.30, .94))).length < .02
assert abs(objects['Oven | hinged door'].rotation_euler.x) < .01
for cookie, slot in zip(cookies, slots):
    assert (tray.matrix_world.inverted() @ cookie.matrix_world.translation - slot).length < .02
at(115.72)
for number, cookie in enumerate(cookies, start=1):
    edge = objects[f'Cookie {number} | toasted lower edge']
    assert visible(edge)
    assert (edge.matrix_world.translation - cookie.matrix_world.translation).length < .05
at(118.12)
assert (mitt.matrix_world.translation - mitt_home).length > .35
at(126.07)
assert (tray.matrix_world.translation - tray_home).length < .01
assert (mitt.matrix_world.translation - mitt_home).length < .01
left_mitt_home = mitts['L'].matrix_world.translation.copy()
at(0)
assert (mitts['L'].matrix_world.translation - left_mitt_home).length < .01
at(126.07)
assert all(cookie.matrix_world.to_scale().z < height * .65 for cookie, height in zip(cookies, raw_heights))
# Each bite removes a distinct piece at the bill. The remaining pieces move
# away with the flipper before the next bite.
samples = [scene.timeline_markers[f'Cookie bite {i}'].frame for i in range(1, 6)]
pieces = [objects[f'Cookie 1 | bite {i}'] for i in range(2, 6)]
whole = objects['Cookie 1 | golden dough']
bill = objects['Beak • upper bill']
for i, frame in enumerate(samples):
    scene.frame_set(frame - 1)
    mouthful = whole if i == 0 else pieces[i - 1]
    assert visible(mouthful)
    lip = bill.matrix_world @ Vector((.015, -.13, -.052))
    close = (cookies[0].matrix_world.translation - lip).length
    hit, point, _, _ = mouthful.closest_point_on_mesh(mouthful.matrix_world.inverted() @ lip)
    assert hit and (mouthful.matrix_world @ point - lip).length < .06, (frame, 'bite at bill')
    scene.frame_set(frame + 1)
    assert not visible(whole)
    assert list(map(visible, pieces)) == [j > i for j in range(1, 5)]
    if i < 4:
        scene.frame_set(frame + 13)
        lip = bill.matrix_world @ Vector((.015, -.13, -.052))
        assert (cookies[0].matrix_world.translation - lip).length > close + .08

celebration = scene.timeline_markers['Happy dance after the last bite'].frame
assert celebration > samples[-1]
hips, palms = [], []
for frame in range(celebration, scene.frame_end + 1, 4):
    scene.frame_set(frame)
    assert not visible(cookies[0]) and all(visible(c) for c in cookies[1:])
    hips.append(objects['Penguini | kitchen placement'].matrix_world.translation.x)
    palms.append((rig.matrix_world @ rig.pose.bones['Foreflipper.R'].tail).z)
assert max(hips) - min(hips) > .23 and max(palms) - min(palms) > .40
scene.frame_set(scene.frame_end)
assert list(map(visible, cookies)) == [False, True, True]
print('PASS: planted steps, torso clearance, prop contacts and cookie recipe continuity')
