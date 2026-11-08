//! Scene simulation + camera + HUD data. OWNER: scene subagent.
//! This bootstrap stub will be replaced; it exists so the renderer/UI can be
//! developed in parallel against the real frame types.

use glam::{DVec3, Vec3};
use wasm_bindgen::JsValue;

use crate::frame::*;
use crate::settings::*;
use crate::terrain_field;

pub struct Scene {
    viewport: (f64, f64),
    surface_pos: [f64; 2],
    eye_height_m: f32,
    yaw_deg: f32,
    pitch_deg: f32,
    orbit_az_deg: f32,
    orbit_el_deg: f32,
    orbit_distance_km: f64,
}

impl Scene {
    pub fn new(
        _canvas: &web_sys::HtmlCanvasElement,
        settings: &Settings,
        viewport: (f64, f64),
    ) -> Result<Self, JsValue> {
        let h = terrain_field::height_m(0.0, 0.0);
        Ok(Self {
            viewport,
            surface_pos: [0.0, 0.0],
            eye_height_m: h + 1.7,
            yaw_deg: 18.0,
            pitch_deg: 20.0,
            orbit_az_deg: 28.0,
            orbit_el_deg: 28.0,
            orbit_distance_km: 850_000.0,
        })
        .map(|s| {
            let _ = settings;
            s
        })
    }

    pub fn set_viewport(&mut self, w: f64, h: f64) {
        self.viewport = (w.max(1.0), h.max(1.0));
    }

    pub fn update(&mut self, dt: f64, settings: &mut Settings) {
        if !settings.paused {
            settings.sim_time += dt * settings.time_scale;
        }
    }

    pub fn apply_event(&mut self, event: UiEvent, settings: &mut Settings) {
        match event {
            UiEvent::SetView(v) => settings.view = v,
            UiEvent::SetFocus(f) => settings.focus = f,
            UiEvent::SetQuality(q) => settings.quality = q,
            UiEvent::SetPaused(p) => settings.paused = p,
            UiEvent::SetTimeScale(s) => settings.time_scale = s,
            UiEvent::ScrubTime(_) => {}
            UiEvent::SetExposure(v) => settings.exposure = v,
            UiEvent::SetHaze(v) => settings.haze = v,
            UiEvent::SetRings(v) => settings.ring_brightness = v,
            UiEvent::SetVhalLight(v) => settings.vhal_light = v,
            UiEvent::SetFov(v) => settings.fov_deg = v,
            UiEvent::ToggleMarkers => settings.show_markers = !settings.show_markers,
            UiEvent::ToggleOrbits => settings.show_orbits = !settings.show_orbits,
            UiEvent::ToggleUi => settings.show_ui = !settings.show_ui,
            UiEvent::ResetCamera => self.reset_camera(),
            UiEvent::Fullscreen => {}
            UiEvent::SetMoveAxis(_, _) => {}
        }
    }

    pub fn set_view_instant(&mut self, view: ViewMode) {
        let _ = view;
    }

    pub fn set_sim_time(&mut self, t: f64, settings: &mut Settings) {
        settings.sim_time = t;
    }

    pub fn scrub(&mut self, fraction: f32, settings: &mut Settings) {
        settings.sim_time = fraction as f64 * 73_554.0;
    }

    pub fn set_surface_look(&mut self, yaw_deg: f32, pitch_deg: f32) {
        self.yaw_deg = yaw_deg;
        self.pitch_deg = pitch_deg.clamp(-85.0, 85.0);
    }

    pub fn set_orbit_camera(&mut self, az_deg: f32, el_deg: f32, distance_km: f64) {
        self.orbit_az_deg = az_deg;
        self.orbit_el_deg = el_deg.clamp(-85.0, 85.0);
        self.orbit_distance_km = distance_km.max(6000.0);
    }

    pub fn reset_camera(&mut self) {
        self.yaw_deg = 18.0;
        self.pitch_deg = 20.0;
        self.orbit_az_deg = 28.0;
        self.orbit_el_deg = 28.0;
        self.orbit_distance_km = 850_000.0;
    }

    fn camera_basis(&self, settings: &Settings) -> (Vec3, Vec3, Vec3) {
        let yaw = self.yaw_deg.to_radians();
        let pitch = self.pitch_deg.to_radians();
        if settings.view == ViewMode::Surface {
            let forward = Vec3::new(yaw.sin() * pitch.cos(), pitch.sin(), -yaw.cos() * pitch.cos());
            let right = Vec3::new(yaw.cos(), 0.0, yaw.sin());
            let up = right.cross(forward).normalize();
            (forward, right, up)
        } else {
            let az = self.orbit_az_deg.to_radians();
            let el = self.orbit_el_deg.to_radians();
            let eye = Vec3::new(az.sin() * el.cos(), el.sin(), -az.cos() * el.cos());
            let forward = -eye;
            let right = Vec3::new(az.cos(), 0.0, az.sin());
            let up = right.cross(forward).normalize();
            (forward, right, up)
        }
    }

