//! Room-local light framing and bounded shadow quality, independent of the camera.
use super::assembly::{Motion, Room};
use super::flight::point;
use super::math::{look_at, mul, norm, Mat4};

// Master key softbox, converted from Blender Z-up to glTF Y-up.
pub const LIGHT: [f32; 3] = [-3., 9., 4.];

pub struct LightFrame {
    pub matrix: Mat4,
    pub depth: f32,
    /// Light-plane axes weighted by width/height relative to the depth range.
    pub plane_axes: [[f32; 3]; 2],
}
impl LightFrame {
    pub fn new(motions: &[Motion], room: Room) -> Self {
        let dir = norm(LIGHT);
        let view = look_at(dir.map(|v| v * 30.), [0.; 3]);
        let mut low = [f32::INFINITY; 3];
        let mut high = [f32::NEG_INFINITY; 3];
        // Include the entire flight, not just the finished room, in a fixed frustum.
        // Stable texel placement also prevents shadows swimming as pieces settle.
        for motion in motions {
            for frame in 1..=160 {
                let b = motion
                    .bounds
                    .transformed(motion.pose(frame as f32, room).matrix);
                for bits in 0..8 {
                    let p = point(
                        view,
                        std::array::from_fn(|i| {
                            if bits & (1 << i) == 0 {
                                b.min[i]
                            } else {
                                b.max[i]
                            }
                        }),
                    );
                    for i in 0..3 {
                        low[i] = low[i].min(p[i] - 0.3);
                        high[i] = high[i].max(p[i] + 0.3);
                    }
                }
            }
        }
        let span: [f32; 3] = std::array::from_fn(|i| (high[i] - low[i]).max(0.01));
        let ortho = [
            2. / span[0],
            0.,
            0.,
            0.,
            0.,
            2. / span[1],
            0.,
            0.,
            0.,
            0.,
            -2. / span[2],
            0.,
            -(high[0] + low[0]) / span[0],
            -(high[1] + low[1]) / span[1],
            (high[2] + low[2]) / span[2],
            1.,
        ];
        Self {
            matrix: mul(ortho, view),
            depth: span[2],
            plane_axes: std::array::from_fn(|axis| {
                std::array::from_fn(|i| view[i * 4 + axis] * span[axis] / span[2])
            }),
        }
    }
}

pub struct Quality {
    ceiling: u32,
    slow: u8,
}
impl Default for Quality {
    fn default() -> Self {
        Self {
            ceiling: 2048,
            slow: 0,
        }
    }
}
impl Quality {
    pub fn resolution(&self, mobile: bool, hardware_max: u32) -> u32 {
        let limit = self
            .ceiling
            .min(if mobile { 1024 } else { 2048 })
            .min(hardware_max);
        if limit < 256 {
            return 0;
        }
        1 << (31 - limit.leading_zeros())
    }
    pub fn observe(&mut self, milliseconds: f64, current_size: u32) {
        // Ignore pauses/background tabs and isolated compilation/GC hitches.
        if !milliseconds.is_finite() || !(1. ..1000.).contains(&milliseconds) {
            return;
        }
        if milliseconds > 48. {
            self.slow += 1;
        } else {
            self.slow = self.slow.saturating_sub(1);
        }
        if self.slow >= 12 {
            self.ceiling = self.ceiling.min((current_size / 2).max(512));
            self.slow = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::about_tour::{assembly, gltf};
    #[test]
    fn every_room_fits_its_shadow_frustum_throughout_entry() {
        for (room, bytes) in [
            (
                Room::Kitchen,
                include_bytes!("../../../assets/about/sams-kitchen-detailed.glb").as_slice(),
            ),
            (
                Room::Office,
                include_bytes!("../../../assets/about/sams-office.glb").as_slice(),
            ),
            (
                Room::Potluck,
                include_bytes!("../../../assets/about/sams-potluck.glb").as_slice(),
            ),
            (
                Room::Bedroom,
                include_bytes!("../../../assets/about/sams-bedroom.glb").as_slice(),
            ),
            (
                Room::Library,
                include_bytes!("../../../assets/about/sams-library.glb").as_slice(),
            ),
            (
                Room::Supermarket,
                include_bytes!("../../../assets/about/sams-supermarket.glb").as_slice(),
            ),
            (
                Room::CorporateOffice,
                include_bytes!("../../../assets/about/sams-corporate-office.glb").as_slice(),
            ),
            (
                Room::Garden,
                include_bytes!("../../../assets/about/sams-garden.glb").as_slice(),
            ),
        ] {
            let original = gltf::decode(bytes).unwrap();
            let model = assembly::prepare(&original, room);
            let motions: Vec<_> = model
                .parts
                .iter()
                .map(|p| Motion::new(&model, p, room))
                .collect();
            let light = LightFrame::new(&motions, room);
            assert!(light.depth.is_finite() && light.depth > 1. && light.depth < 80.);
            // PCF taps must compare against the receiver's plane, including broad lawns.
            // Check horizontal and both vertical planes independently of room dimensions.
            for (normal, offset) in [
                ([0., 1., 0.], [0.23, 0., -0.17]),
                ([-1., 0., 0.], [0., 0.19, 0.11]),
                ([0., 0., 1.], [0.21, -0.13, 0.]),
            ] {
                let a = point(light.matrix, [0.; 3]);
                let b = point(light.matrix, offset);
                let gradient = light.plane_axes.map(|axis| {
                    super::super::math::dot(normal, axis)
                        / super::super::math::dot(normal, norm(LIGHT))
                });
                let predicted = gradient[0] * (b[0] - a[0]) + gradient[1] * (b[1] - a[1]);
                assert!((predicted - (b[2] - a[2])).abs() < 0.00001);
            }
            for motion in &motions {
                for step in 0..=636 {
                    let b = motion
                        .bounds
                        .transformed(motion.pose(1. + step as f32 * 0.25, room).matrix);
                    for bits in 0..8 {
                        let p = point(
                            light.matrix,
                            std::array::from_fn(|i| {
                                if bits & (1 << i) == 0 {
                                    b.min[i]
                                } else {
                                    b.max[i]
                                }
                            }),
                        );
                        assert!(
                            p.iter().all(|v| (-1. ..=1.).contains(v)),
                            "clipped {room:?} shadow at {step}: {p:?}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn quality_obeys_hardware_caps_and_degrades_only_after_sustained_slow_frames() {
        let mut q = Quality::default();
        assert_eq!(q.resolution(false, 4096), 2048);
        assert_eq!(q.resolution(true, 4096), 1024);
        assert_eq!(q.resolution(false, 768), 512);
        assert_eq!(q.resolution(false, 128), 0);
        for ms in [90., 33., f64::NAN, 2000.] {
            q.observe(ms, q.resolution(false, 4096));
        }
        assert_eq!(q.resolution(false, 4096), 2048);
        for _ in 0..12 {
            q.observe(60., q.resolution(false, 4096));
        }
        assert_eq!(q.resolution(false, 4096), 1024);
        for _ in 0..120 {
            q.observe(60., q.resolution(false, 4096));
        }
        assert_eq!(q.resolution(true, 4096), 512);
        let mut phone = Quality::default();
        for _ in 0..12 {
            phone.observe(60., phone.resolution(true, 4096));
        }
        assert_eq!(phone.resolution(true, 4096), 512);
    }
}
