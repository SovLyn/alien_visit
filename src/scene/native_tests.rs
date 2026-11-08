//! Standalone native test root: excludes the WebGL-only renderer/main.
//! Run via src/scene/test_native.py; no additional dependencies or manifests.
#![allow(dead_code)]
#[path = "../frame.rs"]
mod frame;
#[path = "mod.rs"]
mod scene;
#[path = "../settings.rs"]
mod settings;
#[path = "../terrain_field.rs"]
mod terrain_field;