    pub fn build_frame(&self, settings: &Settings) -> RenderFrame {
        let (forward, right, up) = self.camera_basis(settings);
        let aspect = (self.viewport.0 / self.viewport.1) as f32;
        let camera = CameraView {
            forward,
            right,
            up,
            fov_y_deg: settings.fov_deg,
            aspect,
            surface: settings.view == ViewMode::Surface,
        };

        let star_dir = dir_from_az_el(55.0, 5.0);
        let vhal_dir = dir_from_az_el(0.0, 42.0);
        let mut bodies = [BodyView {
            id: BodyId::Kesh,
            rel_pos_km: Vec3::ZERO,
            radius_km: 0.0,
            kind: BodyKind::Star,
            axis: Vec3::Y,
            spin_rad: 0.0,
            tint: Vec3::ONE,
            enabled: false,
        }; MAX_BODIES];

        let mut set = |id: BodyId, dir: Vec3, dist: f32, radius: f32, kind: BodyKind, tint: Vec3| {
            bodies[id.index()] = BodyView {
                id,
                rel_pos_km: dir * dist,
                radius_km: radius,
                kind,
                axis: Vec3::Y,
                spin_rad: 0.0,
                tint,
                enabled: true,
            };
        };
        set(BodyId::Kesh, star_dir, 7.5e6, 140_000.0, BodyKind::Star, Vec3::ONE);
        set(BodyId::Vhal, vhal_dir, 250_000.0, 60_000.0, BodyKind::GasGiant, Vec3::ONE);
        set(BodyId::Ossa, -vhal_dir, 250_000.0, 1900.0, BodyKind::Desert, Vec3::ONE);
        set(BodyId::Thren, dir_from_az_el(-25.0, 20.0), 180_000.0, 620.0, BodyKind::Rock, Vec3::new(0.55, 0.5, 0.46));
        set(BodyId::Sable, dir_from_az_el(30.0, 12.0), 300_000.0, 1050.0, BodyKind::Ice, Vec3::new(0.62, 0.68, 0.7));
        set(BodyId::Nadir, dir_from_az_el(120.0, 2.0), 9.0e6, 7500.0, BodyKind::Dark, Vec3::new(0.45, 0.47, 0.5));
        set(BodyId::Cinder, dir_from_az_el(118.0, 1.0), 9.0e6, 280.0, BodyKind::Rock, Vec3::new(0.4, 0.38, 0.35));
        set(BodyId::Sur, dir_from_az_el(-120.0, 8.0), 5.0e6, 3200.0, BodyKind::Rock, Vec3::new(0.5, 0.45, 0.4));

        if settings.view == ViewMode::Surface {
            bodies[BodyId::Ossa.index()].enabled = false;
        }

        let ring = RingView {
            center_rel_km: bodies[BodyId::Vhal.index()].rel_pos_km,
            normal: Vec3::new(0.2, 0.96, 0.18).normalize(),
            inner_km: 81_000.0,
            outer_km: 141_000.0,
            brightness: settings.ring_brightness,
            enabled: true,
        };

        RenderFrame {
            camera,
            bodies,
            ring,
            orbits: Vec::new(),
            view: settings.view,
            quality: settings.quality,
            exposure: settings.exposure,
            haze: settings.haze,
            vhal_light: settings.vhal_light,
            sim_time_s: settings.sim_time,
            sun_visibility: 1.0,
            surface_pos_m: self.surface_pos,
            eye_height_m: self.eye_height_m,
        }
    }

    pub fn build_hud(&self, settings: &Settings, viewport: (f32, f32), _fps: f32) -> HudFrame {
        let frame = self.build_frame(settings);
        let viewport = glam::Vec2::new(viewport.0, viewport.1);
        let mut markers = Vec::new();
        let mut arrows = Vec::new();
        for id in [BodyId::Vhal, BodyId::Kesh] {
            let b = &frame.bodies[id.index()];
            if !b.enabled {
                continue;
            }
            let p = frame.camera.project(b.rel_pos_km, viewport);
            let dist = b.rel_pos_km.length();
            let detail = format_distance_km(dist);
            if p.in_front && p.on_screen {
                markers.push(HudMarker {
                    body: id,
                    label: id.name(),
                    detail,
                    x: p.x,
                    y: p.y,
                    state: MarkerState::Visible,
                });
            } else {
                let e = frame.camera.edge_arrow(b.rel_pos_km, viewport, 30.0);
                arrows.push(HudArrow {
                    body: id,
                    label: id.name(),
                    x: e.x,
                    y: e.y,
                    angle_deg: e.angle_deg,
                    detail,
                    state: MarkerState::Visible,
                });
            }
        }
        let telemetry = Telemetry {
            mode_line: settings.view.label().to_string(),
            location_line: "48.0°N 12.4°E".to_string(),
            vhal_phase: "76.5%".to_string(),
            kesh_elevation: "+5.0°".to_string(),
            ring_tilt: "12.0°".to_string(),
            clock: "DAY 0001 · 06:42:10".to_string(),
            perf: "60 · HIGH".to_string(),
            scale: None,
            focus: settings.focus,
            view: settings.view,
            show_markers: settings.show_markers,
            show_orbits: settings.show_orbits,
        };
        HudFrame {
            markers,
            arrows,
            telemetry,
        }
    }
}

fn dir_from_az_el(az_deg: f32, el_deg: f32) -> Vec3 {
    let az = az_deg.to_radians();
    let el = el_deg.to_radians();
    Vec3::new(az.sin() * el.cos(), el.sin(), -az.cos() * el.cos()).normalize()
}

// --- bootstrap-only additions used by the director shell -------------------
impl Scene {
    pub fn set_focus_instant(&mut self, focus: Focus) {
        let _ = focus;
    }
    pub fn set_move_axis(&mut self, _x: f32, _y: f32) {}
}
