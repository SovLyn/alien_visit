//! OSSA — application shell and main loop.
//! Director-owned glue: module wiring, settings, URL/debug hooks, RAF clock.

mod frame;
mod render;
mod scene;
mod settings;
mod terrain_field;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::HtmlCanvasElement;

use render::Renderer;
use scene::Scene;
use settings::{Focus, Quality, Settings, UiEvent, ViewMode};
use ui::Ui;

struct App {
    settings: Settings,
    scene: Scene,
    renderer: Renderer,
    ui: Ui,
    canvas: HtmlCanvasElement,
    viewport: (f64, f64),
    dpr: f64,
    last_ts: f64,
    have_ts: bool,
    fps: f32,
    frame_count: u32,
    booted_at: f64,
    last_quality: Quality,
}

impl App {
    fn tick(&mut self, ts_ms: f64) {
        let dt = if self.have_ts {
            ((ts_ms - self.last_ts) / 1000.0).clamp(0.0, 0.1)
        } else {
            0.016
        };
        self.have_ts = true;
        self.last_ts = ts_ms;
        self.frame_count = self.frame_count.saturating_add(1);

        if dt > 0.0 {
            let inst = (1.0 / dt) as f32;
            self.fps = if self.frame_count < 20 {
                inst
            } else {
                self.fps * 0.92 + inst * 0.08
            };
        }

        // Viewport / DPR sync.
        let css_w = self.canvas.client_width().max(1) as f64;
        let css_h = self.canvas.client_height().max(1) as f64;
        let dpr = web_sys::window()
            .map(|w| w.device_pixel_ratio())
            .unwrap_or(1.0);
        if (css_w - self.viewport.0).abs() > 0.5
            || (css_h - self.viewport.1).abs() > 0.5
            || (dpr - self.dpr).abs() > 0.01
        {
            self.viewport = (css_w, css_h);
            self.dpr = dpr;
            self.scene.set_viewport(css_w, css_h);
            self.renderer
                .resize(css_w, css_h, dpr, self.settings.quality);
        }

        // Drain UI events.
        let events = self.ui.poll();
        for ev in events {
            if ev == UiEvent::Fullscreen {
                toggle_fullscreen();
                continue;
            }
            if let UiEvent::SetQuality(q) = ev {
                self.settings.quality = q;
            }
            self.scene.apply_event(ev, &mut self.settings);
        }
        if self.settings.quality != self.last_quality {
            self.last_quality = self.settings.quality;
            self.renderer.resize(
                self.viewport.0,
                self.viewport.1,
                self.dpr,
                self.settings.quality,
            );
        }

        self.scene.update(dt, &mut self.settings);

        let frame = self.scene.build_frame(&self.settings);
        self.renderer.render(&frame);

        let hud = self
            .scene
            .build_hud(&self.settings, (css_w as f32, css_h as f32), self.fps);
        self.ui.update_hud(&hud);
        self.ui.reflect(&self.settings);

        if self.frame_count == 2 {
            self.ui.set_ready();
        }
        let _ = self.booted_at;
    }
}

fn main() {
    console_error_panic_hook::set_once();
    if let Err(e) = boot() {
        web_sys::console::error_1(&format!("[ossa] boot failed: {e}").into());
        show_fatal(&e);
    }
}

fn boot() -> Result<(), String> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let canvas = document
        .get_element_by_id("scene")
        .ok_or("missing #scene canvas")?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| "#scene is not a canvas")?;

    let mut settings = Settings::default();
    settings.reduced_motion = Settings::prefers_reduced_motion();
    let initial_quality = Settings::auto_quality();
    settings.quality = initial_quality;

    let params = UrlParams::new();
    params.apply(&mut settings);

    let css_w = canvas.client_width().max(1) as f64;
    let css_h = canvas.client_height().max(1) as f64;
    if css_w / css_h < 0.85 && params.get("fov").is_none() {
        settings.fov_deg = 74.0;
    }

    let renderer = Renderer::new(&canvas, settings.quality)?;
    let mut scene =
        Scene::new(&canvas, &settings, (css_w, css_h)).map_err(|e| format!("scene init: {e:?}"))?;
    let ui = Ui::new(&settings).map_err(|e| format!("UI init: {e:?}"))?;

    params.apply_camera(&mut scene, &mut settings);

    let mut app = App {
        settings,
        scene,
        renderer,
        ui,
        canvas,
        viewport: (css_w, css_h),
        dpr: window.device_pixel_ratio(),
        last_ts: 0.0,
        have_ts: false,
        fps: 60.0,
        frame_count: 0,
        booted_at: window.performance().map(|p| p.now()).unwrap_or(0.0),
        last_quality: initial_quality,
    };
    app.renderer
        .resize(css_w, css_h, app.dpr, app.settings.quality);
    app.scene.set_viewport(css_w, css_h);
    app.ui.reflect(&app.settings);

    let app = Rc::new(RefCell::new(app));
    install_debug_api(&app);

    let cb: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let cb_next = cb.clone();
    let app_next = app.clone();
    *cb.borrow_mut() = Some(Closure::new(move |ts: f64| {
        if let Ok(mut a) = app_next.try_borrow_mut() {
            a.tick(ts);
        }
        if let Some(w) = web_sys::window() {
            if let Some(f) = cb_next.borrow().as_ref() {
                let _ = w.request_animation_frame(f.as_ref().unchecked_ref());
            }
        }
    }));
    window
        .request_animation_frame(cb.borrow().as_ref().unwrap().as_ref().unchecked_ref())
        .map_err(|e| format!("requestAnimationFrame: {e:?}"))?;

    // Keep the closure alive for the page lifetime.
    std::mem::forget(cb);
    Ok(())
}

