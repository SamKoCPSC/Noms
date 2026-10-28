use super::flight::{self, Camera};
use super::math::*;
use std::f32::consts::PI;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    CanvasRenderingContext2d, Document, HtmlCanvasElement, HtmlImageElement, WebGlBuffer,
    WebGlProgram, WebGlRenderingContext as Gl, WebGlShader, WebGlTexture, WebGlUniformLocation,
};

const VERTEX: &str = r#"
attribute vec3 a_pos; attribute vec3 a_normal; attribute vec2 a_uv; attribute vec4 a_color;
uniform mat4 u_model; uniform mat4 u_vp; uniform mat3 u_normal;
varying vec3 v_normal; varying vec3 v_world; varying vec2 v_uv; varying vec4 v_color;
void main(){vec4 w=u_model*vec4(a_pos,1.0);v_world=w.xyz;v_normal=normalize(u_normal*a_normal);v_uv=a_uv;v_color=a_color;gl_Position=u_vp*w;}
"#;
const FRAGMENT: &str = r#"
precision mediump float;
uniform vec4 u_color; uniform vec3 u_eye; uniform sampler2D u_texture;
uniform float u_textured; uniform float u_shadow; uniform float u_reveal; uniform float u_diorama;
varying vec3 v_normal; varying vec3 v_world; varying vec2 v_uv; varying vec4 v_color;
// The glTF diorama palettes are scene-linear; display encoding happens after lighting.
vec3 display_srgb(vec3 c){
 c=max(c,vec3(0.0));
 return mix(c*12.92,1.055*pow(c,vec3(1.0/2.4))-0.055,step(vec3(0.0031308),c));
}
// AgX approximation adapted from three.js (MIT); see assets/about/THIRD_PARTY_LICENSES.txt.
// Blender's AgX display treatment softens bright ivory without shifting orange into yellow.
vec3 diorama_agx(vec3 color){
 const mat3 to2020=mat3(0.627404,0.069097,0.0163916,0.329283,0.919540,0.0880132,0.0433136,0.0113612,0.895595);
 const mat3 toSRGB=mat3(1.660491,-0.124550,-0.018151,-0.587641,1.132900,-0.100579,-0.072850,-0.008349,1.118730);
 const mat3 inset=mat3(0.8566271533,0.1373189729,0.1118982130,0.0951212405,0.7612419906,0.0767994186,0.0482516061,0.1014390365,0.8113023684);
 const mat3 outset=mat3(1.1271005818,-0.1413297635,-0.1413297635,-0.1106066431,1.1578237022,-0.1106066431,-0.0164939387,-0.0164939387,1.2519364066);
 vec3 x=clamp((log2(max(inset*to2020*color,vec3(0.00001)))+12.47393)/16.5,0.0,1.0);
 vec3 x2=x*x;vec3 x4=x2*x2;
 x=15.5*x4*x2-40.14*x4*x+31.96*x4-6.868*x2*x+0.4298*x2+0.1191*x-0.00232;
 // A modest contrast/saturation look approximates the master's Medium High Contrast setting.
 x=pow(max(x,vec3(0.0)),vec3(1.25));
 x=mix(vec3(dot(x,vec3(0.2126,0.7152,0.0722))),x,1.50);
 return clamp(toSRGB*pow(max(outset*x,vec3(0.0)),vec3(2.2)),0.0,1.0);
}
void main(){
 if(v_uv.x>u_reveal)discard;
 if(u_shadow>0.5){float r=length(v_uv*2.0-1.0);gl_FragColor=vec4(u_color.rgb,u_color.a*exp(-3.2*r*r)*(1.0-smoothstep(0.65,1.0,r)));return;}
 vec3 n=normalize(v_normal);vec3 light=normalize(vec3(-3.0,7.0,5.0));
 float diffuse=max(dot(n,light),0.0);float hemi=n.y*0.5+0.5;
 vec3 base=u_color.rgb*v_color.rgb;if(u_textured>0.5)base*=texture2D(u_texture,v_uv).rgb;
 if(u_diorama>0.5){
  // Neutral studio fill keeps the master's terracotta, peach, ivory and oak distinct.
  if(u_textured>0.5){
   vec3 tex=texture2D(u_texture,v_uv).rgb;
   base=u_color.rgb*v_color.rgb*mix(tex/12.92,pow((tex+0.055)/1.055,vec3(2.4)),step(vec3(0.04045),tex));
  }
  float key=max(dot(n,normalize(vec3(-3.0,7.0,5.0))),0.0);
  float fill=max(dot(n,normalize(vec3(5.0,3.0,-2.0))),0.0);
  float illumination=0.95+0.55*key+0.20*fill;
  vec3 halfway=normalize(light+normalize(u_eye-v_world));
  float spec=pow(max(dot(n,halfway),0.0),48.0)*0.025;
  vec3 radiance=base*illumination*2.00+vec3(spec);
  gl_FragColor=vec4(display_srgb(diorama_agx(radiance)),u_color.a);
  return;
 }
 vec3 lighting=vec3(0.44,0.45,0.39)+vec3(0.47,0.45,0.39)*diffuse+vec3(0.13)*hemi;
 vec3 halfway=normalize(light+normalize(u_eye-v_world));float spec=pow(max(dot(n,halfway),0.0),36.0)*0.09;
 float fog=1.0-exp(-0.00115*pow(length(u_eye-v_world),2.0));
 gl_FragColor=vec4(mix(base*lighting+spec,vec3(0.933,0.937,0.885),fog),u_color.a);
}
"#;
struct Mesh {
    buffer: WebGlBuffer,
    count: i32,
    colored: bool,
}
struct DioramaDraw {
    mesh: Mesh,
    part: super::gltf::DioramaPart,
}
#[derive(Default)]
struct Diorama {
    draws: Vec<DioramaDraw>,
    texture_base: usize,
}
struct Uniforms {
    model: WebGlUniformLocation,
    vp: WebGlUniformLocation,
    normal: WebGlUniformLocation,
    color: WebGlUniformLocation,
    eye: WebGlUniformLocation,
    textured: WebGlUniformLocation,
    shadow: WebGlUniformLocation,
    reveal: WebGlUniformLocation,
    diorama: WebGlUniformLocation,
}
pub struct Renderer {
    gl: Gl,
    canvas: HtmlCanvasElement,
    program: WebGlProgram,
    uniforms: Uniforms,
    rounded: Mesh,
    sphere: Mesh,
    quad: Mesh,
    platform: Mesh,
    thread: Mesh,
    grid: Mesh,
    arch: Mesh,
    textures: Vec<WebGlTexture>,
    kitchen: Diorama,
    office: Diorama,
    a_pos: u32,
    a_normal: u32,
    a_uv: u32,
    a_color: u32,
}
impl Renderer {
    pub fn new(canvas: HtmlCanvasElement, doc: &Document) -> Result<Self, JsValue> {
        let gl: Gl = canvas
            .get_context("webgl")?
            .ok_or_else(|| JsValue::from_str("WebGL unavailable"))?
            .dyn_into()?;
        let vert = shader(&gl, Gl::VERTEX_SHADER, VERTEX)?;
        let frag = shader(&gl, Gl::FRAGMENT_SHADER, FRAGMENT)?;
        let program = gl
            .create_program()
            .ok_or_else(|| JsValue::from_str("No shader program"))?;
        gl.attach_shader(&program, &vert);
        gl.attach_shader(&program, &frag);
        gl.link_program(&program);
        if !gl
            .get_program_parameter(&program, Gl::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            return Err(JsValue::from_str(
                &gl.get_program_info_log(&program).unwrap_or_default(),
            ));
        }
        gl.delete_shader(Some(&vert));
        gl.delete_shader(Some(&frag));
        gl.use_program(Some(&program));
        let uniform = |name: &str| {
            gl.get_uniform_location(&program, name)
                .ok_or_else(|| JsValue::from_str(name))
        };
        let uniforms = Uniforms {
            model: uniform("u_model")?,
            vp: uniform("u_vp")?,
            normal: uniform("u_normal")?,
            color: uniform("u_color")?,
            eye: uniform("u_eye")?,
            textured: uniform("u_textured")?,
            shadow: uniform("u_shadow")?,
            reveal: uniform("u_reveal")?,
            diorama: uniform("u_diorama")?,
        };
        let a_pos = gl.get_attrib_location(&program, "a_pos") as u32;
        let a_normal = gl.get_attrib_location(&program, "a_normal") as u32;
        let a_uv = gl.get_attrib_location(&program, "a_uv") as u32;
        let a_color = gl.get_attrib_location(&program, "a_color") as u32;
        let rounded = mesh(&gl, &rounded_vertices())?;
        let sphere = mesh(&gl, &sphere_vertices())?;
        let quad = mesh(&gl, &quad_vertices())?;
        let platform = mesh(&gl, &cylinder_vertices())?;
        let thread = mesh(&gl, &route_vertices())?;
        let grid = mesh(&gl, &grid_vertices())?;
        let arch = mesh(&gl, &arch_vertices())?;
        let textures = (0..5)
            .map(|i| texture(&gl, doc, i))
            .collect::<Result<Vec<_>, _>>()?;
        gl.enable(Gl::DEPTH_TEST);
        gl.depth_func(Gl::LEQUAL);
        gl.enable(Gl::BLEND);
        gl.blend_func(Gl::SRC_ALPHA, Gl::ONE_MINUS_SRC_ALPHA);
        gl.clear_color(0., 0., 0., 0.);
        Ok(Self {
            gl,
            canvas,
            program,
            uniforms,
            rounded,
            sphere,
            quad,
            platform,
            thread,
            grid,
            arch,
            textures,
            kitchen: Diorama::default(),
            office: Diorama::default(),
            a_pos,
            a_normal,
            a_uv,
            a_color,
        })
    }
    fn upload_diorama(
        &mut self,
        data: &super::gltf::DioramaMesh,
        images: &[HtmlImageElement],
    ) -> Result<Diorama, JsValue> {
        if data.parts.iter().any(|part| part.texture > images.len()) {
            return Err(JsValue::from_str("Missing diorama texture"));
        }
        let mut diorama = Diorama {
            draws: Vec::new(),
            texture_base: self.textures.len(),
        };
        for image in images {
            self.textures.push(image_texture(&self.gl, image)?);
        }
        for part in &data.parts {
            let mut model = mesh(
                &self.gl,
                &data.vertices[part.first * 12..(part.first + part.count) * 12],
            )?;
            model.count = part.count as i32;
            model.colored = true;
            diorama.draws.push(DioramaDraw {
                mesh: model,
                part: part.clone(),
            });
        }
        // Draw glazing last and without depth writes, exposing the PC interior.
        diorama
            .draws
            .sort_by(|a, b| b.part.opacity.total_cmp(&a.part.opacity));
        Ok(diorama)
    }
    pub fn upload_models(
        &mut self,
        kitchen: &super::gltf::DioramaMesh,
        kitchen_images: &[HtmlImageElement],
        office: &super::gltf::DioramaMesh,
        office_images: &[HtmlImageElement],
    ) -> Result<(), JsValue> {
        self.kitchen = self.upload_diorama(kitchen, kitchen_images)?;
        self.office = self.upload_diorama(office, office_images)?;
        for (name, data) in [("kitchen", kitchen), ("office", office)] {
            let _ = self.canvas.set_attribute(
                &format!("data-{name}-groups"),
                &data
                    .parts
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        Ok(())
    }
    pub fn resize(&self, dpr: f64) {
        let rect = self.canvas.get_bounding_client_rect();
        let w = (rect.width() * dpr.min(1.5)).round().max(1.) as u32;
        let h = (rect.height() * dpr.min(1.5)).round().max(1.) as u32;
        if self.canvas.width() != w || self.canvas.height() != h {
            self.canvas.set_width(w);
            self.canvas.set_height(h);
        }
        self.gl.viewport(0, 0, w as i32, h as i32);
    }
    fn draw(
        &self,
        mesh: &Mesh,
        model: Mat4,
        color: [f32; 4],
        texture: Option<usize>,
        shadow: bool,
        reveal: f32,
    ) {
        let gl = &self.gl;
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&mesh.buffer));
        for (attr, size, offset) in [
            (self.a_pos, 3, 0),
            (self.a_normal, 3, 12),
            (self.a_uv, 2, 24),
        ] {
            gl.enable_vertex_attrib_array(attr);
            gl.vertex_attrib_pointer_with_i32(
                attr,
                size,
                Gl::FLOAT,
                false,
                if mesh.colored { 48 } else { 32 },
                offset,
            );
        }
        if mesh.colored {
            gl.enable_vertex_attrib_array(self.a_color);
            gl.vertex_attrib_pointer_with_i32(self.a_color, 4, Gl::FLOAT, false, 48, 32);
        } else {
            gl.disable_vertex_attrib_array(self.a_color);
            gl.vertex_attrib4f(self.a_color, 1., 1., 1., 1.);
        }
        let mut normal = [0.; 9];
        for c in 0..3 {
            let len2 = model[c * 4] * model[c * 4]
                + model[c * 4 + 1] * model[c * 4 + 1]
                + model[c * 4 + 2] * model[c * 4 + 2];
            for row in 0..3 {
                normal[c * 3 + row] = model[c * 4 + row] / len2.max(0.000001);
            }
        }
        gl.uniform_matrix4fv_with_f32_array(Some(&self.uniforms.model), false, &model);
        gl.uniform_matrix3fv_with_f32_array(Some(&self.uniforms.normal), false, &normal);
        gl.uniform4fv_with_f32_array(Some(&self.uniforms.color), &color);
        gl.uniform1f(
            Some(&self.uniforms.textured),
            if texture.is_some() { 1. } else { 0. },
        );
        gl.uniform1f(Some(&self.uniforms.shadow), if shadow { 1. } else { 0. });
        gl.uniform1f(Some(&self.uniforms.reveal), reveal);
        gl.uniform1f(
            Some(&self.uniforms.diorama),
            if mesh.colored { 1. } else { 0. },
        );
        if let Some(i) = texture {
            gl.bind_texture(Gl::TEXTURE_2D, Some(&self.textures[i]));
        }
        gl.draw_arrays(Gl::TRIANGLES, 0, mesh.count);
    }
    fn block(&self, parent: Mat4, pos: [f32; 3], size: [f32; 3], color: [f32; 4]) {
        self.draw(
            &self.rounded,
            mul(
                parent,
                mul(
                    translate(pos[0], pos[1], pos[2]),
                    scale(size[0], size[1], size[2]),
                ),
            ),
            color,
            None,
            false,
            2.,
        );
    }
    fn ball(&self, parent: Mat4, pos: [f32; 3], size: [f32; 3], color: [f32; 4]) {
        self.draw(
            &self.sphere,
            mul(
                parent,
                mul(
                    translate(pos[0], pos[1], pos[2]),
                    scale(size[0], size[1], size[2]),
                ),
            ),
            color,
            None,
            false,
            2.,
        );
    }
    fn book(&self, root: Mat4, open: f32, tint: [f32; 4]) {
        let cream = [0.94, 0.90, 0.75, 1.];
        let dark = [tint[0] * 0.67, tint[1] * 0.69, tint[2] * 0.65, 1.];
        self.block(root, [0., -0.24, 0.], [1.30, 0.065, 1.73], tint);
        self.block(root, [0.025, 0., 0.], [1.23, 0.20, 1.64], cream);
        // Individual paper edges make the book read as a volume at close range.
        for i in 0..6 {
            self.block(
                root,
                [0.05, -0.15 + i as f32 * 0.055, 0.],
                [1.232, 0.008, 1.643],
                [0.82, 0.78, 0.64, 1.],
            );
        }
        self.block(root, [-1.23, 0., 0.], [0.105, 0.26, 1.71], dark);
        self.draw(
            &self.quad,
            mul(
                root,
                transform([0.05, 0.205, 0.], [0., 0., 0.], [1.07, 1., 1.42]),
            ),
            [1., 1., 1., 1.],
            Some(3),
            false,
            2.,
        );
        self.block(
            root,
            [0.81, 0.195, 1.61],
            [0.11, 0.014, 0.49],
            [0.77, 0.35, 0.23, 1.],
        );
        let cover = mul(
            root,
            mul(
                mul(translate(-1.25, 0.255, 0.), rz(open)),
                translate(1.25, 0., 0.),
            ),
        );
        self.block(cover, [0., 0., 0.], [1.30, 0.065, 1.73], tint);
        self.draw(
            &self.quad,
            mul(
                cover,
                transform([0.04, -0.072, 0.], [PI, 0., 0.], [1.10, 1., 1.48]),
            ),
            [1., 1., 1., 1.],
            Some(2),
            false,
            2.,
        );
        self.block(
            cover,
            [-1.12, 0.063, 0.],
            [0.025, 0.009, 1.60],
            [0.35, 0.43, 0.23, 1.],
        );
        self.draw(
            &self.quad,
            mul(
                cover,
                transform([0.08, 0.072, -0.12], [0., 0., 0.], [0.83, 1., 1.12]),
            ),
            [1., 1., 1., 1.],
            Some(0),
            false,
            2.,
        );
    }
    fn card(&self, root: Mat4, kind: usize) {
        self.block(
            root,
            [0., 0., 0.],
            [0.83, 0.026, 0.64],
            [0.96, 0.92, 0.79, 1.],
        );
        self.draw(
            &self.quad,
            mul(
                root,
                transform([0., 0.029, 0.], [0., 0., 0.], [0.76, 1., 0.57]),
            ),
            [1., 1., 1., 1.],
            Some(kind),
            false,
            2.,
        );
    }
    pub fn render(
        &self,
        camera: &Camera,
        time: f32,
        motion: bool,
        mobile: bool,
        assembly_frame: f32,
        office_frame: f32,
    ) {
        let gl = &self.gl;
        gl.use_program(Some(&self.program));
        gl.clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT);
        gl.uniform_matrix4fv_with_f32_array(Some(&self.uniforms.vp), false, &camera.vp);
        gl.uniform3fv_with_f32_array(Some(&self.uniforms.eye), &camera.eye);
        let base = identity();
        // A continuous floor, route, and distant architecture make translation unmistakable.
        self.draw(
            &self.quad,
            transform([0., -1.55, -36.], [0.; 3], [55., 1., 88.]),
            [0.91, 0.92, 0.85, 1.],
            None,
            false,
            2.,
        );
        self.draw(&self.grid, base, [0.80, 0.84, 0.74, 1.], None, false, 2.);
        self.draw(&self.thread, base, [0.74, 0.40, 0.26, 1.], None, false, 2.);
        for i in 0..flight::COUNT {
            let artwork = [0, 1, 3, 4][i];
            let root = flight::station(i);
            if !(-42.0..=6.0).contains(&(flight::point(root, [0., 0., -10.])[2] - camera.eye[2])) {
                continue;
            }
            let color = if i % 2 == 0 {
                [0.70, 0.77, 0.58, 1.]
            } else {
                [0.80, 0.67, 0.51, 1.]
            };
            // Each arch belongs to the world; the camera flies through it en route.
            self.draw(&self.arch, root, color, None, false, 2.);
            if i <= 1 {
                let ratio = self.canvas.width() as f32 / self.canvas.height().max(1) as f32;
                let short_mobile = self.canvas.get_bounding_client_rect().height() < 600.;
                let (x, y, sc) = if mobile && i == 1 {
                    if short_mobile {
                        (0., 3.1, 0.28)
                    } else {
                        (0., 3.25, 0.33)
                    }
                } else if mobile {
                    (0., 2.7, 0.34)
                } else {
                    (ratio * 2.35, -1.45, 0.78)
                };
                let pose = if i == 0 {
                    transform([x, y, -11.7], [0.58, -0.62, 0.], [sc; 3])
                } else {
                    // Turn the room before pitching it, keeping vertical walls upright.
                    mul(
                        translate(x, y, -11.7),
                        mul(rx(0.58), mul(ry(0.62), scale(sc, sc, sc))),
                    )
                };
                let model = mul(root, pose);
                self.draw_diorama(
                    model,
                    if i == 0 { &self.kitchen } else { &self.office },
                    if i == 0 { assembly_frame } else { office_frame },
                    i == 1,
                );
                continue;
            }
            // The creator now has its own placeholder exhibit; the kitchen belongs to the idea.
            let bob = if motion {
                (time * 0.65 + i as f32).sin() * 0.045
            } else {
                0.
            };
            let ratio = self.canvas.width() as f32 / self.canvas.height().max(1) as f32;
            let display_x = if mobile { 0. } else { ratio * 2.6 };
            let display_y = if mobile && artwork == 2 {
                2.15
            } else if mobile {
                if self.canvas.get_bounding_client_rect().height() < 780. {
                    3.2
                } else {
                    2.7
                }
            } else {
                -0.1
            };
            let sc = if mobile && artwork == 2 {
                0.44
            } else if mobile {
                0.63
            } else {
                1.1
            };
            let exhibit = mul(
                root,
                transform(
                    [display_x, display_y + bob, -10.4],
                    [0.53, -0.40 + i as f32 * 0.04, -0.08],
                    [sc; 3],
                ),
            );
            let near = 1. - ((camera.progress - i as f32).abs() / 0.9).min(1.);
            self.book(
                exhibit,
                if artwork == 2 { 1.7 * near } else { 0.13 },
                if artwork == 3 {
                    [0.69, 0.41, 0.28, 1.]
                } else {
                    [0.40, 0.51, 0.28, 1.]
                },
            );
            if !mobile {
                self.draw(
                    &self.platform,
                    mul(
                        root,
                        transform([display_x, -2.55, -10.4], [0., 0., 0.], [2.5, 0.14, 2.5]),
                    ),
                    [0.81, 0.86, 0.71, 1.],
                    None,
                    false,
                    2.,
                );
                self.draw(
                    &self.platform,
                    mul(
                        root,
                        transform([display_x, -3.3, -10.4], [0., 0., 0.], [1.1, 0.8, 1.1]),
                    ),
                    [0.72, 0.78, 0.62, 1.],
                    None,
                    false,
                    2.,
                );
            }
            for j in 0..3 {
                let t = j as f32;
                self.card(
                    mul(
                        root,
                        transform(
                            [
                                display_x + (t - 1.) * 2.35 * sc,
                                display_y + 1.6 * sc + bob,
                                -12. - t * 0.25,
                            ],
                            [0.85, -0.4 + t * 0.3, 0.08 - t * 0.08],
                            [0.65 * sc; 3],
                        ),
                    ),
                    j + 1,
                );
            }
            if artwork == 4 {
                for j in 0..2 {
                    let x = display_x + if j == 0 { -2.3 } else { 2.1 };
                    self.book(
                        mul(
                            root,
                            transform([x, display_y - 0.8, -11.], [0.55, -0.3, 0.1], [0.45; 3]),
                        ),
                        0.,
                        [0.67, 0.45, 0.30, 1.],
                    );
                }
            }
            // Sculptural herbs and ingredients at different depths provide near-field parallax.
            let herb = mul(root, translate(-7., -2.2, -6.));
            self.draw(
                &self.platform,
                mul(herb, scale(0.6, 0.7, 0.6)),
                [0.73, 0.43, 0.30, 1.],
                None,
                false,
                2.,
            );
            for k in 0..7 {
                let t = k as f32;
                self.ball(
                    mul(herb, ry(t * 1.7)),
                    [0.3, 0.9 + t * 0.12, 0.],
                    [0.9, 0.1, 0.3],
                    [0.42, 0.55, 0.31, 1.],
                );
            }
            self.ball(root, [6.8, -2.2, -4.], [0.42; 3], [0.77, 0.39, 0.25, 1.]);
            self.ball(
                root,
                [6.8, -1.8, -4.],
                [0.18, 0.04, 0.18],
                [0.36, 0.49, 0.24, 1.],
            );
            self.card(
                mul(
                    root,
                    transform([-5.3, 1.8, -3.], [1.2, 0.4, -0.2], [0.8; 3]),
                ),
                1,
            );
        }
    }
    fn draw_diorama(&self, model: Mat4, diorama: &Diorama, frame: f32, office: bool) {
        for draw in &diorama.draws {
            let p = &draw.part;
            let t = if office {
                super::assembly::office_amount(frame, p.start, p.end)
            } else {
                super::assembly::amount(frame, p.start, p.end)
            };
            if t <= 0. {
                continue;
            }
            let pose = super::assembly::arrival_pose(p.pivot, p.offset, t, office);
            let local = mul(
                translate(
                    pose.translation[0],
                    pose.translation[1],
                    pose.translation[2],
                ),
                scale(pose.scale, pose.scale, pose.scale),
            );
            self.gl.depth_mask(p.opacity >= 1.);
            self.draw(
                &draw.mesh,
                mul(model, local),
                [1., 1., 1., p.opacity],
                if p.texture > 0 {
                    Some(diorama.texture_base + p.texture - 1)
                } else {
                    None
                },
                false,
                2.,
            );
        }
        self.gl.depth_mask(true);
    }
    /// Warm both rooms, including prints and transparent PC surfaces, before entry.
    pub fn warm_models(&self, aspect: f32, mobile: bool) -> bool {
        for stop in [0., 1.] {
            self.render(
                &flight::camera(stop, aspect, [0.; 2]),
                0.,
                false,
                mobile,
                160.,
                160.,
            );
        }
        self.gl.finish();
        self.error() == Gl::NO_ERROR
    }
    pub fn error(&self) -> u32 {
        self.gl.get_error()
    }
}
impl Drop for Renderer {
    fn drop(&mut self) {
        for m in [
            &self.rounded,
            &self.sphere,
            &self.quad,
            &self.platform,
            &self.thread,
            &self.grid,
            &self.arch,
        ] {
            self.gl.delete_buffer(Some(&m.buffer));
        }
        for draw in self.kitchen.draws.iter().chain(&self.office.draws) {
            self.gl.delete_buffer(Some(&draw.mesh.buffer));
        }
        for t in &self.textures {
            self.gl.delete_texture(Some(t));
        }
        self.gl.delete_program(Some(&self.program));
    }
}
fn shader(gl: &Gl, kind: u32, source: &str) -> Result<WebGlShader, JsValue> {
    let s = gl
        .create_shader(kind)
        .ok_or_else(|| JsValue::from_str("No shader"))?;
    gl.shader_source(&s, source);
    gl.compile_shader(&s);
    if gl
        .get_shader_parameter(&s, Gl::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(s)
    } else {
        Err(JsValue::from_str(
            &gl.get_shader_info_log(&s).unwrap_or_default(),
        ))
    }
}
fn mesh(gl: &Gl, v: &[f32]) -> Result<Mesh, JsValue> {
    let buffer = gl
        .create_buffer()
        .ok_or_else(|| JsValue::from_str("No vertex buffer"))?;
    gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&buffer));
    let data = js_sys::Float32Array::from(v);
    gl.buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &data, Gl::STATIC_DRAW);
    Ok(Mesh {
        buffer,
        count: (v.len() / 8) as i32,
        colored: false,
    })
}
fn vertex(v: &mut Vec<f32>, p: [f32; 3], n: [f32; 3], uv: [f32; 2]) {
    v.extend_from_slice(&[p[0], p[1], p[2], n[0], n[1], n[2], uv[0], uv[1]]);
}
fn rounded_vertices() -> Vec<f32> {
    let mut v = vec![];
    for axis in 0..3 {
        for sign in [-1., 1.] {
            for i in 0..4 {
                for j in 0..4 {
                    for (u, w) in [
                        (i, j),
                        (i + 1, j),
                        (i + 1, j + 1),
                        (i, j),
                        (i + 1, j + 1),
                        (i, j + 1),
                    ] {
                        let mut p = [0.; 3];
                        p[axis] = sign;
                        p[(axis + 1) % 3] = u as f32 * 0.5 - 1.;
                        p[(axis + 2) % 3] = w as f32 * 0.5 - 1.;
                        let c = p.map(|x| x.clamp(-0.86, 0.86));
                        let n = norm([p[0] - c[0], p[1] - c[1], p[2] - c[2]]);
                        vertex(
                            &mut v,
                            [c[0] + n[0] * 0.14, c[1] + n[1] * 0.14, c[2] + n[2] * 0.14],
                            n,
                            [0., 0.],
                        );
                    }
                }
            }
        }
    }
    v
}
fn sphere_vertices() -> Vec<f32> {
    let mut v = vec![];
    for i in 0..10 {
        for j in 0..16 {
            for (a, b) in [
                (i, j),
                (i + 1, j),
                (i + 1, j + 1),
                (i, j),
                (i + 1, j + 1),
                (i, j + 1),
            ] {
                let t = a as f32 / 10. * PI;
                let p = b as f32 / 16. * PI * 2.;
                let n = [t.sin() * p.cos(), t.cos(), t.sin() * p.sin()];
                vertex(&mut v, n, n, [0., 0.]);
            }
        }
    }
    v
}
fn quad_vertices() -> Vec<f32> {
    let mut v = vec![];
    for (x, z) in [
        (-1., -1.),
        (1., -1.),
        (1., 1.),
        (-1., -1.),
        (1., 1.),
        (-1., 1.),
    ] {
        vertex(
            &mut v,
            [x, 0., z],
            [0., 1., 0.],
            [(x + 1.) * 0.5, (1. - z) * 0.5],
        );
    }
    v
}
fn cylinder_vertices() -> Vec<f32> {
    let mut v = vec![];
    for i in 0..64 {
        let a = i as f32 / 64. * PI * 2.;
        let b = (i + 1) as f32 / 64. * PI * 2.;
        let p = [a.cos(), a.sin()];
        let q = [b.cos(), b.sin()];
        vertex(&mut v, [0., 1., 0.], [0., 1., 0.], [0., 0.]);
        vertex(&mut v, [p[0], 1., p[1]], [0., 1., 0.], [0., 0.]);
        vertex(&mut v, [q[0], 1., q[1]], [0., 1., 0.], [0., 0.]);
        for (x, y, z) in [
            (p[0], -1., p[1]),
            (q[0], -1., q[1]),
            (q[0], 1., q[1]),
            (p[0], -1., p[1]),
            (q[0], 1., q[1]),
            (p[0], 1., p[1]),
        ] {
            vertex(&mut v, [x, y, z], [x, 0., z], [0., 0.]);
        }
    }
    v
}
fn grid_vertices() -> Vec<f32> {
    let mut v = vec![];
    for k in 0..37 {
        let z = 18. - k as f32 * 3.;
        for p in [
            [-28., z - 0.012],
            [28., z - 0.012],
            [28., z + 0.012],
            [-28., z - 0.012],
            [28., z + 0.012],
            [-28., z + 0.012],
        ] {
            vertex(&mut v, [p[0], -1.338, p[1]], [0., 1., 0.], [0.; 2]);
        }
    }
    for k in -8..9 {
        let x = k as f32 * 3.;
        for p in [
            [x - 0.012, 32.],
            [x + 0.012, 32.],
            [x + 0.012, -104.],
            [x - 0.012, 32.],
            [x + 0.012, -104.],
            [x - 0.012, -104.],
        ] {
            vertex(&mut v, [p[0], -1.338, p[1]], [0., 1., 0.], [0.; 2]);
        }
    }
    v
}
fn arch_vertices() -> Vec<f32> {
    let mut v = vec![];
    let mut points = vec![[6.7, -4.9, -15.]];
    for i in 0..=40 {
        let a = i as f32 / 40. * PI;
        points.push([6.7 * a.cos(), 3.9 + 6.7 * a.sin(), -15.]);
    }
    points.push([-6.7, -4.9, -15.]);
    for pair in points.windows(2) {
        let a = pair[0];
        let b = pair[1];
        let side = norm([b[1] - a[1], a[0] - b[0], 0.]);
        for j in 0..12 {
            for (p, k) in [(a, j), (b, j), (b, j + 1), (a, j), (b, j + 1), (a, j + 1)] {
                let t = k as f32 / 12. * 2. * PI;
                let n = [side[0] * t.cos(), side[1] * t.cos(), t.sin()];
                vertex(
                    &mut v,
                    [p[0] + n[0] * 0.12, p[1] + n[1] * 0.12, p[2] + n[2] * 0.12],
                    n,
                    [0.; 2],
                );
            }
        }
    }
    v
}
fn route_vertices() -> Vec<f32> {
    let mut v = vec![];
    for i in 0..240 {
        let t = i as f32 / 240.;
        let u = (i + 1) as f32 / 240.;
        let a = flight::camera(t * (flight::COUNT - 1) as f32, 1., [0.; 2]).eye;
        let b = flight::camera(u * (flight::COUNT - 1) as f32, 1., [0.; 2]).eye;
        for (p, side) in [(a, -1.), (b, -1.), (b, 1.), (a, -1.), (b, 1.), (a, 1.)] {
            vertex(
                &mut v,
                [p[0] + side * 0.055, -1.31, p[2]],
                [0., 1., 0.],
                [t, 0.],
            );
        }
    }
    v
}
fn texture(gl: &Gl, doc: &Document, kind: usize) -> Result<WebGlTexture, JsValue> {
    let canvas: HtmlCanvasElement = doc.create_element("canvas")?.dyn_into()?;
    canvas.set_width(512);
    canvas.set_height(512);
    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("No 2D canvas"))?
        .dyn_into()?;
    ctx.set_fill_style_str("#f4ecd1");
    ctx.fill_rect(0., 0., 512., 512.);
    ctx.set_stroke_style_str("#d9cba6");
    ctx.set_line_width(2.);
    ctx.stroke_rect(20., 20., 472., 472.);
    if kind == 0 {
        ctx.set_fill_style_str("#455635");
        ctx.set_font("500 100px Fredoka, sans-serif");
        ctx.set_text_align("center");
        ctx.fill_text("NOMS", 256., 235.)?;
        ctx.set_font("600 19px Nunito, sans-serif");
        ctx.fill_text("THE RECIPE ARCHIVE", 256., 295.)?;
        ctx.set_fill_style_str("#a47e52");
        ctx.fill_rect(213., 337., 86., 2.);
        ctx.set_font("400 21px Nunito, sans-serif");
        ctx.fill_text("Keep the good recipes.", 256., 394.)?;
    } else {
        let (top, title, lines) = match kind {
            1 => (
                "KITCHEN NOTES",
                "A little more garlic.",
                [
                    "Less sugar. More lemon.",
                    "Keep the small discoveries.",
                    "Make it yours.",
                ],
            ),
            2 => (
                "THE NEXT VERSION",
                "Try. Taste. Refine.",
                [
                    "Save the original.",
                    "Remember what changed.",
                    "Cook it again.",
                ],
            ),
            3 => (
                "FROM YOUR RECIPE BOOK",
                "Something worth keeping.",
                [
                    "A favorite dish.",
                    "A personal touch.",
                    "A recipe to share.",
                ],
            ),
            _ => (
                "MADE WITH CARE",
                "For every kind of cook.",
                ["Your recipes.", "Your process.", "Your community."],
            ),
        };
        ctx.set_text_align("left");
        ctx.set_fill_style_str("#a5704c");
        ctx.set_font("700 20px Nunito, sans-serif");
        ctx.fill_text(top, 48., 80.)?;
        ctx.set_fill_style_str("#465536");
        ctx.set_font("500 29px Fredoka, sans-serif");
        ctx.fill_text(title, 48., 151.)?;
        ctx.set_fill_style_str("#8c8f73");
        ctx.set_font("400 24px Nunito, sans-serif");
        for (i, line) in lines.iter().enumerate() {
            ctx.fill_text(line, 48., 220. + i as f64 * 60.)?;
            ctx.set_fill_style_str("#ddd5b9");
            ctx.fill_rect(48., 238. + i as f64 * 60., 416., 1.);
            ctx.set_fill_style_str("#8c8f73");
        }
        ctx.set_fill_style_str("#ba805b");
        ctx.fill_rect(48., 432., 110., 5.);
    }
    let tex = gl
        .create_texture()
        .ok_or_else(|| JsValue::from_str("No texture"))?;
    gl.bind_texture(Gl::TEXTURE_2D, Some(&tex));
    gl.pixel_storei(Gl::UNPACK_FLIP_Y_WEBGL, 1);
    gl.tex_image_2d_with_u32_and_u32_and_canvas(
        Gl::TEXTURE_2D,
        0,
        Gl::RGBA as i32,
        Gl::RGBA,
        Gl::UNSIGNED_BYTE,
        &canvas,
    )?;
    gl.generate_mipmap(Gl::TEXTURE_2D);
    gl.tex_parameteri(
        Gl::TEXTURE_2D,
        Gl::TEXTURE_MIN_FILTER,
        Gl::LINEAR_MIPMAP_LINEAR as i32,
    );
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
    Ok(tex)
}

fn image_texture(gl: &Gl, image: &HtmlImageElement) -> Result<WebGlTexture, JsValue> {
    let tex = gl
        .create_texture()
        .ok_or_else(|| JsValue::from_str("No diorama texture"))?;
    gl.bind_texture(Gl::TEXTURE_2D, Some(&tex));
    // glTF UVs have their origin at the top of the supplied print image.
    gl.pixel_storei(Gl::UNPACK_FLIP_Y_WEBGL, 0);
    gl.tex_image_2d_with_u32_and_u32_and_image(
        Gl::TEXTURE_2D,
        0,
        Gl::RGBA as i32,
        Gl::RGBA,
        Gl::UNSIGNED_BYTE,
        image,
    )?;
    gl.generate_mipmap(Gl::TEXTURE_2D);
    gl.tex_parameteri(
        Gl::TEXTURE_2D,
        Gl::TEXTURE_MIN_FILTER,
        Gl::LINEAR_MIPMAP_LINEAR as i32,
    );
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
    Ok(tex)
}
