//! One portable WebGL1 packed-depth map. Only a visible diorama owns a target.
use super::{math::Mat4, scene::shader, shadows::Quality};
use wasm_bindgen::JsValue;
use web_sys::{
    WebGlBuffer, WebGlFramebuffer, WebGlProgram, WebGlRenderbuffer, WebGlRenderingContext as Gl,
    WebGlTexture, WebGlUniformLocation,
};

const VERTEX: &str = r#"
attribute vec3 a_pos; uniform mat4 u_light_mvp;
void main(){gl_Position=u_light_mvp*vec4(a_pos,1.0);}
"#;
const FRAGMENT: &str = r#"
precision highp float;
uniform float u_enter;
float bayer2(vec2 p){return 2.0*mod(p.x+p.y,2.0)+mod(p.y,2.0);}
float coverage(vec2 p){p=floor(p);return (16.0*bayer2(mod(p,2.0))+4.0*bayer2(mod(floor(p/2.0),2.0))+bayer2(mod(floor(p/4.0),2.0))+0.5)/64.0;}
void main(){
 if(u_enter<1.0 && coverage(gl_FragCoord.xy)>u_enter)discard;
 vec3 depth=fract(min(gl_FragCoord.z,0.999999)*vec3(1.0,255.0,65025.0));
 depth.xy-=depth.yz/255.0;
 gl_FragColor=vec4(depth,1.0);
}
"#;
struct Target {
    gl: Gl,
    framebuffer: Option<WebGlFramebuffer>,
    depth: Option<WebGlRenderbuffer>,
    texture: Option<WebGlTexture>,
    size: u32,
}
impl Target {
    fn new(gl: &Gl, size: u32) -> Option<Self> {
        let target = Self {
            gl: gl.clone(),
            framebuffer: gl.create_framebuffer(),
            depth: gl.create_renderbuffer(),
            texture: gl.create_texture(),
            size,
        };
        // Drop also cleans up partially allocated targets.
        let (Some(fb), Some(depth), Some(texture)) =
            (&target.framebuffer, &target.depth, &target.texture)
        else {
            return None;
        };
        gl.bind_texture(Gl::TEXTURE_2D, Some(texture));
        for param in [Gl::TEXTURE_MIN_FILTER, Gl::TEXTURE_MAG_FILTER] {
            gl.tex_parameteri(Gl::TEXTURE_2D, param, Gl::NEAREST as i32);
        }
        for param in [Gl::TEXTURE_WRAP_S, Gl::TEXTURE_WRAP_T] {
            gl.tex_parameteri(Gl::TEXTURE_2D, param, Gl::CLAMP_TO_EDGE as i32);
        }
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            Gl::TEXTURE_2D,
            0,
            Gl::RGBA as i32,
            size as i32,
            size as i32,
            0,
            Gl::RGBA,
            Gl::UNSIGNED_BYTE,
            None,
        )
        .ok()?;
        gl.bind_renderbuffer(Gl::RENDERBUFFER, Some(depth));
        gl.renderbuffer_storage(
            Gl::RENDERBUFFER,
            Gl::DEPTH_COMPONENT16,
            size as i32,
            size as i32,
        );
        gl.bind_framebuffer(Gl::FRAMEBUFFER, Some(fb));
        gl.framebuffer_texture_2d(
            Gl::FRAMEBUFFER,
            Gl::COLOR_ATTACHMENT0,
            Gl::TEXTURE_2D,
            Some(texture),
            0,
        );
        gl.framebuffer_renderbuffer(
            Gl::FRAMEBUFFER,
            Gl::DEPTH_ATTACHMENT,
            Gl::RENDERBUFFER,
            Some(depth),
        );
        let complete = gl.check_framebuffer_status(Gl::FRAMEBUFFER) == Gl::FRAMEBUFFER_COMPLETE;
        gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
        gl.bind_renderbuffer(Gl::RENDERBUFFER, None);
        let healthy = gl.get_error() == Gl::NO_ERROR;
        if complete && healthy {
            Some(target)
        } else {
            None
        }
    }
}
impl Drop for Target {
    fn drop(&mut self) {
        self.gl.delete_framebuffer(self.framebuffer.as_ref());
        self.gl.delete_renderbuffer(self.depth.as_ref());
        self.gl.delete_texture(self.texture.as_ref());
    }
}
pub struct ShadowMap {
    gl: Gl,
    program: WebGlProgram,
    matrix: WebGlUniformLocation,
    reveal: WebGlUniformLocation,
    target: Option<Target>,
    quality: Quality,
    hardware_max: u32,
    cached_frame: Option<f32>,
    disabled: bool,
    pub updates: u32,
}
impl ShadowMap {
    pub fn new(gl: &Gl) -> Result<Self, JsValue> {
        let vert = shader(gl, Gl::VERTEX_SHADER, VERTEX)?;
        let frag = match shader(gl, Gl::FRAGMENT_SHADER, FRAGMENT) {
            Ok(frag) => frag,
            Err(e) => {
                gl.delete_shader(Some(&vert));
                return Err(e);
            }
        };
        let program = gl.create_program();
        let Some(program) = program else {
            gl.delete_shader(Some(&vert));
            gl.delete_shader(Some(&frag));
            return Err(JsValue::from_str("No shadow program"));
        };
        gl.attach_shader(&program, &vert);
        gl.attach_shader(&program, &frag);
        gl.bind_attrib_location(&program, 0, "a_pos");
        gl.link_program(&program);
        gl.delete_shader(Some(&vert));
        gl.delete_shader(Some(&frag));
        if !gl
            .get_program_parameter(&program, Gl::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            gl.delete_program(Some(&program));
            return Err(JsValue::from_str("Shadow shader unavailable"));
        }
        let matrix = gl.get_uniform_location(&program, "u_light_mvp");
        let reveal = gl.get_uniform_location(&program, "u_enter");
        let (Some(matrix), Some(reveal)) = (matrix, reveal) else {
            gl.delete_program(Some(&program));
            return Err(JsValue::from_str("Shadow uniforms unavailable"));
        };
        let hardware_max = [Gl::MAX_TEXTURE_SIZE, Gl::MAX_RENDERBUFFER_SIZE]
            .map(|p| {
                gl.get_parameter(p)
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.) as u32
            })
            .into_iter()
            .min()
            .unwrap_or(0);
        Ok(Self {
            gl: gl.clone(),
            program,
            matrix,
            reveal,
            target: None,
            quality: Quality::default(),
            hardware_max,
            cached_frame: None,
            disabled: false,
            updates: 0,
        })
    }
    pub fn observe(&mut self, milliseconds: f64) {
        self.quality.observe(milliseconds, self.size());
    }
    pub fn release(&mut self) {
        self.target = None;
        self.cached_frame = None;
    }
    pub fn size(&self) -> u32 {
        self.target.as_ref().map_or(0, |t| t.size)
    }
    pub fn texture(&self) -> Option<&WebGlTexture> {
        self.target.as_ref().and_then(|t| t.texture.as_ref())
    }
    pub fn begin(&mut self, frame: f32, mobile: bool) -> bool {
        if self.disabled {
            return false;
        }
        let size = self.quality.resolution(mobile, self.hardware_max);
        if size == 0 {
            self.release();
            return false;
        }
        if self.size() != size {
            self.release();
            self.target = Target::new(&self.gl, size);
            if self.target.is_none() {
                self.disabled = true;
                return false;
            }
        }
        if self.cached_frame == Some(frame) {
            return false;
        }
        self.cached_frame = Some(frame);
        self.updates += 1;
        let gl = &self.gl;
        gl.bind_framebuffer(
            Gl::FRAMEBUFFER,
            self.target.as_ref().and_then(|t| t.framebuffer.as_ref()),
        );
        gl.viewport(0, 0, size as i32, size as i32);
        gl.use_program(Some(&self.program));
        gl.disable(Gl::BLEND);
        gl.disable(Gl::DITHER);
        gl.depth_mask(true);
        gl.clear_color(1., 1., 1., 1.);
        gl.clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT);
        gl.enable(Gl::POLYGON_OFFSET_FILL);
        gl.polygon_offset(1., 1.);
        true
    }
    pub fn draw(&self, matrix: Mat4, buffer: &WebGlBuffer, count: i32, reveal: f32) {
        let gl = &self.gl;
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(buffer));
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 3, Gl::FLOAT, false, 48, 0);
        gl.uniform_matrix4fv_with_f32_array(Some(&self.matrix), false, &matrix);
        gl.uniform1f(Some(&self.reveal), reveal);
        gl.draw_arrays(Gl::TRIANGLES, 0, count);
    }
    pub fn finish(&self) {
        self.gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
        self.gl.disable(Gl::POLYGON_OFFSET_FILL);
        self.gl.enable(Gl::BLEND);
        self.gl.enable(Gl::DITHER);
    }
}
impl Drop for ShadowMap {
    fn drop(&mut self) {
        self.gl.delete_program(Some(&self.program));
    }
}
