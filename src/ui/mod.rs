//! Instrument panel and screen-space annotations.
use crate::frame::{BodyId, HudFrame};
use crate::settings::{Focus, Quality, Settings, UiEvent, ViewMode};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{
    Document, Element, Event, HtmlElement, HtmlInputElement, KeyboardEvent, PointerEvent,
};

pub struct Ui {
    elements: HashMap<String, Element>,
    queue: Rc<RefCell<Vec<UiEvent>>>,
    paused: Rc<Cell<bool>>,
}
fn text(el: Option<&Element>, value: &str) {
    if let Some(e) = el {
        if e.text_content().as_deref() != Some(value) {
            e.set_text_content(Some(value));
        }
    }
}
fn class(el: Option<&Element>, name: &str, on: bool) {
    if let Some(e) = el {
        let _ = e.class_list().toggle_with_force(name, on);
    }
}
fn hidden(el: Option<&Element>, value: bool) {
    if let Some(e) = el {
        if value {
            let _ = e.set_attribute("hidden", "");
        } else {
            let _ = e.remove_attribute("hidden");
        }
    }
}
fn pressed(el: Option<&Element>, value: bool) {
    if let Some(e) = el {
        let _ = e.set_attribute("aria-pressed", if value { "true" } else { "false" });
    }
}
fn listen(
    target: &web_sys::EventTarget,
    kind: &str,
    f: impl FnMut(Event) + 'static,
) -> Result<(), JsValue> {
    let cb = Closure::<dyn FnMut(Event)>::new(f);
    target.add_event_listener_with_callback(kind, cb.as_ref().unchecked_ref())?;
    cb.forget();
    Ok(())
}
fn blink(doc: &Document) {
    if let Some(e) = doc.get_element_by_id("veil") {
        let _ = e.class_list().remove_1("blink"); // Restart without forcing layout.
        let e2 = e.clone();
        let cb = Closure::once_into_js(move || {
            let _ = e2.class_list().add_1("blink");
        });
        if let Some(w) = web_sys::window() {
            let _ = w.request_animation_frame(cb.unchecked_ref());
        }
    }
}
fn slider(e: Option<&Element>, value: f32, min: f32, max: f32) {
    if let Some(e) = e {
        if let Some(input) = e.dyn_ref::<HtmlInputElement>() {
            let s = value.to_string();
            if input.value() != s {
                input.set_value(&s);
            }
            let _ = input.style().set_property(
                "--v",
                &format!(
                    "{}%",
                    ((value - min) / (max - min) * 100.0).clamp(0.0, 100.0)
                ),
            );
        }
    }
}
impl Ui {
    pub fn new(settings: &Settings) -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or("no window")?;
        let doc = window.document().ok_or("no document")?;
        let ids = [
            "ui-root",
            "loader",
            "fatal",
            "fatal-message",
            "deck",
            "deck-toggle",
            "deck-symbol",
            "status",
            "phase",
            "elevation",
            "tilt",
            "clock",
            "perf",
            "hint",
            "scale",
            "scale-label",
            "orbit-scale",
            "orbit-lens",
            "orbit-key",
            "lens-row",
            "haze-row",
            "haze-note",
            "focus-group",
            "pause",
            "scrub",
            "exposure",
            "haze",
            "rings",
            "light",
            "fov",
            "exposure-value",
            "haze-value",
            "rings-value",
            "light-value",
            "fov-value",
            "view-surface",
            "view-orbit",
            "speed-1",
            "speed-60",
            "speed-600",
            "speed-3600",
            "quality-low",
            "quality-med",
            "quality-high",
            "focus-ossa",
            "focus-vhal",
            "focus-kesh",
            "focus-system",
            "markers",
            "orbits",
            "reset",
            "fullscreen",
            "joystick",
            "joystick-dot",
            "marker-vhal",
            "marker-kesh",
            "marker-focus",
            "marker-ossa",
            "marker-thren",
            "marker-sable",
            "marker-nadir",
            "marker-cinder",
            "marker-sur",
            "arrow-vhal",
            "arrow-kesh",
            "arrow-focus",
        ];
        let mut elements: HashMap<String, Element> = ids
            .iter()
            .filter_map(|id| doc.get_element_by_id(id).map(|e| (id.to_string(), e)))
            .collect();
        for id in [
            "marker-vhal",
            "marker-kesh",
            "marker-focus",
            "marker-ossa",
            "marker-thren",
            "marker-sable",
            "marker-nadir",
            "marker-cinder",
            "marker-sur",
            "arrow-vhal",
            "arrow-kesh",
            "arrow-focus",
        ] {
            if let Some(span) = elements.get(id).and_then(|e| e.last_element_child()) {
                elements.insert(format!("{id}-label"), span);
            }
        }
        let queue = Rc::new(RefCell::new(Vec::new()));
        let paused = Rc::new(Cell::new(settings.paused));
        let buttons = [
            ("view-surface", UiEvent::SetView(ViewMode::Surface)),
            ("view-orbit", UiEvent::SetView(ViewMode::Orbit)),
            ("speed-1", UiEvent::SetTimeScale(1.0)),
            ("speed-60", UiEvent::SetTimeScale(60.0)),
            ("speed-600", UiEvent::SetTimeScale(600.0)),
            ("speed-3600", UiEvent::SetTimeScale(3600.0)),
            ("quality-low", UiEvent::SetQuality(Quality::Low)),
            ("quality-med", UiEvent::SetQuality(Quality::Medium)),
            ("quality-high", UiEvent::SetQuality(Quality::High)),
            ("focus-ossa", UiEvent::SetFocus(Focus::Ossa)),
            ("focus-vhal", UiEvent::SetFocus(Focus::Vhal)),
            ("focus-kesh", UiEvent::SetFocus(Focus::Kesh)),
            ("focus-system", UiEvent::SetFocus(Focus::System)),
            ("markers", UiEvent::ToggleMarkers),
            ("orbits", UiEvent::ToggleOrbits),
            ("reset", UiEvent::ResetCamera),
            ("fullscreen", UiEvent::Fullscreen),
        ];
        for (id, event) in buttons {
            if let Some(e) = elements.get(id) {
                let q = queue.clone();
                let d = doc.clone();
                listen(e, "click", move |_| {
                    if matches!(event, UiEvent::SetView(_)) {
                        blink(&d);
                    }
                    q.borrow_mut().push(event.clone());
                })?;
            }
        }
        if let Some(e) = elements.get("pause") {
            let q = queue.clone();
            let p = paused.clone();
            listen(e, "click", move |_| {
                let v = !p.get();
                p.set(v);
                q.borrow_mut().push(UiEvent::SetPaused(v));
            })?;
        }
        for id in ["exposure", "haze", "rings", "light", "fov", "scrub"] {
            if let Some(e) = elements.get(id) {
                let q = queue.clone();
                let input = e.clone().unchecked_into::<HtmlInputElement>();
                let key = id.to_string();
                listen(e, "input", move |_| {
                    let v = input.value_as_number() as f32;
                    if !v.is_finite() {
                        return;
                    }
                    let min = input.min().parse::<f32>().unwrap_or(0.0);
                    let max = input.max().parse::<f32>().unwrap_or(1.0);
                    let _ = input
                        .style()
                        .set_property("--v", &format!("{}%", (v - min) / (max - min) * 100.0));
                    let ev = match key.as_str() {
                        "exposure" => UiEvent::SetExposure(v),
                        "haze" => UiEvent::SetHaze(v),
                        "rings" => UiEvent::SetRings(v),
                        "light" => UiEvent::SetVhalLight(v),
                        "fov" => UiEvent::SetFov(v),
                        _ => UiEvent::ScrubTime(v),
                    };
                    q.borrow_mut().push(ev);
                })?;
            }
        }
        if let (Some(toggle), Some(deck)) = (elements.get("deck-toggle"), elements.get("deck")) {
            let collapsed = window
                .match_media("(max-width: 720px), (pointer: coarse)")?
                .map(|m| m.matches())
                .unwrap_or(false);
            class(Some(deck), "collapsed", collapsed);
            let _ = toggle.set_attribute("aria-expanded", if collapsed { "false" } else { "true" });
            text(
                elements.get("deck-symbol"),
                if collapsed { "+" } else { "−" },
            );
            let deck = deck.clone();
            let toggle = toggle.clone();
            let symbol = elements.get("deck-symbol").cloned();
            listen(&toggle.clone(), "click", move |_| {
                let c = !deck.class_list().contains("collapsed");
                class(Some(&deck), "collapsed", c);
                let _ = toggle.set_attribute("aria-expanded", if c { "false" } else { "true" });
                text(symbol.as_ref(), if c { "+" } else { "−" });
            })?;
        }
        {
            let q = queue.clone();
            let p = paused.clone();
            let d = doc.clone();
            listen(&window, "keydown", move |e| {
                let Some(k) = e.dyn_ref::<KeyboardEvent>() else {
                    return;
                };
                if k.repeat() || k.ctrl_key() || k.alt_key() || k.meta_key() {
                    return;
                }
                if let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
                    let tag = t.tag_name();
                    if tag == "INPUT"
                        || tag == "TEXTAREA"
                        || tag == "SELECT"
                        || t.has_attribute("contenteditable")
                    {
                        return;
                    }
                }
                let ev = match k.key().to_lowercase().as_str() {
                    "1" => Some(UiEvent::SetView(ViewMode::Surface)),
                    "2" => Some(UiEvent::SetView(ViewMode::Orbit)),
                    " " => {
                        let v = !p.get();
                        p.set(v);
                        Some(UiEvent::SetPaused(v))
                    }
                    "h" => Some(UiEvent::ToggleUi),
                    "m" => Some(UiEvent::ToggleMarkers),
                    "o" => Some(UiEvent::ToggleOrbits),
                    "r" => Some(UiEvent::ResetCamera),
                    "f" => Some(UiEvent::Fullscreen),
                    _ => None,
                };
                if let Some(ev) = ev {
                    e.prevent_default();
                    if matches!(ev, UiEvent::SetView(_)) {
                        blink(&d)
                    }
                    q.borrow_mut().push(ev);
                }
            })?;
        }
        if let (Some(joy), Some(dot)) = (elements.get("joystick"), elements.get("joystick-dot")) {
            let active = Rc::new(Cell::new(-1));
            for kind in [
                "pointerdown",
                "pointermove",
                "pointerup",
                "pointercancel",
                "lostpointercapture",
            ] {
                let q = queue.clone();
                let a = active.clone();
                let j = joy.clone();
                let dot = dot.clone();
                let kind = kind.to_string();
                listen(joy, &kind.clone(), move |e| {
                    let Some(p) = e.dyn_ref::<PointerEvent>() else {
                        return;
                    };
                    e.stop_propagation();
                    e.prevent_default();
                    if kind == "pointerdown" {
                        if a.get() != -1 {
                            return;
                        }
                        a.set(p.pointer_id());
                        let _ = j.set_pointer_capture(p.pointer_id());
                    }
                    if a.get() != p.pointer_id() {
                        return;
                    }
                    if kind == "pointerup"
                        || kind == "pointercancel"
                        || kind == "lostpointercapture"
                    {
                        a.set(-1);
                        q.borrow_mut().push(UiEvent::SetMoveAxis(0.0, 0.0));
                        if let Some(d) = dot.dyn_ref::<HtmlElement>() {
                            let _ = d.style().set_property("transform", "translate(0px,0px)");
                        }
                        return;
                    }
                    let r = j.get_bounding_client_rect();
                    let mut x = ((p.client_x() as f64 - r.left() - r.width() / 2.0) / 48.0) as f32;
                    let mut y = ((p.client_y() as f64 - r.top() - r.height() / 2.0) / 48.0) as f32;
                    let len = (x * x + y * y).sqrt();
                    if len > 1.0 {
                        x /= len;
                        y /= len
                    }
                    if let Some(d) = dot.dyn_ref::<HtmlElement>() {
                        let _ = d.style().set_property(
                            "transform",
                            &format!("translate({}px,{}px)", x * 40.0, y * 40.0),
                        );
                    }
                    let mag = (x * x + y * y).sqrt();
                    if mag < 0.12 {
                        x = 0.0;
                        y = 0.0
                    } else {
                        let k = ((mag - 0.12) / 0.88) / mag;
                        x *= k;
                        y *= k
                    }
                    q.borrow_mut().push(UiEvent::SetMoveAxis(x, -y));
                })?;
            }
        }
        let mut ui = Self {
            elements,
            queue,
            paused,
        };
        ui.reflect(settings);
        Ok(ui)
    }
    pub fn poll(&mut self) -> Vec<UiEvent> {
        std::mem::take(&mut *self.queue.borrow_mut())
    }
    pub fn reflect(&mut self, s: &Settings) {
        let e = &self.elements;
        self.paused.set(s.paused);
        class(e.get("ui-root"), "is-hidden", !s.show_ui);
        class(e.get("ui-root"), "surface", s.view == ViewMode::Surface);
        pressed(e.get("view-surface"), s.view == ViewMode::Surface);
        pressed(e.get("view-orbit"), s.view == ViewMode::Orbit);
        for (id, v) in [
            ("speed-1", 1.0),
            ("speed-60", 60.0),
            ("speed-600", 600.0),
            ("speed-3600", 3600.0),
        ] {
            pressed(e.get(id), (s.time_scale - v).abs() < 0.01)
        }
        for (id, v) in [
            ("quality-low", Quality::Low),
            ("quality-med", Quality::Medium),
            ("quality-high", Quality::High),
        ] {
            pressed(e.get(id), s.quality == v)
        }
        for (id, v) in [
            ("focus-ossa", Focus::Ossa),
            ("focus-vhal", Focus::Vhal),
            ("focus-kesh", Focus::Kesh),
            ("focus-system", Focus::System),
        ] {
            pressed(e.get(id), s.focus == v)
        }
        pressed(e.get("markers"), s.show_markers);
        pressed(e.get("orbits"), s.show_orbits);
        pressed(e.get("pause"), s.paused);
        text(e.get("pause"), if s.paused { "▶" } else { "Ⅱ" });
        if let Some(p) = e.get("pause") {
            let _ = p.set_attribute(
                "aria-label",
                if s.paused {
                    "Play simulation"
                } else {
                    "Pause simulation"
                },
            );
        }
        let orbit = s.view == ViewMode::Orbit;
        hidden(e.get("focus-group"), !orbit);
        hidden(e.get("orbit-key"), !orbit);
        hidden(e.get("lens-row"), orbit);
        hidden(e.get("orbit-lens"), !orbit);
        text(
            e.get("haze-note"),
            if orbit { " / SURFACE ONLY" } else { "" },
        );
        if let Some(h) = e.get("haze").and_then(|e| e.dyn_ref::<HtmlInputElement>()) {
            h.set_disabled(orbit)
        }
        text(
            e.get("hint"),
            if orbit {
                "DRAG ORBIT · SCROLL ZOOM"
            } else {
                "WASD MOVE · DRAG LOOK · SHIFT RUN"
            },
        );
        for (id, val, min, max) in [
            ("exposure", s.exposure, 0.6, 1.6),
            ("haze", s.haze, 0.0, 1.5),
            ("rings", s.ring_brightness, 0.0, 1.5),
            ("light", s.vhal_light, 0.0, 1.5),
            ("fov", s.fov_deg, 55.0, 95.0),
            (
                "scrub",
                (s.sim_time.rem_euclid(20.43 * 3600.0) / (20.43 * 3600.0)) as f32,
                0.0,
                1.0,
            ),
        ] {
            slider(e.get(id), val, min, max);
            if id != "scrub" {
                text(
                    e.get(&format!("{id}-value")),
                    &if id == "fov" {
                        format!("{val:.0}°")
                    } else {
                        format!("{val:.2}")
                    },
                );
            }
        }
    }
    pub fn update_hud(&mut self, h: &HudFrame) {
        let e = &self.elements;
        let t = &h.telemetry;
        text(
            e.get("status"),
            &format!("{} · {}", t.mode_line, t.location_line),
        );
        for (id, v) in [
            ("phase", &t.vhal_phase),
            ("elevation", &t.kesh_elevation),
            ("tilt", &t.ring_tilt),
            ("clock", &t.clock),
            ("perf", &t.perf),
        ] {
            text(e.get(id), v)
        }
        hidden(
            e.get("scale"),
            t.view != ViewMode::Orbit || t.scale.is_none(),
        );
        text(e.get("scale-label"), t.scale.as_deref().unwrap_or(""));
        text(e.get("orbit-scale"), t.scale.as_deref().unwrap_or("—"));
        for kind in ["marker", "arrow"] {
            for body in [
                "vhal", "kesh", "focus", "ossa", "thren", "sable", "nadir", "cinder", "sur",
            ] {
                hidden(e.get(&format!("{kind}-{body}")), true)
            }
        }
        if !t.show_markers {
            return;
        }
        let width = web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(1440.0) as f32;
        for m in &h.markers {
            let id = match m.body {
                BodyId::Vhal => "marker-vhal",
                BodyId::Kesh => "marker-kesh",
                BodyId::Ossa => "marker-ossa",
                BodyId::Thren => "marker-thren",
                BodyId::Sable => "marker-sable",
                BodyId::Nadir => "marker-nadir",
                BodyId::Cinder => "marker-cinder",
                BodyId::Sur => "marker-sur",
            };
            if let Some(el) = e.get(id) {
                hidden(Some(el), false);
                class(Some(el), "warning", m.state.is_warning());
                // Keep labels inside the frame: left of the reticle except for
                // markers very close to the left edge, where they flip right.
                class(Some(el), "leftward", m.x > 260.0);
                class(
                    Some(el),
                    "minor",
                    !matches!(m.body, BodyId::Vhal | BodyId::Kesh),
                );
                if let Some(html) = el.dyn_ref::<HtmlElement>() {
                    let _ = html
                        .style()
                        .set_property("transform", &format!("translate({}px,{}px)", m.x, m.y));
                }
                text(
                    e.get(&format!("{id}-label")),
                    &format!("{} · {}", m.label, m.detail),
                );
            }
        }
        for a in &h.arrows {
            let id = match a.body {
                BodyId::Vhal => "arrow-vhal",
                BodyId::Kesh => "arrow-kesh",
                _ => "arrow-focus",
            };
            if let Some(el) = e.get(id) {
                hidden(Some(el), false);
                class(Some(el), "warning", a.state.is_warning());
                class(Some(el), "leftward", a.x > width / 2.0);
                if let Some(html) = el.dyn_ref::<HtmlElement>() {
                    let _ = html
                        .style()
                        .set_property("transform", &format!("translate({}px,{}px)", a.x, a.y));
                    let _ = html
                        .style()
                        .set_property("--angle", &format!("{}deg", a.angle_deg + 45.0));
                }
                text(
                    e.get(&format!("{id}-label")),
                    &format!("{} · {}", a.label, a.detail),
                );
            }
        }
    }
    pub fn set_ready(&mut self) {
        class(self.elements.get("loader"), "is-hidden", true)
    }
    pub fn notify_error(&self, msg: &str) {
        web_sys::console::error_1(&JsValue::from_str(msg));
        text(
            self.elements.get("fatal-message"),
            "Please reload to reconnect.",
        );
        hidden(self.elements.get("fatal"), false);
        class(self.elements.get("fatal"), "is-visible", true);
        class(self.elements.get("loader"), "is-hidden", true)
    }
}
