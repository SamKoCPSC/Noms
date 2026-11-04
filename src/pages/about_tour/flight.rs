use super::math::*;
pub const COUNT: usize = super::super::about::IDS.len();
pub const FOV: f32 = 0.88;
// Keep in sync with the mobile card height in about-tour.css.
pub const MOBILE_PANEL_TOP: f32 = 0.48;
const EYES: [[f32; 3]; COUNT] = [
    [0., 3.1, 12.],
    [7., 3.9, -10.],
    [-4., 3.2, -32.],
    [6., 4.4, -54.],
    [-3., 3.7, -76.],
];
const YAWS: [f32; COUNT] = [0., 0.32, -0.28, 0.28, -0.22];
pub struct Camera {
    pub eye: [f32; 3],
    pub vp: Mat4,
    pub progress: f32,
}
pub fn station(i: usize) -> Mat4 {
    mul(translate(EYES[i][0], EYES[i][1], EYES[i][2]), ry(YAWS[i]))
}
pub fn point(m: Mat4, p: [f32; 3]) -> [f32; 3] {
    [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
    ]
}
fn catmull(a: f32, b: f32, c: f32, d: f32, t: f32) -> f32 {
    0.5 * ((2. * b)
        + (-a + c) * t
        + (2. * a - 5. * b + 4. * c - d) * t * t
        + (-a + 3. * b - 3. * c + d) * t * t * t)
}
pub fn camera(p: f32, aspect: f32, pointer: [f32; 2]) -> Camera {
    let p = p.clamp(0., (COUNT - 1) as f32);
    let i = (p.floor() as usize).min(COUNT - 2);
    let t = smooth(0.06, 0.94, p - i as f32);
    let mut eye = [0.; 3];
    for (k, v) in eye.iter_mut().enumerate() {
        *v = catmull(
            EYES[i.saturating_sub(1)][k],
            EYES[i][k],
            EYES[i + 1][k],
            EYES[(i + 2).min(COUNT - 1)][k],
            t,
        )
    }
    let yaw = YAWS[i] + (YAWS[i + 1] - YAWS[i]) * t;
    eye[0] += pointer[0] * 0.055;
    eye[1] -= pointer[1] * 0.035;
    let target = [eye[0] - yaw.sin() * 10., eye[1], eye[2] - yaw.cos() * 10.];
    Camera {
        eye,
        vp: mul(perspective(FOV, aspect), look_at(eye, target)),
        progress: i as f32 + t,
    }
}
/// Perspective framing needs the silhouette in X/Z and Y/Z, not the empty
/// corners of a bounding box. Cache only those convex outlines, not the mesh.
#[derive(Default)]
pub struct RoomFrame {
    center: [f32; 3],
    outlines: [Vec<[f32; 2]>; 2],
}
impl RoomFrame {
    pub fn new(points: Vec<[f32; 3]>) -> Self {
        let mut bounds = [[f32::INFINITY; 3], [f32::NEG_INFINITY; 3]];
        for point in &points {
            for k in 0..3 {
                bounds[0][k] = bounds[0][k].min(point[k]);
                bounds[1][k] = bounds[1][k].max(point[k]);
            }
        }
        let center = std::array::from_fn(|k| (bounds[0][k] + bounds[1][k]) * 0.5);
        let outlines = [0, 1].map(|axis| {
            let mut plane: Vec<_> = points
                .iter()
                .map(|p| [p[axis] - center[axis], p[2] - center[2]])
                .collect();
            plane.sort_unstable_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
            plane.dedup();
            let mut hull: Vec<[f32; 2]> = Vec::new();
            for reverse in [false, true] {
                let start = hull.len();
                for i in 0..plane.len() {
                    let p = plane[if reverse { plane.len() - 1 - i } else { i }];
                    while hull.len() >= start + 2 {
                        let a = hull[hull.len() - 2];
                        let b = hull[hull.len() - 1];
                        if (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) > 0. {
                            break;
                        }
                        hull.pop();
                    }
                    hull.push(p);
                }
                hull.pop();
            }
            hull
        });
        Self { center, outlines }
    }
    pub fn model(&self, w: f32, h: f32, rotation: Mat4) -> Mat4 {
        let (left, top, right, bottom) = if w <= 700. {
            (8., 6., w - 8., h * MOBILE_PANEL_TOP - 10.)
        } else {
            (
                w * 0.035 + (w * 0.4).clamp(320., 520.) + 16.,
                76.,
                w - 20.,
                h - 24.,
            )
        };
        let focal = h * 0.5 / (FOV * 0.5).tan();
        let offset = [(left + right - w) * 0.5, (h - top - bottom) * 0.5];
        let margin = [(right - left).max(1.) * 0.48, (bottom - top).max(1.) * 0.48];
        let distance = 11.7;
        let mut size: f32 = 10.;
        for (k, outline) in self.outlines.iter().enumerate() {
            for &[p, z] in outline {
                let denominator = (focal * p + offset[k] * z).abs() + margin[k] * z;
                if denominator > 0. {
                    size = size.min(margin[k] * distance / denominator);
                }
            }
        }
        // Width-limited rooms otherwise leave a large empty gap below them.
        // Align the projected silhouette with the card, retaining the fit above.
        let y = if w <= 700. {
            self.outlines[1]
                .iter()
                .map(|&[p, z]| (h * 0.5 - bottom) * (distance - size * z) / focal - size * p)
                .fold(f32::NEG_INFINITY, f32::max)
        } else {
            offset[1] * distance / focal
        };
        mul(
            translate(offset[0] * distance / focal, y, -distance),
            mul(
                scale(size, size, size),
                mul(
                    translate(-self.center[0], -self.center[1], -self.center[2]),
                    rotation,
                ),
            ),
        )
    }
}

