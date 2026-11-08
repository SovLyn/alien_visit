//! Terrain + sun-shadow pass. OWNER: renderer-terrain subagent.
//! This file is a compile stub at project bootstrap; replace it wholesale.

use crate::frame::RenderFrame;
use crate::settings::Quality;
use super::gl_util::Gpu;

pub struct TerrainPass {
    quality: Quality,
}

impl TerrainPass {
    pub fn new(_gpu: &Gpu, quality: Quality) -> Result<Self, String> {
        Ok(Self { quality })
    }

    pub fn set_quality(
        &mut self,
        _gpu: &Gpu,
        quality: Quality,
        _width: i32,
        _height: i32,
        _render_scale: f32,
    ) {
        self.quality = quality;
    }

    pub fn render(&mut self, _gpu: &Gpu, _frame: &RenderFrame, _width: i32, _height: i32) {
        let _ = self.quality;
    }
}
