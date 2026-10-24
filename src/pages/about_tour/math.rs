pub type Mat4 = [f32; 16];
pub fn identity() -> Mat4 {
    [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ]
}
pub fn mul(a: Mat4, b: Mat4) -> Mat4 {
    let mut r = [0.; 16];
    for c in 0..4 {
        for row in 0..4 {
            for k in 0..4 {
                r[c * 4 + row] += a[k * 4 + row] * b[c * 4 + k];
            }
        }
    }
    r
}
pub fn translate(x: f32, y: f32, z: f32) -> Mat4 {
    let mut m = identity();
    m[12] = x;
    m[13] = y;
    m[14] = z;
    m
}
pub fn scale(x: f32, y: f32, z: f32) -> Mat4 {
    let mut m = identity();
    m[0] = x;
    m[5] = y;
    m[10] = z;
    m
}
pub fn rx(t: f32) -> Mat4 {
    let (s, c) = t.sin_cos();
    [1., 0., 0., 0., 0., c, s, 0., 0., -s, c, 0., 0., 0., 0., 1.]
}
pub fn ry(t: f32) -> Mat4 {
    let (s, c) = t.sin_cos();
    [c, 0., -s, 0., 0., 1., 0., 0., s, 0., c, 0., 0., 0., 0., 1.]
}
pub fn rz(t: f32) -> Mat4 {
    let (s, c) = t.sin_cos();
    [c, s, 0., 0., -s, c, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.]
}
pub fn transform(pos: [f32; 3], rot: [f32; 3], size: [f32; 3]) -> Mat4 {
    mul(
        mul(
            mul(
                mul(translate(pos[0], pos[1], pos[2]), ry(rot[1])),
                rx(rot[0]),
            ),
            rz(rot[2]),
        ),
        scale(size[0], size[1], size[2]),
    )
}
pub fn norm(v: [f32; 3]) -> [f32; 3] {
    let d = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2])
        .sqrt()
        .max(0.00001);
    [v[0] / d, v[1] / d, v[2] / d]
}
pub fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn look_at(eye: [f32; 3], target: [f32; 3]) -> Mat4 {
    let z = norm([eye[0] - target[0], eye[1] - target[1], eye[2] - target[2]]);
    let x = norm(cross([0., 1., 0.], z));
    let y = cross(z, x);
    [
        x[0],
        y[0],
        z[0],
        0.,
        x[1],
        y[1],
        z[1],
        0.,
        x[2],
        y[2],
        z[2],
        0.,
        -dot(x, eye),
        -dot(y, eye),
        -dot(z, eye),
        1.,
    ]
}
pub fn perspective(fov: f32, aspect: f32) -> Mat4 {
    let f = 1. / (fov * 0.5).tan();
    let (n, far) = (0.1, 100.);
    [
        f / aspect,
        0.,
        0.,
        0.,
        0.,
        f,
        0.,
        0.,
        0.,
        0.,
        (far + n) / (n - far),
        -1.,
        0.,
        0.,
        2. * far * n / (n - far),
        0.,
    ]
}
pub fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
