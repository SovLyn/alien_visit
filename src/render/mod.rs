//! Renderer orchestration. Director-owned glue; the heavy passes live in
//! `sky`, `terrain` and `orbit_lines`.

pub mod gl_util;
pub mod orbit_lines;
pub mod sky;
pub mod terrain;

use glow::HasContext;
use wasm_bindgen::JsCast;

use crate::frame::RenderFrame;
use crate::settings::Quality;

use gl_util::Gpu;
use orbit_lines::OrbitPass;
use sky::SkyPass;
use terrain::TerrainPass;

pub struct Renderer {
    gpu: Gpu,
    sky: SkyPass,
    terrain: TerrainPass,
    orbit: OrbitPass,
    canvas: web_sys::HtmlCanvasElement,
    pub width: i32,
    pub height: i32,
    pub gpu_info: String,
    quality: Quality,
    dpr_effective: f32,
}

impl Renderer {
    pub fn new(canvas: &web_sys::HtmlCanvasElement, quality: Quality) -> Result<Self, String> {
        let gpu = Gpu::new(canvas)?;
        let sky = SkyPass::new(&gpu)?;
        let terrain = TerrainPass::new(&gpu, quality)?;
        let orbit = OrbitPass::new(&gpu)?;
        let gpu_info = unsafe {
            let r = gpu.gl.get_parameter_string(glow::RENDERER);
            let v = gpu.gl.get_parameter_string(glow::VERSION);
            format!("{r} / {v}")
        };
        web_sys::console::log_1(&format!("[ossa] GL: {gpu_info}").into());
        let canvas: web_sys::HtmlCanvasElement = canvas
            .clone()
            .dyn_into()
            .map_err(|_| "canvas type".to_string())?;
        let mut renderer = Self {
            gpu,
            sky,
            terrain,
            orbit,
            canvas,
            width: 1,
            height: 1,
            gpu_info,
            quality,
            dpr_effective: 1.0,
        };
        renderer.resize(1.0, 1.0, 1.0, quality);
        Ok(renderer)
    }

    pub fn resize(&mut self, css_w: f64, css_h: f64, device_pixel_ratio: f64, quality: Quality) {
        let eff = (device_pixel_ratio as f32)
            .max(1.0)
            .min(quality.dpr_cap())
            * quality.render_scale();
        let w = ((css_w.max(1.0) * eff as f64).round() as i32).clamp(1, 8192);
        let h = ((css_h.max(1.0) * eff as f64).round() as i32).clamp(1, 8192);
        if w == self.width && h == self.height && quality == self.quality {
            return;
        }
        self.width = w;
        self.height = h;
        self.dpr_effective = eff;
        self.quality = quality;
        self.canvas.set_width(w as u32);
        self.canvas.set_height(h as u32);
        unsafe {
            self.gpu.gl.viewport(0, 0, w, h);
        }
        self.sky.set_quality(&self.gpu, quality);
        self.terrain
            .set_quality(&self.gpu, quality, w, h, eff);
        self.orbit.set_quality(&self.gpu, quality);
        web_sys::console::log_1(
            &format!("[ossa] viewport {w}x{h} (css {css_w:.0}x{css_h:.0}, dpr {eff:.2})").into(),
        );
    }

    pub fn render(&mut self, frame: &RenderFrame) {
        if frame.quality != self.quality {
            // A quality change without an explicit resize still needs targets
            // rebuilt; keep the current CSS size implied by the drawing buffer.
            let css_w = self.width as f64 / self.dpr_effective.max(0.1) as f64;
            let css_h = self.height as f64 / self.dpr_effective.max(0.1) as f64;
            let dpr = self.dpr_effective / self.quality.render_scale().max(0.1);
            self.resize(css_w, css_h, dpr as f64, frame.quality);
        }

        unsafe {
            self.gpu.gl.viewport(0, 0, self.width, self.height);
            self.gpu.gl.disable(glow::SCISSOR_TEST);
            self.gpu.gl.disable(glow::DEPTH_TEST);
            self.gpu.gl.depth_mask(true);
            self.gpu.gl.clear_color(0.006, 0.010, 0.014, 1.0);
            self.gpu.gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }

        // Sky fills the frame first, with no depth interaction. Terrain then
        // occludes it and writes depth for its own self-shadowing.
        self.sky.render(&self.gpu, frame);

        if frame.view == crate::settings::ViewMode::Orbit {
            self.orbit
                .render(&self.gpu, frame, self.width, self.height);
        }

        if frame.view == crate::settings::ViewMode::Surface {
            self.terrain
                .render(&self.gpu, frame, self.width, self.height);
        }

        unsafe {
            self.gpu.gl.bind_vertex_array(None);
            self.gpu.gl.use_program(None);
        }
    }

    pub fn drawing_size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}
