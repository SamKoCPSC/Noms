use super::math::*;
pub const COUNT: usize = 4;
pub const FOV: f32 = 0.88;
const EYES: [[f32; 3]; COUNT] = [
    [0., 3.1, 12.],
    [7., 3.9, -10.],
    [-4., 3.2, -32.],
    [6., 4.4, -54.],
];
const YAWS: [f32; COUNT] = [0., 0.32, -0.28, 0.28];
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
    let p = p.clamp(0., 3.);
    let i = (p.floor() as usize).min(2);
    let t = smooth(0.06, 0.94, p - i as f32);
    let mut eye = [0.; 3];
    for (k, v) in eye.iter_mut().enumerate() {
        *v = catmull(
            EYES[i.saturating_sub(1)][k],
            EYES[i][k],
            EYES[i + 1][k],
            EYES[(i + 2).min(3)][k],
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
pub fn panel_model(i: usize, w: f32, h: f32, pw: f32, ph: f32) -> Mat4 {
    let focal = h * 0.5 / (FOV * 0.5).tan();
    let unit = 10. / focal;
    let mobile = w <= 700.;
    let x = if mobile {
        0.
    } else {
        ((w - pw) * 0.15 + pw * 0.5 - w * 0.5) * unit
    };
    let bottom = if h < 520. { h - 66. } else { h - 134. };
    let desired = if h < 520. {
        h * 0.5
    } else if mobile {
        h * 0.585
    } else {
        h * 0.495
    };
    let center = desired.min(bottom - ph * 0.5);
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