pub fn panel_model(i: usize, w: f32, h: f32, pw: f32, ph: f32) -> Mat4 {
    let focal = h * 0.5 / (FOV * 0.5).tan();
    let unit = 10. / focal;
    let mobile = w <= 700.;
    let x = if mobile {
        0.
    } else {
        (w * 0.035 + pw * 0.5 - w * 0.5) * unit
    };
    let center = if mobile {
        h * MOBILE_PANEL_TOP + ph * 0.5
    } else {
        (h * 0.52).min(h - 24. - ph * 0.5)
    };
    let y = (h * 0.5 - center) * unit;
    mul(
        station(i),
        mul(
            translate(x, y, -10.),
            mul(
                scale(unit, -unit, unit),
                translate(-pw * 0.5, -ph * 0.5, 0.),
            ),
        ),
    )
}
pub fn css_projection(vp: Mat4, model: Mat4, w: f32, h: f32) -> (Mat4, f32) {
    let m = mul(vp, model);
    let mut out = [0.; 16];
    for c in 0..4 {
        out[c * 4] = (m[c * 4] + m[c * 4 + 3]) * w * 0.5;
        out[c * 4 + 1] = (-m[c * 4 + 1] + m[c * 4 + 3]) * h * 0.5;
        out[c * 4 + 2] = m[c * 4 + 2] * 0.001;
        out[c * 4 + 3] = m[c * 4 + 3];
    }
    (out, m[15])
}

