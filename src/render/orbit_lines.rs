//! Camera-projected hairlines, with CPU sphere occlusion and a streaming VBO.
//! Lines deliberately do not use depth: the analytic sky writes no depth buffer.

use glam::{Vec2, Vec3};
use glow::HasContext;

use super::gl_util::{Gpu, Program};
use crate::frame::{BodyView, CameraView, RenderFrame};
use crate::settings::Quality;

const VERTEX: &str = r#"#version 300 es
precision highp float;
layout(location=0) in vec2 a_pixel;
layout(location=1) in vec4 a_color;
uniform vec2 u_viewport;
out vec4 v_color;
void main() {
    gl_Position = vec4(a_pixel/u_viewport*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.0,1.0);
    v_color = a_color;
}
"#;
const FRAGMENT: &str = r#"#version 300 es
precision highp float;
in vec4 v_color;
out vec4 frag_color;
void main() {
    // OrbitCurve colors are linear; blend display-referred, like the sky output.
    vec3 c = clamp(v_color.rgb,0.0,1.0);
    vec3 srgb = mix(c*12.92,1.055*pow(c,vec3(1.0/2.4))-0.055,step(vec3(0.0031308),c));
    frag_color = vec4(srgb,v_color.a);
}
"#;

pub struct OrbitPass {
    program: Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    vertices: Vec<[f32; 6]>,
}

impl OrbitPass {
    pub fn new(gpu: &Gpu) -> Result<Self, String> {
        let program = Program::new(gpu, VERTEX, FRAGMENT, "orbit hairlines")?;
        unsafe {
            let gl = &gpu.gl;
            let vao = gl
                .create_vertex_array()
                .map_err(|e| format!("orbit VAO: {e}"))?;
            let vbo = gl.create_buffer().map_err(|e| format!("orbit VBO: {e}"))?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 24, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 24, 8);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            Ok(Self {
                program,
                vao,
                vbo,
                vertices: Vec::new(),
            })
        }
    }

    pub fn set_quality(&mut self, _gpu: &Gpu, _quality: Quality) {}

    pub fn render(&mut self, gpu: &Gpu, frame: &RenderFrame, width: i32, height: i32) {
        if width <= 0 || height <= 0 || frame.orbits.is_empty() {
            return;
        }
        self.vertices.clear();
        let viewport = Vec2::new(width as f32, height as f32);
        let mut batches = Vec::with_capacity(frame.orbits.len());
        for curve in &frame.orbits {
            if curve.alpha <= 0.0 {
                continue;
            }
            let start = self.vertices.len() as i32;
            let color = curve.color.clamp(Vec3::ZERO, Vec3::ONE);
            let rgba = [color.x, color.y, color.z, curve.alpha.clamp(0.0, 0.40)];
            for points in curve.points_rel.windows(2) {
                append_segment(
                    &mut self.vertices,
                    &frame.camera,
                    &frame.bodies,
                    viewport,
                    points[0],
                    points[1],
                    rgba,
                );
            }
            let count = self.vertices.len() as i32 - start;
            if count > 0 {
                batches.push((start, count, curve.width_px));
            }
        }
        if self.vertices.is_empty() {
            return;
        }
        self.program.bind();
        self.program.u2f("u_viewport", viewport.x, viewport.y);
        unsafe {
            let gl = &gpu.gl;
            gl.disable(glow::DEPTH_TEST);
            gl.depth_mask(false);
            gl.enable(glow::BLEND);
            gl.blend_equation(glow::FUNC_ADD);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&self.vertices),
                glow::DYNAMIC_DRAW,
            );
            // Most WebGL implementations expose [1,1]. Respect the width hint
            // only within their advertised range, never causing a GL error.
            let mut widths = [1.0; 2];
            gl.get_parameter_f32_slice(glow::ALIASED_LINE_WIDTH_RANGE, &mut widths);
            for (start, count, hint) in batches {
                let hint = if hint.is_finite() { hint } else { 1.0 };
                gl.line_width(hint.clamp(widths[0], widths[1]));
                gl.draw_arrays(glow::LINES, start, count);
            }
            gl.line_width(1.0);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.disable(glow::BLEND);
            gl.depth_mask(true);
        }
    }
}

/// Camera-to-sample ray, evaluated in f64 to avoid subtracting squared system
/// distances in f32. Disabled bodies never occlude an orbit.
fn occluded(point: Vec3, bodies: &[BodyView]) -> bool {
    let point = point.as_dvec3();
    let distance = point.length();
    if !distance.is_finite() || distance <= 0.0 {
        return true;
    }
    let ray = point / distance;
    bodies
        .iter()
        .filter(|b| b.enabled && b.radius_km > 0.0)
        .any(|body| {
            let center = body.rel_pos_km.as_dvec3();
            let along = center.dot(ray);
            if along <= 0.0 {
                return false;
            }
            let perpendicular = center - ray * along;
            let disc = (body.radius_km as f64).powi(2) - perpendicular.length_squared();
            if disc < 0.0 {
                return false;
            }
            let near = along - disc.sqrt();
            near > 0.0 && near < distance - (distance * 1e-7).max(0.001)
        })
}

