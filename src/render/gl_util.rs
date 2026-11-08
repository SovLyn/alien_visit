//! Thin WebGL2 helpers over glow. Owned by the director: the sky and terrain
//! passes build on this API, but should not need to change it.

use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};

use glow::HasContext;

pub struct Gpu {
    pub gl: Rc<glow::Context>,
}

impl Gpu {
    pub fn new(canvas: &web_sys::HtmlCanvasElement) -> Result<Self, String> {
        let opts = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&opts, &"alpha".into(), &JsValue::FALSE);
        let _ = js_sys::Reflect::set(&opts, &"antialias".into(), &JsValue::TRUE);
        let _ = js_sys::Reflect::set(&opts, &"depth".into(), &JsValue::TRUE);
        let _ = js_sys::Reflect::set(&opts, &"stencil".into(), &JsValue::FALSE);
        let _ = js_sys::Reflect::set(&opts, &"powerPreference".into(), &"high-performance".into());
        let ctx = canvas
            .get_context_with_context_options("webgl2", &opts)
            .map_err(|e| format!("getContext(webgl2) failed: {e:?}"))?
            .ok_or_else(|| "WebGL2 is not available in this browser.".to_string())?;
        let ctx: web_sys::WebGl2RenderingContext = ctx
            .dyn_into()
            .map_err(|_| "webgl2 context had an unexpected type".to_string())?;
        let gl = unsafe { glow::Context::from_webgl2_context(ctx) };
        Ok(Self { gl: Rc::new(gl) })
    }

    pub fn max_texture_size(&self) -> i32 {
        unsafe { self.gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE).max(1024) }
    }
}

pub struct Program {
    gl: Rc<glow::Context>,
    pub raw: glow::Program,
    uniforms: HashMap<String, Option<glow::UniformLocation>>,
}

impl Program {
    pub fn new(gpu: &Gpu, vs_src: &str, fs_src: &str, label: &str) -> Result<Self, String> {
        unsafe {
            let gl = &gpu.gl;
            let vs = compile_shader(gl, glow::VERTEX_SHADER, vs_src, label)?;
            let fs = compile_shader(gl, glow::FRAGMENT_SHADER, fs_src, label)?;
            let raw = gl
                .create_program()
                .map_err(|e| format!("{label}: create_program: {e}"))?;
            gl.attach_shader(raw, vs);
            gl.attach_shader(raw, fs);
            gl.link_program(raw);
            if !gl.get_program_link_status(raw) {
                let log = gl.get_program_info_log(raw);
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                gl.delete_program(raw);
                return Err(format!("{label}: link error:\n{log}"));
            }
            gl.detach_shader(raw, vs);
            gl.detach_shader(raw, fs);
            gl.delete_shader(vs);
            gl.delete_shader(fs);
            Ok(Self {
                gl: gpu.gl.clone(),
                raw,
                uniforms: HashMap::new(),
            })
        }
    }

    pub fn bind(&self) {
        unsafe { self.gl.use_program(Some(self.raw)) }
    }

    fn loc(&mut self, name: &str) -> Option<&glow::UniformLocation> {
        if !self.uniforms.contains_key(name) {
            let l = unsafe { self.gl.get_uniform_location(self.raw, name) };
            self.uniforms.insert(name.to_string(), l);
        }
        self.uniforms.get(name).and_then(|v| v.as_ref())
    }

    fn loc_idx(&mut self, name: &str) -> Option<&glow::UniformLocation> {
        let key = format!("{name}[0]");
        self.loc(&key)
    }

    pub fn u1f(&mut self, name: &str, v: f32) {
        let gl = self.gl.clone();
        unsafe { gl.uniform_1_f32(self.loc(name), v) }
    }

    pub fn u2f(&mut self, name: &str, x: f32, y: f32) {
        let gl = self.gl.clone();
        unsafe { gl.uniform_2_f32(self.loc(name), x, y) }
    }

    pub fn u3f(&mut self, name: &str, x: f32, y: f32, z: f32) {
        let gl = self.gl.clone();
        unsafe { gl.uniform_3_f32(self.loc(name), x, y, z) }
    }

    pub fn u3v(&mut self, name: &str, v: glam::Vec3) {
        self.u3f(name, v.x, v.y, v.z)
    }

    pub fn u4f(&mut self, name: &str, x: f32, y: f32, z: f32, w: f32) {
        let gl = self.gl.clone();
        unsafe { gl.uniform_4_f32(self.loc(name), x, y, z, w) }
    }

    pub fn u1i(&mut self, name: &str, v: i32) {
        let gl = self.gl.clone();
        unsafe { gl.uniform_1_i32(self.loc(name), v) }
    }