fn toggle_fullscreen() {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if doc.fullscreen_element().is_some() {
        let _ = doc.exit_fullscreen();
    } else if let Some(el) = doc.document_element() {
        let _ = el.request_fullscreen();
        let _ = el.request_pointer_lock();
    }
}

fn show_fatal(msg: &str) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if let Some(el) = doc.get_element_by_id("fatal") {
        el.set_attribute("hidden", "false").ok();
        el.set_class_name("fatal is-visible");
        if let Some(t) = doc.get_element_by_id("fatal-message") {
            t.set_text_content(Some(msg));
        }
    }
    if let Some(loader) = doc.get_element_by_id("loader") {
        loader.set_class_name("loader is-hidden");
    }
}

// ---------------------------------------------------------------------------
// URL parameters (deterministic QA / sharing)
// ---------------------------------------------------------------------------

struct UrlParams {
    map: Vec<(String, String)>,
}

impl UrlParams {
    fn new() -> Self {
        let search = web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default();
        let mut map = Vec::new();
        // `UrlSearchParams::entries` is not always bound; iterate via JS forEach.
        if let Ok(p) = web_sys::UrlSearchParams::new_with_str(&search) {
            let keys = js_sys::Array::new();
            let keys_cb = keys.clone();
            let cb = Closure::<dyn Fn(JsValue, JsValue, JsValue)>::new(
                move |_v: JsValue, k: JsValue, _this: JsValue| {
                    keys_cb.push(&k);
                },
            );
            if let Ok(f) = js_sys::Reflect::get(&p, &"forEach".into()) {
                if let Ok(f) = f.dyn_into::<js_sys::Function>() {
                    let _ = f.call3(&p, cb.as_ref(), &JsValue::UNDEFINED, &p);
                }
            }
            for i in 0..keys.length() {
                if let Some(k) = keys.get(i).as_string() {
                    if let Some(v) = p.get(k.as_str()) {
                        map.push((k, v));
                    }
                }
            }
        }
        Self { map }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.map
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    fn f32_of(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(|v| v.parse().ok())
    }

    fn f64_of(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.parse().ok())
    }

    fn bool_of(&self, key: &str) -> Option<bool> {
        self.get(key)
            .map(|v| !matches!(v, "0" | "false" | "off" | "no"))
    }

    fn apply(&self, s: &mut Settings) {
        if let Some(v) = self.get("view") {
            s.view = if v.starts_with("or") {
                ViewMode::Orbit
            } else {
                ViewMode::Surface
            };
        }
        if let Some(v) = self.get("quality") {
            s.quality = match v {
                "low" => Quality::Low,
                "med" | "medium" => Quality::Medium,
                "high" => Quality::High,
                _ => s.quality,
            };
        }
        if let Some(v) = self.get("focus") {
            s.focus = match v {
                "ossa" => Focus::Ossa,
                "kesh" => Focus::Kesh,
                "system" => Focus::System,
                _ => Focus::Vhal,
            };
        }
        if let Some(v) = self.f64_of("t") {
            s.sim_time = v;
        }
        if let Some(v) = self.f64_of("time_scale") {
            s.time_scale = v;
        }
        if let Some(v) = self.bool_of("paused") {
            s.paused = v;
        }
        if let Some(v) = self.f32_of("exposure") {
            s.exposure = v.clamp(0.4, 2.0);
        }
        if let Some(v) = self.f32_of("haze") {
            s.haze = v.clamp(0.0, 1.5);
        }
        if let Some(v) = self.f32_of("rings") {
            s.ring_brightness = v.clamp(0.0, 1.5);
        }
        if let Some(v) = self.f32_of("vhal_light") {
            s.vhal_light = v.clamp(0.0, 1.5);
        }
        if let Some(v) = self.f32_of("fov") {
            s.fov_deg = v.clamp(40.0, 110.0);
        }
        if let Some(v) = self.bool_of("markers") {
            s.show_markers = v;
        }
        if let Some(v) = self.bool_of("orbits") {
            s.show_orbits = v;
        }
        if let Some(v) = self.bool_of("ui") {
            s.show_ui = v;
        }
    }