/// Two drawing-pixel spans keep body-limb clipping tight. The closest projected
/// point to each body is also checked, so even a subpixel moon cannot be missed
/// just because it falls between the endpoint/midpoint samples. Work is bounded.
fn append_segment(
    out: &mut Vec<[f32; 6]>,
    camera: &CameraView,
    bodies: &[BodyView],
    viewport: Vec2,
    a: Vec3,
    b: Vec3,
    color: [f32; 4],
) {
    if !a.is_finite() || !b.is_finite() {
        return;
    }
    let pa = camera.project(a, viewport);
    let pb = camera.project(b, viewport);
    if !pa.in_front || !pb.in_front {
        return;
    }
    let sa = Vec2::new(pa.x, pa.y);
    let sb = Vec2::new(pb.x, pb.y);
    // Trivial viewport rejection; GL clips the remaining crossing segments.
    if (sa.x < 0.0 && sb.x < 0.0)
        || (sa.x > viewport.x && sb.x > viewport.x)
        || (sa.y < 0.0 && sb.y < 0.0)
        || (sa.y > viewport.y && sb.y > viewport.y)
    {
        return;
    }
    let steps = ((sb - sa).length() / 2.0).ceil().clamp(1.0, 128.0) as usize;
    // Uniform screen-space subdivision needs perspective-correct world points.
    let sample = |t: f32| {
        let wa = (1.0 - t) / pa.depth_km;
        let wb = t / pb.depth_km;
        (a * wa + b * wb) / (wa + wb)
    };
    for step in 0..steps {
        let t0 = step as f32 / steps as f32;
        let t1 = (step + 1) as f32 / steps as f32;
        let s0 = sa.lerp(sb, t0);
        let s1 = sa.lerp(sb, t1);
        if occluded(sample(t0), bodies)
            || occluded(sample(t1), bodies)
            || occluded(sample((t0 + t1) * 0.5), bodies)
        {
            continue;
        }
        let delta = s1 - s0;
        let length2 = delta.length_squared().max(1e-12);
        let crosses_body = bodies.iter().filter(|b| b.enabled).any(|body| {
            let center = camera.project(body.rel_pos_km, viewport);
            if !center.in_front {
                return false;
            }
            let t = ((Vec2::new(center.x, center.y) - s0).dot(delta) / length2).clamp(0.0, 1.0);
            occluded(sample(t0 + (t1 - t0) * t), std::slice::from_ref(body))
        });
        if crosses_body {
            continue;
        }
        out.push([s0.x, s0.y, color[0], color[1], color[2], color[3]]);
        out.push([s1.x, s1.y, color[0], color[1], color[2], color[3]]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{BodyId, BodyKind};

    fn body() -> BodyView {
        BodyView {
            id: BodyId::Vhal,
            rel_pos_km: Vec3::new(0.0, 0.0, 10.0),
            radius_km: 1.0,
            kind: BodyKind::GasGiant,
            axis: Vec3::Y,
            spin_rad: 0.0,
            tint: Vec3::ONE,
            enabled: true,
        }
    }
    fn camera() -> CameraView {
        CameraView {
            forward: Vec3::Z,
            right: Vec3::X,
            up: Vec3::Y,
            fov_y_deg: 90.0,
            aspect: 1.0,
            surface: false,
        }
    }
    #[test]
    fn sphere_occludes_only_farther_samples() {
        let mut b = body();
        assert!(occluded(Vec3::new(0.0, 0.0, 20.0), &[b]));
        assert!(!occluded(Vec3::new(0.0, 0.0, 5.0), &[b]));
        assert!(!occluded(Vec3::new(4.0, 0.0, 20.0), &[b]));
        b.enabled = false;
        assert!(!occluded(Vec3::new(0.0, 0.0, 20.0), &[b]));
    }
    #[test]
    fn occulted_center_is_removed_but_side_arcs_remain() {
        let mut out = Vec::new();
        append_segment(
            &mut out,
            &camera(),
            &[body()],
            Vec2::splat(100.0),
            Vec3::new(-8.0, 0.0, 20.0),
            Vec3::new(8.0, 0.0, 20.0),
            [1.0; 4],
        );
        assert!(!out.is_empty());
        for pair in out.chunks_exact(2) {
            assert!(pair[1][0] <= 45.0 || pair[0][0] >= 55.0);
        }
    }
    #[test]
    fn behind_camera_segment_is_rejected() {
        let mut out = Vec::new();
        append_segment(
            &mut out,
            &camera(),
            &[],
            Vec2::splat(100.0),
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, 20.0),
            [1.0; 4],
        );
        assert!(out.is_empty());
    }
    #[test]
    fn projection_uses_drawing_pixels_and_preserves_style() {
        let mut out = Vec::new();
        let color = [0.3, 0.4, 0.5, 0.25];
        append_segment(
            &mut out,
            &camera(),
            &[],
            Vec2::splat(200.0),
            Vec3::new(-10.0, 0.0, 20.0),
            Vec3::new(10.0, 0.0, 20.0),
            color,
        );
        assert!((out.first().unwrap()[0] - 50.0).abs() < 0.001);
        assert!((out.last().unwrap()[0] - 150.0).abs() < 0.001);
        assert_eq!(&out[0][2..], &color);
    }
    #[test]
    fn subpixel_body_between_regular_samples_still_occludes() {
        let mut out = Vec::new();
        let mut b = body();
        b.rel_pos_km.x = 0.15; // x=50.75px, between regular x=50,51,52 samples
        b.radius_km = 0.01;
        append_segment(
            &mut out,
            &camera(),
            &[b],
            Vec2::splat(100.0),
            Vec3::new(0.0, 0.0, 20.0),
            Vec3::new(0.8, 0.0, 20.0),
            [1.0; 4],
        );
        assert!(out.is_empty());
    }
    #[test]
    fn small_distant_body_intersection_is_stable() {
        let mut b = body();
        b.rel_pos_km = Vec3::new(0.0, 0.0, 40_000_000.0);
        b.radius_km = 280.0;
        assert!(occluded(Vec3::new(0.0, 0.0, 50_000_000.0), &[b]));
        assert!(!occluded(Vec3::new(1000.0, 0.0, 50_000_000.0), &[b]));
    }
}
