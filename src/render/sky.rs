//! Analytic, camera-relative celestial renderer. No textures or scene geometry.

use glow::HasContext;

use super::gl_util::{self, Gpu, Program};
use crate::frame::{BodyKind, MAX_BODIES, RenderFrame};
use crate::settings::{Quality, ViewMode};

const VERTEX: &str = r#"#version 300 es
precision highp float;
out vec2 v_ndc;
void main() {
    vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
    v_ndc = p * 2.0 - 1.0;
    gl_Position = vec4(v_ndc, 0.0, 1.0);
}
"#;

pub struct SkyPass {
    program: Program,
    vao: glow::VertexArray,
}

impl SkyPass {
    pub fn new(gpu: &Gpu) -> Result<Self, String> {
        let program = Program::new(gpu, VERTEX, include_str!("sky.glsl"), "celestial sky")?;
        let vao = gl_util::fullscreen_vao(gpu)?;
        Ok(Self { program, vao })
    }

    pub fn set_quality(&mut self, _gpu: &Gpu, _quality: Quality) {
        // Work limits are uploaded from the authoritative RenderFrame each draw.
    }

    pub fn render(&mut self, gpu: &Gpu, frame: &RenderFrame) {
        let mut spheres = [[0.0; 4]; MAX_BODIES];
        let mut axes = [[0.0; 4]; MAX_BODIES];
        let mut materials = [[0.0; 4]; MAX_BODIES];
        let mut kinds = [0.0; MAX_BODIES];
        for (i, b) in frame.bodies.iter().enumerate() {
            spheres[i] = [b.rel_pos_km.x, b.rel_pos_km.y, b.rel_pos_km.z, b.radius_km];
            axes[i] = [b.axis.x, b.axis.y, b.axis.z, b.spin_rad];
            materials[i] = [
                b.tint.x,
                b.tint.y,
                b.tint.z,
                if b.enabled { 1.0 } else { 0.0 },
            ];
            kinds[i] = match b.kind {
                BodyKind::Star => 0.0,
                BodyKind::GasGiant => 1.0,
                BodyKind::Desert => 2.0,
                BodyKind::Rock => 3.0,
                BodyKind::Ice => 4.0,
                BodyKind::Dark => 5.0,
            };
        }
        let p = &mut self.program;
        p.bind();
        p.vec4_array("u_spheres", &spheres);
        p.vec4_array("u_axes", &axes);
        p.vec4_array("u_materials", &materials);
        p.f32_array("u_kinds", &kinds);
        p.u3v("u_forward", frame.camera.forward);
        p.u3v("u_right", frame.camera.right);
        p.u3v("u_up", frame.camera.up);
        p.u2f("u_lens", frame.camera.tan_half_fov_y(), frame.camera.aspect);
        p.u3v("u_ring_center", frame.ring.center_rel_km);
        p.u3v("u_ring_normal", frame.ring.normal.normalize_or_zero());
        p.u4f(
            "u_ring",
            frame.ring.inner_km,
            frame.ring.outer_km,
            frame.ring.brightness,
            if frame.ring.enabled { 1.0 } else { 0.0 },
        );
        p.u1i("u_surface", i32::from(frame.view == ViewMode::Surface));
        p.u1i("u_octaves", frame.quality.sky_octaves());
        p.u1i("u_ring_steps", frame.quality.ring_steps());
        p.u1f("u_exposure", frame.exposure);
        p.u1f("u_haze", frame.haze);
        p.u1f("u_vhal_light", frame.vhal_light);
        unsafe {
            gpu.gl.disable(glow::DEPTH_TEST);
            gpu.gl.disable(glow::BLEND);
            gpu.gl.disable(glow::CULL_FACE);
            gpu.gl.depth_mask(false);
            gl_util::draw_fullscreen(gpu, self.vao);
            gpu.gl.depth_mask(true);
        }
    }
}