    fn apply_camera(&self, scene: &mut Scene, s: &mut Settings) {
        scene.set_view_instant(s.view);
        if self.get("t").is_some() {
            scene.set_sim_time(s.sim_time, s);
        }
        scene.set_focus_instant(s.focus);
        if let Some(yaw) = self.f32_of("yaw") {
            let pitch = self.f32_of("pitch").unwrap_or(20.0);
            scene.set_surface_look(yaw, pitch);
        } else if let Some(pitch) = self.f32_of("pitch") {
            scene.set_surface_look(18.0, pitch);
        }
        if let Some(az) = self.f32_of("orbit_az") {
            let el = self.f32_of("orbit_el").unwrap_or(28.0);
            let d = self.f64_of("distance").unwrap_or(850_000.0);
            scene.set_orbit_camera(az, el, d);
        } else if self.f32_of("distance").is_some() {
            scene.set_orbit_camera(28.0, 28.0, self.f64_of("distance").unwrap_or(850_000.0));
        }
    }
}

// ---------------------------------------------------------------------------
// In-page debug API, used by tools/qa. Stable enough for screenshots/bench.
// ---------------------------------------------------------------------------

macro_rules! install_fn {
    ($api:expr, $name:expr, $closure:expr) => {{
        let f: js_sys::Function = $closure
            .as_ref()
            .unchecked_ref::<js_sys::Function>()
            .clone();
        let _ = js_sys::Reflect::set(&$api, &JsValue::from_str($name), &f);
        $closure.forget();
    }};
}

fn install_debug_api(app: &Rc<RefCell<App>>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let api = js_sys::Object::new();

    let a = app.clone();
    let snapshot = Closure::<dyn Fn() -> JsValue>::new(move || {
        let Ok(app) = a.try_borrow() else {
            return JsValue::from_str("{}");
        };
        let s = &app.settings;
        let json = format!(
            "{{\"view\":\"{}\",\"focus\":\"{}\",\"time\":{:.3},\"paused\":{},\"timeScale\":{:.1},\"quality\":\"{}\",\"fps\":{:.1},\"width\":{},\"height\":{},\"gpu\":\"{}\",\"ui\":{},\"markers\":{},\"orbits\":{},\"exposure\":{:.3},\"haze\":{:.3},\"rings\":{:.3},\"vhalLight\":{:.3},\"fov\":{:.2}}}",
            s.view.label(),
            s.focus.label(),
            s.sim_time,
            s.paused,
            s.time_scale,
            s.quality.label(),
            app.fps,
            app.renderer.width,
            app.renderer.height,
            app.renderer.gpu_info.replace('"', "'"),
            s.show_ui,
            s.show_markers,
            s.show_orbits,
            s.exposure,
            s.haze,
            s.ring_brightness,
            s.vhal_light,
            s.fov_deg
        );
        JsValue::from_str(&json)
    });
    install_fn!(api, "snapshot", snapshot);

    let a = app.clone();
    let set = Closure::<dyn Fn(JsValue)>::new(move |obj: JsValue| {
        let Ok(mut app) = a.try_borrow_mut() else {
            return;
        };
        debug_apply(&mut app, &obj);
    });
    install_fn!(api, "set", set);

    let a = app.clone();
    let set_time = Closure::<dyn Fn(f64)>::new(move |t: f64| {
        if let Ok(mut app) = a.try_borrow_mut() {
            let mut s = app.settings;
            app.scene.set_sim_time(t, &mut s);
            app.settings = s;
        }
    });
    install_fn!(api, "setTime", set_time);

    let a = app.clone();
    let set_camera = Closure::<dyn Fn(JsValue)>::new(move |obj: JsValue| {
        if let Ok(mut app) = a.try_borrow_mut() {
            if let Some(yaw) = js_f32(&obj, "yaw") {
                let pitch = js_f32(&obj, "pitch").unwrap_or(20.0);
                app.scene.set_surface_look(yaw, pitch);
            }
            if let Some(az) = js_f32(&obj, "orbitAz") {
                let el = js_f32(&obj, "orbitEl").unwrap_or(28.0);
                let d = js_f64(&obj, "distance").unwrap_or(850_000.0);
                app.scene.set_orbit_camera(az, el, d);
            }
        }
    });
    install_fn!(api, "setCamera", set_camera);

    let a = app.clone();
    let fps = Closure::<dyn Fn() -> f64>::new(move || {
        a.try_borrow().map(|a| a.fps as f64).unwrap_or(0.0)
    });
    install_fn!(api, "fps", fps);

    let a = app.clone();
    let state = Closure::<dyn Fn() -> JsValue>::new(move || {
        let Ok(app) = a.try_borrow() else {
            return JsValue::from_str("{}");
        };
        JsValue::from_str(&format!(
            "{{\"view\":\"{}\",\"focus\":\"{}\",\"time\":{:.3},\"quality\":\"{}\"}}",
            app.settings.view.label(),
            app.settings.focus.label(),
            app.settings.sim_time,
            app.settings.quality.label()
        ))
    });
    install_fn!(api, "state", state);

    let a = app.clone();
    let hide = Closure::<dyn Fn(bool)>::new(move |v: bool| {
        if let Ok(mut app) = a.try_borrow_mut() {
            app.settings.show_ui = !v;
            let s = app.settings;
            app.ui.reflect(&s);
        }
    });
    install_fn!(api, "hideUI", hide);

    let ready = Closure::<dyn Fn() -> bool>::new(move || true);
    install_fn!(api, "ready", ready);

    let _ = js_sys::Reflect::set(&window, &"__ossa".into(), &api);
}

