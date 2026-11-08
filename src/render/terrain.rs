//! Camera-following polar terrain and directional sun shadow map.
use super::gl_util::{DepthTarget, Gpu, Program};
use crate::{
    frame::{BodyId, RenderFrame},
    settings::Quality,
    terrain_field,
};
use glam::{Mat4, Vec3};
use glow::HasContext;
use std::rc::Rc;

const VS: &str = include_str!("terrain_vertex.glsl");
const FS: &str = include_str!("terrain.glsl");
const SHADOW_VS: &str = include_str!("terrain_shadow.glsl");
const SHADOW_FS: &str = "#version 300 es\nprecision highp float;\nvoid main() {}\n";

pub struct TerrainPass {
    gl: Rc<glow::Context>,
    program: Program,
    shadow_program: Program,
    vao: glow::VertexArray,
    vertices: glow::Buffer,
    indices: glow::Buffer,
    shadow: DepthTarget,
    quality: Quality,
    center: Option<[f64; 2]>,
    count: i32,
    logged: bool,
}

fn bytes<T>(data: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr().cast(), std::mem::size_of_val(data)) }
}

impl TerrainPass {
    pub fn new(gpu: &Gpu, quality: Quality) -> Result<Self, String> {
        let program = Program::new(gpu, VS, FS, "terrain")?;
        let shadow_program = Program::new(gpu, SHADOW_VS, SHADOW_FS, "terrain shadow")?;
        let shadow = DepthTarget::new(gpu, quality.shadow_size().min(gpu.max_texture_size()))?;
        let gl = &gpu.gl;
        unsafe {
            let vao = gl.create_vertex_array()?;
            let vertices = gl.create_buffer()?;
            let indices = gl.create_buffer()?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertices));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(indices));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 24, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 24, 12);
            gl.bind_vertex_array(None);
            Ok(Self {
                gl: gl.clone(),
                program,
                shadow_program,
                vao,
                vertices,
                indices,
                shadow,
                quality,
                center: None,
                count: 0,
                logged: false,
            })
        }
    }
    pub fn set_quality(
        &mut self,
        gpu: &Gpu,
        quality: Quality,
        _width: i32,
        _height: i32,
        _render_scale: f32,
    ) {
        if quality == self.quality {
            return;
        }
        match DepthTarget::new(gpu, quality.shadow_size().min(gpu.max_texture_size())) {
            Ok(target) => {
                unsafe {
                    self.gl.delete_framebuffer(self.shadow.fbo);
                    self.gl.delete_texture(self.shadow.texture);
                }
                self.shadow = target;
            }
            Err(e) => {
                web_sys::console::error_1(&format!("[ossa] terrain shadow resize: {e}").into())
            }
        }
        self.quality = quality;
        self.center = None;
    }
    fn rebuild(&mut self, frame: &RenderFrame) {
        let performance = web_sys::window().and_then(|w| w.performance());
        let start = performance.as_ref().map_or(0.0, |p| p.now());
        let (angles, rings) = self.quality.terrain_grid();
        let (a, r) = (angles as usize, rings as usize);
        let center = frame.surface_pos_m;
        let mut positions = Vec::with_capacity(1 + a * r);
        positions.push(Vec3::new(
            0.0,
            terrain_field::height_m(center[0], center[1]),
            0.0,
        ));
        let directions: Vec<_> = (0..a)
            .map(|i| (std::f32::consts::TAU * i as f32 / a as f32).sin_cos())
            .collect();
        for j in 0..r {
            let radius = 1.5_f32 * (30000.0_f32 / 1.5).powf(j as f32 / (r - 1) as f32);
            for &(s, c) in &directions {
                let x = radius * s;
                let z = -radius * c;
                positions.push(Vec3::new(
                    x,
                    terrain_field::height_m(center[0] + x as f64, center[1] - z as f64),
                    z,
                ));
            }
        }
        let mut data = Vec::<f32>::with_capacity(positions.len() * 6);
        for (idx, p) in positions.iter().enumerate() {
            let normal = if idx == 0 {
                Vec3::Y
            } else {
                let j = (idx - 1) / a;
                let i = (idx - 1) % a;
                let prev = positions[1 + j * a + (i + a - 1) % a];
                let next = positions[1 + j * a + (i + 1) % a];
                let inner = if j == 0 {
                    positions[0]
                } else {
                    positions[1 + (j - 1) * a + i]
                };
                let outer = positions[1 + (j + 1).min(r - 1) * a + i];
                (next - prev).cross(outer - inner).normalize_or_zero()
            };
            data.extend_from_slice(&[p.x, p.y, p.z, normal.x, normal.y, normal.z]);
        }
        let mut indices = Vec::<u32>::with_capacity(a * (r - 1) * 6 + a * 3);
        for i in 0..a {
            indices.extend_from_slice(&[0, (1 + i) as u32, (1 + (i + 1) % a) as u32]);
        }
        for j in 0..r - 1 {
            for i in 0..a {
                let n = (i + 1) % a;
                let x = (1 + j * a + i) as u32;
                let y = (1 + j * a + n) as u32;
                let z = (1 + (j + 1) * a + i) as u32;
                let w = (1 + (j + 1) * a + n) as u32;
                indices.extend_from_slice(&[x, z, y, y, z, w]);
            }
        }
        unsafe {
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertices));
            self.gl
                .buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&data), glow::DYNAMIC_DRAW);
            self.gl
                .bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.indices));
            self.gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytes(&indices),
                glow::DYNAMIC_DRAW,
            );
            self.gl.bind_vertex_array(None);
        }
        self.count = indices.len() as i32;
        self.center = Some(center);
        if !self.logged {
            web_sys::console::log_1(
                &format!(
                    "[ossa] terrain {:?} mesh build + upload {:.1} ms",
                    self.quality,
                    performance.as_ref().map_or(0.0, |p| p.now() - start)
                )
                .into(),
            );
            self.logged = true;
        }
    }
    pub fn render(&mut self, gpu: &Gpu, frame: &RenderFrame, width: i32, height: i32) {
        let needs = self.center.map_or(true, |p| {
            (p[0] - frame.surface_pos_m[0]).powi(2) + (p[1] - frame.surface_pos_m[1]).powi(2)
                > 96.0 * 96.0
        });
        if needs {
            self.rebuild(frame);
        }
        let center = self.center.unwrap();
        let offset = Vec3::new(
            (center[0] - frame.surface_pos_m[0]) as f32,
            -frame.eye_height_m,
            -(center[1] - frame.surface_pos_m[1]) as f32,
        );
        let sun = frame.bodies[BodyId::Kesh.index()]
            .rel_pos_km
            .normalize_or_zero();
        let vhal = frame.bodies[BodyId::Vhal.index()]
            .rel_pos_km
            .normalize_or_zero();
        let light = if sun.length_squared() > 0.1 {
            sun
        } else {
            Vec3::Y
        };
        let light_up = if light.dot(Vec3::Y).abs() > 0.98 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        let light_view = Mat4::look_at_rh(light * 6000.0, Vec3::ZERO, light_up);
        let light_vp =
            Mat4::orthographic_rh_gl(-2200.0, 2200.0, -2200.0, 2200.0, 1.0, 12000.0) * light_view;
        let view = Mat4::look_to_rh(Vec3::ZERO, frame.camera.forward, frame.camera.up);
        let vp = Mat4::perspective_rh_gl(
            frame.camera.fov_y_deg.to_radians(),
            frame.camera.aspect.max(0.01),
            0.2,
            40000.0,
        ) * view;
        unsafe {
            let gl = &self.gl;
            self.shadow.bind(gpu);
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::BLEND);
            gl.clear_depth_f32(1.0);
            gl.clear(glow::DEPTH_BUFFER_BIT);
            self.shadow_program.bind();
            self.shadow_program.mat4("u_light_vp", &light_vp);
            self.shadow_program.u3v("u_offset", offset);
            gl.bind_vertex_array(Some(self.vao));
            gl.draw_elements(glow::TRIANGLES, self.count, glow::UNSIGNED_INT, 0);
            DepthTarget::unbind(gpu);
            gl.viewport(0, 0, width, height);
            self.program.bind();
            self.program.mat4("u_vp", &vp);
            self.program.mat4("u_light_vp", &light_vp);
            self.program.u3v("u_offset", offset);
            self.program.u3v("u_sun", light);
            self.program.u3v("u_vhal", vhal);
            self.program
                .u2f("u_origin", center[0] as f32, center[1] as f32);
            self.program.u1f("u_exposure", frame.exposure);
            self.program.u1f("u_haze", frame.haze);
            self.program.u1f("u_vhal_light", frame.vhal_light);
            self.program.u1f("u_sun_visibility", frame.sun_visibility);
            self.program
                .u1f("u_shadow_texel", 1.0 / self.shadow.size as f32);
            self.program.u1i("u_shadow", 0);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.shadow.texture));
            gl.draw_elements(glow::TRIANGLES, self.count, glow::UNSIGNED_INT, 0);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_vertex_array(None);
        }
    }
}
impl Drop for TerrainPass {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_buffer(self.vertices);
            self.gl.delete_buffer(self.indices);
            self.gl.delete_framebuffer(self.shadow.fbo);
            self.gl.delete_texture(self.shadow.texture);
        }
    }
}
