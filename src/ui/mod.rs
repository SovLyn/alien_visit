//! DOM UI + HUD. OWNER: ui subagent. Bootstrap stub; replace wholesale.

use wasm_bindgen::JsValue;
use crate::frame::HudFrame;
use crate::settings::{Settings, UiEvent};

pub struct Ui;

impl Ui {
    pub fn new(_settings: &Settings) -> Result<Self, JsValue> {
        Ok(Self)
    }

    pub fn poll(&mut self) -> Vec<UiEvent> {
        Vec::new()
    }

    pub fn reflect(&mut self, settings: &Settings) {
        // Bootstrap: honor the hide-UI flag so QA screenshots work before the
        // real UI lands.
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(root) = doc.get_element_by_id("ui-root") {
                if settings.show_ui {
                    root.class_list().remove_1("is-hidden").ok();
                } else {
                    root.class_list().add_1("is-hidden").ok();
                }
            }
        }
    }

    pub fn update_hud(&mut self, _hud: &HudFrame) {}

    pub fn set_ready(&mut self) {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(loader) = doc.get_element_by_id("loader") {
                loader.class_list().add_1("is-hidden").ok();
            }
        }
    }

    pub fn notify_error(&self, msg: &str) {
        web_sys::console::error_1(&JsValue::from_str(msg));
    }
}