fn debug_apply(app: &mut App, obj: &JsValue) {
    let mut s = app.settings;
    if let Some(v) = js_str(obj, "view") {
        let view = if v.starts_with("or") {
            ViewMode::Orbit
        } else {
            ViewMode::Surface
        };
        app.scene.set_view_instant(view);
        s.view = view;
    }
    if let Some(v) = js_str(obj, "focus") {
        s.focus = match v.as_str() {
            "ossa" => Focus::Ossa,
            "kesh" => Focus::Kesh,
            "system" => Focus::System,
            _ => Focus::Vhal,
        };
        app.scene.set_focus_instant(s.focus);
    }
    if let Some(v) = js_f64(obj, "time") {
        app.scene.set_sim_time(v, &mut s);
    }
    if let Some(v) = js_f32(obj, "timeScale") {
        s.time_scale = v as f64;
    }
    if let Some(v) = js_f32(obj, "exposure") {
        s.exposure = v;
    }
    if let Some(v) = js_f32(obj, "haze") {
        s.haze = v;
    }
    if let Some(v) = js_f32(obj, "rings") {
        s.ring_brightness = v;
    }
    if let Some(v) = js_f32(obj, "vhalLight") {
        s.vhal_light = v;
    }
    if let Some(v) = js_f32(obj, "fov") {
        s.fov_deg = v;
    }
    if let Some(v) = js_str(obj, "quality") {
        s.quality = match v.as_str() {
            "low" => Quality::Low,
            "med" | "medium" => Quality::Medium,
            _ => Quality::High,
        };
    }
    if let Some(v) = js_bool(obj, "markers") {
        s.show_markers = v;
    }
    if let Some(v) = js_bool(obj, "orbits") {
        s.show_orbits = v;
    }
    if let Some(v) = js_bool(obj, "ui") {
        s.show_ui = v;
    }
    if let Some(v) = js_bool(obj, "paused") {
        s.paused = v;
    }
    app.settings = s;
    let s = app.settings;
    app.ui.reflect(&s);
}

fn js_get(obj: &JsValue, key: &str) -> JsValue {
    js_sys::Reflect::get(obj, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
}

fn js_str(obj: &JsValue, key: &str) -> Option<String> {
    js_get(obj, key).as_string()
}

fn js_bool(obj: &JsValue, key: &str) -> Option<bool> {
    let v = js_get(obj, key);
    if v.is_undefined() || v.is_null() {
        None
    } else {
        v.as_bool()
    }
}

fn js_f32(obj: &JsValue, key: &str) -> Option<f32> {
    js_get(obj, key).as_f64().map(|v| v as f32)
}

fn js_f64(obj: &JsValue, key: &str) -> Option<f64> {
    js_get(obj, key).as_f64()
}