    pub fn mat4(&mut self, name: &str, m: &glam::Mat4) {
        let gl = self.gl.clone();
        unsafe {
            gl.uniform_matrix_4_f32_slice(self.loc(name), false, &m.to_cols_array());
        }
    }

    pub fn vec4_array(&mut self, name: &str, values: &[[f32; 4]]) {
        let gl = self.gl.clone();
        let flat: Vec<f32> = values.iter().flat_map(|v| v.iter().copied()).collect();
        unsafe { gl.uniform_4_f32_slice(self.loc_idx(name), &flat) }
    }

    pub fn vec3_array(&mut self, name: &str, values: &[[f32; 3]]) {
        let gl = self.gl.clone();
        let flat: Vec<f32> = values.iter().flat_map(|v| v.iter().copied()).collect();
        unsafe { gl.uniform_3_f32_slice(self.loc_idx(name), &flat) }
    }

    pub fn f32_array(&mut self, name: &str, values: &[f32]) {
        let gl = self.gl.clone();
        unsafe { gl.uniform_1_f32_slice(self.loc_idx(name), values) }
    }

    pub fn i32_array(&mut self, name: &str, values: &[i32]) {
        let gl = self.gl.clone();
        unsafe { gl.uniform_1_i32_slice(self.loc_idx(name), values) }
    }
}

unsafe fn compile_shader(
    gl: &glow::Context,
    kind: u32,
    src: &str,
    label: &str,
) -> Result<glow::Shader, String> {
    let shader = unsafe { gl.create_shader(kind) }.map_err(|e| format!("{label}: {e}"))?;
    unsafe {
        gl.shader_source(shader, src);
        gl.compile_shader(shader);
    }
    if !unsafe { gl.get_shader_compile_status(shader) } {
        let log = unsafe { gl.get_shader_info_log(shader) };
        let numbered = src
            .lines()
            .enumerate()
            .map(|(i, l)| format!("{:4}: {l}", i + 1))
            .collect::<Vec<_>>()
            .join("\n");
        unsafe { gl.delete_shader(shader) };
        return Err(format!(
            "{label}: shader compile error:\n{log}\n--- source ---\n{numbered}"
        ));
    }
    Ok(shader)
}

pub fn fullscreen_vao(gpu: &Gpu) -> Result<glow::VertexArray, String> {
    unsafe {
        gpu.gl
            .create_vertex_array()
            .map_err(|e| format!("fullscreen VAO: {e}"))
    }
}

pub unsafe fn draw_fullscreen(gpu: &Gpu, vao: glow::VertexArray) {
    unsafe {
        gpu.gl.bind_vertex_array(Some(vao));
        gpu.gl.draw_arrays(glow::TRIANGLES, 0, 3);
        gpu.gl.bind_vertex_array(None);
    }
}

pub fn create_buffer(gpu: &Gpu, _target: u32) -> Result<glow::Buffer, String> {
    unsafe {
        gpu.gl
            .create_buffer()
            .map_err(|e| format!("create_buffer: {e}"))
    }
}

pub fn upload_buffer(gpu: &Gpu, buffer: glow::Buffer, target: u32, data: &[u8], usage: u32) {
    unsafe {
        gpu.gl.bind_buffer(target, Some(buffer));
        gpu.gl.buffer_data_u8_slice(target, data, usage);
    }
}

/// WebGL2 depth texture (used for the sun shadow map).
pub struct DepthTarget {
    pub fbo: glow::Framebuffer,
    pub texture: glow::Texture,
    pub size: i32,
}

impl DepthTarget {
    pub fn new(gpu: &Gpu, size: i32) -> Result<Self, String> {
        unsafe {
            let gl = &gpu.gl;
            let texture = gl
                .create_texture()
                .map_err(|e| format!("depth texture: {e}"))?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::DEPTH_COMPONENT24 as i32,
                size,
                size,
                0,
                glow::DEPTH_COMPONENT,
                glow::UNSIGNED_INT,
                glow::PixelUnpackData::Slice(None),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);

            let fbo = gl
                .create_framebuffer()
                .map_err(|e| format!("shadow framebuffer: {e}"))?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );
            gl.draw_buffers(&[glow::NONE]);
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            if status != glow::FRAMEBUFFER_COMPLETE {
                return Err(format!("shadow framebuffer incomplete: 0x{status:04x}"));
            }
            Ok(Self { fbo, texture, size })
        }
    }

    pub fn bind(&self, gpu: &Gpu) {
        unsafe {
            gpu.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            gpu.gl.viewport(0, 0, self.size, self.size);
        }
    }

    pub fn unbind(gpu: &Gpu) {
        unsafe { gpu.gl.bind_framebuffer(glow::FRAMEBUFFER, None) }
    }
}
