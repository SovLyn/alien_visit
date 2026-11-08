//! Sky / celestial impostor pass. OWNER: renderer-sky subagent.
//! This file is a compile stub at project bootstrap; replace it wholesale.

use crate::frame::RenderFrame;
use crate::settings::Quality;
use super::gl_util::Gpu;

pub struct SkyPass {
    #[allow(dead_code)]
    vao: Option<glow::VertexArray>,
}

impl SkyPass {
    pub fn new(gpu: &Gpu) -> Result<Self, String> {
        let vao = super::gl_util::fullscreen_vao(gpu)?;
        Ok(Self { vao: Some(vao) })
    }

    pub fn set_quality(&mut self, _gpu: &Gpu, _quality: Quality) {}

    pub fn render(&mut self, _gpu: &Gpu, _frame: &RenderFrame) {
        // Stub: the director clears the framebuffer in Renderer::render.
    }
}
