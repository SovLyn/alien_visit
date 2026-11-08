//! Orbit-line pass. OWNER: renderer-sky subagent.
//! This file is a compile stub at project bootstrap; replace it wholesale.

use crate::frame::RenderFrame;
use crate::settings::Quality;
use super::gl_util::Gpu;

pub struct OrbitPass;

impl OrbitPass {
    pub fn new(_gpu: &Gpu) -> Result<Self, String> {
        Ok(Self)
    }

    pub fn set_quality(&mut self, _gpu: &Gpu, _quality: Quality) {}

    pub fn render(&mut self, _gpu: &Gpu, _frame: &RenderFrame, _width: i32, _height: i32) {}
}