/// Document-space reading and travel distances for a mobile chapter.
pub struct MobileSection {
    pub read: f32,
    pub hold: f32,
    pub travel: f32,
}
impl MobileSection {
    pub fn new(viewport: f32, text: f32, visible_text: f32) -> Self {
        let read = (text - visible_text).max(0.);
        Self {
            read,
            hold: read + (viewport * 0.16).max(80.),
            travel: (viewport * 1.5).max(480.),
        }
    }
    pub fn progress(&self, distance: f32) -> f32 {
        ((distance - self.hold) / self.travel).clamp(0., 1.)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_rooms_fill_their_space_without_overlapping_text_or_controls() {
        for (i, bytes) in [
            include_bytes!("../../../assets/about/sams-kitchen-detailed.glb").as_slice(),
            include_bytes!("../../../assets/about/sams-office.glb").as_slice(),
            include_bytes!("../../../assets/about/sams-bedroom.glb").as_slice(),
            include_bytes!("../../../assets/about/sams-potluck.glb").as_slice(),
            include_bytes!("../../../assets/about/sams-garden.glb").as_slice(),
        ]
        .into_iter()
        .enumerate()
        {
            let mesh = super::super::gltf::decode(bytes).unwrap();
            for (w, h) in [
                (360., 558.),
                (393., 770.),
                (768., 668.),
                (1280., 796.),
                (1920., 976.),
            ] {
                let rotation = if i == 0 {
                    mul(rx(0.522718), ry(-0.655696))
                } else if i == 1 {
                    mul(rx(0.58), ry(0.62))
                } else if i == 4 && w <= 700. {
                    transform([0.; 3], [0.9, -0.4, 0.], [1.; 3])
                } else {
                    transform([0.; 3], [0.58, -0.62, 0.], [1.; 3])
                };
                let points: Vec<_> = mesh
                    .vertices
                    .as_chunks::<12>()
                    .0
                    .iter()
                    .map(|v| [v[0], v[1], v[2]])
                    .collect();
                let frame = RoomFrame::new(points.iter().map(|p| point(rotation, *p)).collect());
                let camera = camera(i as f32, w / h, [0.; 2]);
                assert!(camera
                    .eye
                    .iter()
                    .zip(EYES[i])
                    .all(|(a, b)| (a - b).abs() < 0.0001));
                assert_eq!(camera.progress, i as f32);
                let model = mul(station(i), frame.model(w, h, rotation));
                let (projection, _) = css_projection(camera.vp, model, w, h);
                let mut extent = [[f32::INFINITY; 2], [f32::NEG_INFINITY; 2]];
                for p in points {
                    let q = point(projection, p);
                    let divisor = projection[3] * p[0]
                        + projection[7] * p[1]
                        + projection[11] * p[2]
                        + projection[15];
                    assert!(divisor > 0.);
                    for k in 0..2 {
                        extent[0][k] = extent[0][k].min(q[k] / divisor);
                        extent[1][k] = extent[1][k].max(q[k] / divisor);
                    }
                }
                let (left, top, right, bottom) = if w <= 700. {
                    (8., 6., w - 8., h * MOBILE_PANEL_TOP - 10.)
                } else {
                    (
                        w * 0.035 + (w * 0.4).clamp(320., 520.) + 16.,
                        76.,
                        w - 20.,
                        h - 24.,
                    )
                };
                assert!(
                    extent[0][0] >= left - 0.1 && extent[1][0] <= right + 0.1,
                    "room {i} horizontal fit at {w}x{h}: {extent:?}"
                );
                assert!(
                    extent[0][1] >= top - 0.1 && extent[1][1] <= bottom + 0.1,
                    "room {i} vertical fit at {w}x{h}: {extent:?}"
                );
                assert!(
                    (extent[1][0] - extent[0][0]) > (right - left) * 0.65
                        || (extent[1][1] - extent[0][1]) > (bottom - top) * 0.65
                );
                let pw = if w <= 700. {
                    w - 24.
                } else {
                    (w * 0.4).clamp(320., 520.)
                };
                let ph = if w <= 700. {
                    h * (1. - MOBILE_PANEL_TOP) - 18.
                } else {
                    h - 104.
                };
                let (panel, _) = css_projection(camera.vp, panel_model(i, w, h, pw, ph), w, h);
                let panel_top = panel[13] / panel[15];
                assert!(panel_top >= 0. && panel_top + ph <= h - 17.);
                if w <= 700. {
                    assert!(
                        (panel_top - extent[1][1] - 10.).abs() < 0.1,
                        "room {i} gap: {extent:?}"
                    );
                } else {
                    assert!(panel[12] / panel[15] + pw < extent[0][0]);
                }
            }
        }
    }
}

#[cfg(test)]
mod mobile_reading_tests {
    use super::*;
    #[test]
    fn camera_waits_for_all_text_even_on_short_screens() {
        for viewport in [308., 558., 770.] {
            for text in [180., 600., 1600.] {
                let visible = viewport * (1. - MOBILE_PANEL_TOP) - 18. - 50.;
                let section = MobileSection::new(viewport, text, visible);
                assert_eq!(section.progress(0.), 0.);
                assert_eq!(section.progress(section.read), 0.);
                assert_eq!(section.progress(section.hold), 0.);
                assert!(
                    (section.progress(section.hold + section.travel * 0.5) - 0.5).abs() < 0.0001
                );
                assert!((section.progress(section.hold + section.travel) - 1.).abs() < 0.0001);
                // The end of the text is inside the visible reading area.
                assert!(text - section.read <= visible + 0.001);
            }
        }
    }
    #[test]
    fn longer_copy_adds_reading_distance_without_speeding_up_travel() {
        let short = MobileSection::new(770., 600., 294.);
        let long = MobileSection::new(770., 1600., 294.);
        assert!((long.hold - short.hold - 1000.).abs() < 0.001);
        assert_eq!(short.travel, long.travel);
    }
}
