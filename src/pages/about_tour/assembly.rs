//! Staged, once-only assembly with bounded curved paths and gradual surface reveals.
use super::gltf::{DioramaMesh, DioramaPart};
use super::math::{identity, mul, transform, translate, Mat4};
use std::f32::consts::PI;
const DURATION: f32 = 3.0;
#[derive(Default)]
pub struct Playback {
    elapsed: f32,
}
impl Playback {
    pub fn advance(&mut self, seconds: f32) {
        self.elapsed = (self.elapsed + seconds.max(0.)).min(DURATION);
    }
    #[cfg(test)]
    pub fn finish(&mut self) {
        self.elapsed = DURATION;
    }
    pub fn finished(&self) -> bool {
        self.elapsed >= DURATION
    }
    pub fn frame(&self) -> f32 {
        1. + 159. * self.elapsed / DURATION
    }
}
pub struct ChapterArrival {
    playback: Playback,
    started: bool,
    chapter: f32,
}
impl ChapterArrival {
    pub fn new(chapter: usize) -> Self {
        Self {
            playback: Playback::default(),
            started: false,
            chapter: chapter as f32,
        }
    }
    pub fn advance(&mut self, progress: f32, seconds: f32) {
        self.started |= (self.chapter - 0.28..=self.chapter + 0.28).contains(&progress);
        if self.started {
            self.playback.advance(seconds);
        }
    }
    pub fn animating(&self) -> bool {
        self.started && !self.playback.finished()
    }
    pub fn frame(&self) -> f32 {
        self.playback.frame()
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Room {
    Kitchen,
    Office,
    Potluck,
    Bedroom,
    Garden,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Floor,
    BackWall,
    SideWall,
    RightFence,
    Furniture,
    Detail,
}
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}
impl Bounds {
    fn empty() -> Self {
        Self {
            min: [f32::INFINITY; 3],
            max: [f32::NEG_INFINITY; 3],
        }
    }
    fn add(&mut self, v: &[f32]) {
        for (i, &x) in v.iter().take(3).enumerate() {
            self.min[i] = self.min[i].min(x);
            self.max[i] = self.max[i].max(x);
        }
    }
    fn center(self) -> [f32; 3] {
        std::array::from_fn(|i| (self.min[i] + self.max[i]) * 0.5)
    }
    fn size(self) -> [f32; 3] {
        std::array::from_fn(|i| self.max[i] - self.min[i])
    }
    pub fn transformed(self, m: Mat4) -> Self {
        let mut b = Self::empty();
        for bits in 0..8 {
            let v: [f32; 3] = std::array::from_fn(|i| {
                if bits & (1 << i) == 0 {
                    self.min[i]
                } else {
                    self.max[i]
                }
            });
            let w: [f32; 3] = std::array::from_fn(|i| {
                m[i] * v[0] + m[4 + i] * v[1] + m[8 + i] * v[2] + m[12 + i]
            });
            b.add(&w);
        }
        b
    }
}
fn bounds(data: &DioramaMesh, part: &DioramaPart) -> Bounds {
    let mut b = Bounds::empty();
    for v in data.vertices[part.first * 12..(part.first + part.count) * 12]
        .as_chunks::<12>()
        .0
        .iter()
    {
        b.add(v);
    }
    b
}
/// Split exported combined architecture into independently animated panels.
/// Every original triangle and vertex attribute is retained verbatim.
pub fn prepare(data: &DioramaMesh, room: Room) -> DioramaMesh {
    let mut out = DioramaMesh {
        vertices: Vec::with_capacity(data.vertices.len()),
        parts: Vec::new(),
    };
    for p in &data.parts {
        let name = p.name.to_lowercase();
        let shell = name.contains("room shell")
            || name.contains("continuous shell")
            || name.contains("continuous room shell")
            || name == "asm_02_room";
        let fence = room == Room::Garden && name.contains("timber garden fence");
        if !shell && !fence {
            let mut part = p.clone();
            part.first = out.vertices.len() / 12;
            out.vertices
                .extend_from_slice(&data.vertices[p.first * 12..(p.first + p.count) * 12]);
            out.parts.push(part);
            continue;
        }
        let b = bounds(data, p);
        let mut buckets: [Vec<f32>; 4] = std::array::from_fn(|_| Vec::new());
        for tri in data.vertices[p.first * 12..(p.first + p.count) * 12]
            .as_chunks::<36>()
            .0
            .iter()
        {
            let cx = (tri[0] + tri[12] + tri[24]) / 3.;
            let cz = (tri[2] + tri[14] + tri[26]) / 3.;
            let side_x = if room == Room::Office {
                b.max[0]
            } else {
                b.min[0]
            };
            let mut index = if (cx - side_x).abs() < (cz - b.min[2]).abs() {
                2
            } else {
                1
            };
            if fence && (cx - b.max[0]).abs() < (cx - b.min[0]).abs().min((cz - b.min[2]).abs()) {
                index = 3;
            }
            if shell && name != "asm_02_room" && tri[1].max(tri[13]).max(tri[25]) <= 0.08 {
                index = 0;
            }
            buckets[index].extend_from_slice(tri);
        }
        for (i, verts) in buckets.into_iter().enumerate() {
            if verts.is_empty() {
                continue;
            }
            let mut part = p.clone();
            part.first = out.vertices.len() / 12;
            part.count = verts.len() / 12;
            part.name = format!(
                "{} / {}",
                p.name,
                ["floor", "back wall", "side wall", "right fence"][i]
            );
            out.vertices.extend(verts);
            out.parts.push(part);
        }
    }
    out
}
#[derive(Clone, Debug)]
pub struct Motion {
    pub kind: Kind,
    pub bounds: Bounds,
    pub start: f32,
    pub end: f32,
    pivot: [f32; 3],
    offset: [f32; 3],
    variant: f32,
}
pub struct ArrivalPose {
    pub matrix: Mat4,
    pub reveal: f32,
}
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0., 1.) as f64;
    (t * t * t * (t * (t * 6. - 15.) + 10.)).clamp(0., 1.) as f32
}
impl Motion {
    pub fn new(data: &DioramaMesh, p: &DioramaPart, room: Room) -> Self {
        // Prints and transparent pieces use exactly the same bounds, timing and pose.
        let mut b = Bounds::empty();
        for other in data.parts.iter().filter(|o| o.name == p.name) {
            let ob = bounds(data, other);
            b.add(&ob.min);
            b.add(&ob.max);
        }
        let name = p.name.to_lowercase();
        let kind =
            if name.ends_with("/ floor") || name == "asm_01_floor" || name.contains("rounded lawn")
            {
                Kind::Floor
            } else if name.ends_with("/ back wall")
                || name.contains("house facade")
                || name.contains("bed alcove root")
            {
                Kind::BackWall
            } else if name.ends_with("/ side wall") {
                Kind::SideWall
            } else if name.ends_with("/ right fence") {
                Kind::RightFence
            } else if p.start >= if room == Room::Kitchen { 92. } else { 75. } {
                Kind::Detail
            } else {
                Kind::Furniture
            };
        let seed = p
            .name
            .bytes()
            .fold(0u32, |h, c| h.wrapping_mul(31).wrapping_add(c as u32));
        let variant = (seed % 101) as f32 / 50. - 1.;
        let (start, end) = match kind {
            Kind::Floor => (1., 24.),
            Kind::BackWall => (9., 45.),
            Kind::SideWall => (17., 53.),
            Kind::RightFence => (23., 59.),
            Kind::Furniture => {
                let s = 43. + (p.start.clamp(18., 88.) - 18.) * 0.65;
                (s, s + 39.)
            }
            Kind::Detail => {
                let s = 92. + (p.start.clamp(75., 120.) - 75.) * 0.62;
                (s, s + 33.)
            }
        };
        let mut pivot = b.center();
        if matches!(kind, Kind::BackWall | Kind::SideWall | Kind::RightFence) {
            pivot[1] = b.min[1];
        }
        Self {
            kind,
            bounds: b,
            start,
            end,
            pivot,
            offset: p.offset,
            variant,
        }
    }
    pub fn amount(&self, frame: f32) -> f32 {
        ((frame - self.start) / (self.end - self.start)).clamp(0., 1.)
    }
    pub fn pose(&self, frame: f32, room: Room) -> ArrivalPose {
        let u = self.amount(frame);
        // Exact identity at completion prevents settling jitter and preserves the master.
        if u >= 1. {
            return ArrivalPose {
                matrix: identity(),
                reveal: 1.,
            };
        }
        let q = smooth(u);
        let rest = 1. - q;
        let sign = if room == Room::Office { -1. } else { 1. };
        let mut size = [1.; 3];
        let mut rot = [0.; 3];
        let mut delta = [0.; 3];
        match self.kind {
            Kind::Floor => {
                size = [0.94 + 0.06 * q; 3];
                delta[1] = -0.20 * rest;
            }
            Kind::BackWall | Kind::SideWall | Kind::RightFence => {
                // Grow from the lower edge and unfold gently toward the room interior.
                size[1] = 0.16 + 0.84 * q;
                if self.kind == Kind::BackWall {
                    rot[0] = 0.32 * rest;
                } else {
                    rot[2] = if self.kind == Kind::RightFence {
                        0.30 * rest
                    } else {
                        -sign * 0.30 * rest
                    };
                }
                delta[1] = 0.16 * rest;
            }
            Kind::Furniture | Kind::Detail => {
                let detail = self.kind == Kind::Detail;
                let arc = 16. * u * u * (1. - u) * (1. - u);
                let footprint = self.bounds.size();
                // Long assemblies get a smaller yaw, keeping their sweep controlled.
                let turn = (1.6 / footprint[0].max(footprint[2]).max(1.6)).min(1.);
                size = [if detail {
                    0.68 + 0.32 * q
                } else {
                    0.86 + 0.14 * q
                }; 3];
                rot = [
                    0.09 * rest * turn,
                    self.variant * 0.30 * rest * turn,
                    self.variant * 0.045 * arc * turn,
                ];
                delta[0] = if room == Room::Garden {
                    self.variant * 0.12 * arc
                } else {
                    sign * (self.offset[0].abs().max(0.85) * rest + 0.35 * arc)
                };
                delta[2] = self.offset[2]
                    .abs()
                    .max(if room == Room::Garden { 2.2 } else { 1.4 })
                    * rest
                    + 0.22 * arc;
                let settle = if u > 0.78 {
                    ((u - 0.78) / 0.22 * PI).sin().powi(2) * 0.035
                } else {
                    0.
                };
                delta[1] = 0.20 * rest + if detail { 0.62 * arc } else { 0.28 * arc } + settle;
            }
        }
        let center = std::array::from_fn(|i| self.pivot[i] + delta[i]);
        let mut matrix = mul(
            transform(center, rot, size),
            translate(-self.pivot[0], -self.pivot[1], -self.pivot[2]),
        );
        if self.kind != Kind::Floor {
            let mut moved = self.bounds.transformed(matrix);
            // Keep the whole rotated AABB on the open side of the original wall planes.
            matrix[14] += (self.bounds.min[2] - moved.min[2]).max(0.);
            matrix[13] += (self.bounds.min[1] - moved.min[1]).max(0.);
            if room == Room::Garden {
                let lo = (-7.6_f32).min(self.bounds.min[0]);
                let hi = 7.5_f32.max(self.bounds.max[0]);
                if moved.max[0] - moved.min[0] > hi - lo {
                    // Fence panels may span the whole plot; shrink the horizontal sweep.
                    let factor = (hi - lo) / (moved.max[0] - moved.min[0]);
                    for c in 0..3 {
                        matrix[c * 4] *= factor;
                    }
                    matrix[12] *= factor;
                    moved = self.bounds.transformed(matrix);
                }
                matrix[12] += (lo - moved.min[0]).max(0.) - (moved.max[0] - hi).max(0.);
            } else if room == Room::Office {
                matrix[12] -= (moved.max[0] - self.bounds.max[0]).max(0.);
            } else {
                matrix[12] += (self.bounds.min[0] - moved.min[0]).max(0.);
            }
        }
        ArrivalPose {
            matrix,
            reveal: smooth(u / 0.42),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn models() -> Vec<(Room, DioramaMesh)> {
        [
            (
                Room::Kitchen,
                include_bytes!("../../../assets/about/sams-kitchen-detailed.glb").as_slice(),
            ),
            (
                Room::Office,
                include_bytes!("../../../assets/about/sams-office.glb").as_slice(),
            ),
            (
                Room::Bedroom,
                include_bytes!("../../../assets/about/sams-bedroom.glb").as_slice(),
            ),
            (
                Room::Potluck,
                include_bytes!("../../../assets/about/sams-potluck.glb").as_slice(),
            ),
            (
                Room::Garden,
                include_bytes!("../../../assets/about/sams-garden.glb").as_slice(),
            ),
        ]
        .into_iter()
        .map(|(room, bytes)| (room, super::super::gltf::decode(bytes).unwrap()))
        .collect()
    }
    fn fingerprint(m: &DioramaMesh) -> Vec<u64> {
        let mut result: Vec<_> = m
            .vertices
            .as_chunks::<36>()
            .0
            .iter()
            .map(|t| {
                t.iter().fold(14695981039346656037u64, |h, v| {
                    (h ^ v.to_bits() as u64).wrapping_mul(1099511628211)
                })
            })
            .collect();
        result.sort_unstable();
        result
    }
    #[test]
    fn entrances_start_at_zero_and_never_replay() {
        let mut p = Playback::default();
        assert_eq!(p.frame(), 1.);
        p.advance(DURATION);
        assert_eq!(p.frame(), 160.);
        p.advance(-10.);
        assert!(p.finished());
        let mut skipped = Playback::default();
        skipped.finish();
        assert_eq!(skipped.frame(), 160.);
        for chapter in 1..5 {
            let mut a = ChapterArrival::new(chapter);
            a.advance(0., 20.);
            assert_eq!(a.frame(), 1.);
            a.advance(chapter as f32 - 0.2, 0.1);
            assert!(a.animating());
            a.advance(0., DURATION);
            for stop in [0., 4., chapter as f32] {
                a.advance(stop, 1.);
                assert_eq!(a.frame(), 160.);
                assert!(!a.animating());
            }
        }
    }
    #[test]
    fn splitting_architecture_preserves_every_triangle_and_final_placement() {
        for (room, original) in models() {
            let m = prepare(&original, room);
            assert_eq!(fingerprint(&original), fingerprint(&m));
            let mut walls = 0;
            let mut floors = 0;
            for p in &m.parts {
                let motion = Motion::new(&m, p, room);
                let rest = motion.pose(160., room);
                assert_eq!(rest.matrix, identity());
                assert_eq!(rest.reveal, 1.);
                if matches!(
                    motion.kind,
                    Kind::BackWall | Kind::SideWall | Kind::RightFence
                ) {
                    walls += 1;
                    assert_eq!(motion.pose(1., room).reveal, 0.);
                    assert_ne!(
                        motion.pose((motion.start + motion.end) * 0.5, room).matrix,
                        identity()
                    );
                }
                if motion.kind == Kind::Floor {
                    floors += 1;
                }
            }
            assert!(walls >= 2, "{room:?}");
            assert!(floors > 0);
        }
    }
    #[test]
    fn all_rotated_groups_stay_clear_of_wall_and_fence_planes() {
        for (room, original) in models() {
            let m = prepare(&original, room);
            for p in &m.parts {
                let motion = Motion::new(&m, p, room);
                if motion.kind == Kind::Floor {
                    continue;
                }
                for frame in 0..=320 {
                    let pose = motion.pose(frame as f32 / 2., room);
                    let b = motion.bounds.transformed(pose.matrix);
                    let rest = motion.bounds;
                    assert!(pose.matrix.iter().all(|v| v.is_finite()));
                    assert!(
                        b.min[1] >= rest.min[1] - 0.00002,
                        "{} floor {frame}",
                        p.name
                    );
                    assert!(
                        b.min[2] >= rest.min[2] - 0.00002,
                        "{} back wall {frame}",
                        p.name
                    );
                    if room == Room::Garden {
                        assert!(
                            b.min[0] >= (-7.6_f32).min(rest.min[0]) - 0.00002,
                            "{} left {frame}",
                            p.name
                        );
                        assert!(
                            b.max[0] <= 7.5_f32.max(rest.max[0]) + 0.00002,
                            "{} right {frame}",
                            p.name
                        );
                    } else if room == Room::Office {
                        assert!(b.max[0] <= rest.max[0] + 0.00002, "{} side {frame}", p.name);
                    } else {
                        assert!(b.min[0] >= rest.min[0] - 0.00002, "{} side {frame}", p.name);
                    }
                }
            }
        }
    }
    #[test]
    fn reveals_are_gradual_and_prints_stay_attached() {
        for (room, original) in models() {
            let m = prepare(&original, room);
            for p in &m.parts {
                let motion = Motion::new(&m, p, room);
                let first = motion.pose(motion.start, room);
                assert_eq!(first.reveal, 0.);
                let early = motion.pose(motion.start + (motion.end - motion.start) * 0.1, room);
                assert!(early.reveal > 0. && early.reveal < 0.5);
                let mut prev = 0.;
                for i in 0..101 {
                    let pose = motion.pose(
                        motion.start + (motion.end - motion.start) * i as f32 / 100.,
                        room,
                    );
                    assert!(pose.reveal >= prev);
                    prev = pose.reveal;
                }
                for same in m.parts.iter().filter(|other| other.name == p.name) {
                    let other = Motion::new(&m, same, room);
                    for frame in [25., 55., 90., 125., 160.] {
                        assert_eq!(
                            motion.pose(frame, room).matrix,
                            other.pose(frame, room).matrix
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn furniture_has_curved_travel_and_rotation() {
        for (room, original) in models() {
            let m = prepare(&original, room);
            let p = m
                .parts
                .iter()
                .find(|p| Motion::new(&m, p, room).kind == Kind::Furniture)
                .unwrap();
            let motion = Motion::new(&m, p, room);
            let start = motion.pose(motion.start, room).matrix;
            let mid = motion.pose((motion.start + motion.end) * 0.5, room).matrix;
            assert!(mid[1].abs() + mid[2].abs() + mid[4].abs() + mid[6].abs() > 0.001);
            assert!(
                mid[13] > (start[13] + motion.pose(motion.end, room).matrix[13]) * 0.5,
                "{room:?} vertical arc"
            );
        }
    }
}
