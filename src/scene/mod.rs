//! Simulation, camera controllers and display-ready HUD data.
//! See ephemeris.rs for the deterministic observer/epoch solve.
mod ephemeris;
mod input;
mod visibility;

use crate::{frame::*, settings::*, terrain_field};
use ephemeris::{RADII, System};
use glam::{DVec2, DVec3, Vec2, Vec3};
use std::{cell::RefCell, f64::consts::TAU, rc::Rc};
use wasm_bindgen::JsValue;

pub struct Scene {
    viewport: (f64, f64),
    system: System,
    positions: [DVec3; MAX_BODIES],
    sim_time: f64,
    input: Rc<RefCell<input::Input>>,
    surface_pos: [f64; 2],
    velocity: DVec2,
    joystick: DVec2,
    eye_height_m: f32,
    walk_phase: f64,
    yaw_deg: f64,
    pitch_deg: f64,
    yaw_target: f64,
    pitch_target: f64,
    orbit_az_deg: f64,
    orbit_el_deg: f64,
    orbit_distance_km: f64,
    orbit_distance_target: f64,
    focus: Focus,
    focus_center: DVec3,
    focus_from: DVec3,
    focus_distance_from: f64,
    focus_elapsed: f64,
    view: ViewMode,
    pending_view: Option<(ViewMode, f64)>,
    settle: f64,
    default_fov_pending: bool,
    #[cfg(target_arch = "wasm32")]
    hud_layout: RefCell<HudLayout>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct HudLayout {
    key: [u64; 5],
    settle_frames: u8,
    deck: Option<[f32; 4]>,
}

fn focus_body(f: Focus) -> BodyId {
    match f {
        Focus::Ossa => BodyId::Ossa,
        Focus::Vhal => BodyId::Vhal,
        Focus::Kesh | Focus::System => BodyId::Kesh,
    }
}
fn preset_distance(f: Focus) -> f64 {
    match f {
        Focus::Ossa => 8000.,
        Focus::Vhal => 420_000.,
        Focus::Kesh => 12_000_000.,
        Focus::System => 18_000_000.,
    }
}
fn smoothstep(t: f64) -> f64 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn camera_basis(yaw: f64, pitch: f64) -> (Vec3, Vec3, Vec3) {
    let y = yaw.to_radians();
    let p = pitch.to_radians();
    let forward = Vec3::new(
        (y.sin() * p.cos()) as f32,
        p.sin() as f32,
        (-y.cos() * p.cos()) as f32,
    );
    let right = Vec3::new(y.cos() as f32, 0., y.sin() as f32);
    (forward, right, right.cross(forward).normalize())
}

impl Scene {
    pub fn new(
        canvas: &web_sys::HtmlCanvasElement,
        settings: &Settings,
        viewport: (f64, f64),
    ) -> Result<Self, JsValue> {
        let eye = terrain_field::height_m(0., 0.) + 1.7;
        let system = System::new(eye);
        let positions = system.positions(settings.sim_time);
        let center = positions[focus_body(settings.focus).index()];
        let portrait = viewport.0 / viewport.1 < 0.85;
        let yaw = if portrait { 32. } else { 37. };
        let pitch = if portrait { 15. } else { 17.5 };
        let has_fov = web_sys::window()
            .and_then(|w| w.location().search().ok())
            .and_then(|s| web_sys::UrlSearchParams::new_with_str(&s).ok())
            .and_then(|p| p.get("fov"))
            .is_some();
        let s = Self {
            viewport,
            system,
            positions,
            sim_time: settings.sim_time,
            input: input::install(canvas)?,
            surface_pos: [0.; 2],
            velocity: DVec2::ZERO,
            joystick: DVec2::ZERO,
            eye_height_m: eye,
            walk_phase: 0.,
            yaw_deg: yaw,
            pitch_deg: pitch,
            yaw_target: yaw,
            pitch_target: pitch,
            orbit_az_deg: 90.,
            orbit_el_deg: 18.,
            orbit_distance_km: preset_distance(settings.focus),
            orbit_distance_target: preset_distance(settings.focus),
            focus: settings.focus,
            focus_center: center,
            focus_from: center,
            focus_distance_from: preset_distance(settings.focus),
            focus_elapsed: 2.2,
            view: settings.view,
            pending_view: None,
            settle: 0.,
            default_fov_pending: !has_fov,
            #[cfg(target_arch = "wasm32")]
            hud_layout: RefCell::new(HudLayout::default()),
        };
        s.log_epoch();
        Ok(s)
    }
    pub fn set_viewport(&mut self, w: f64, h: f64) {
        self.viewport = (w.max(1.), h.max(1.));
    }

    pub fn update(&mut self, dt: f64, settings: &mut Settings) {
        let dt = if dt.is_finite() {
            dt.clamp(0., 0.1)
        } else {
            0.
        };
        // Compatibility with the bootstrap Settings default (70) and mobile
        // shell (74). Explicit URL/UI FOV values are never overwritten.
        if self.default_fov_pending {
            if settings.fov_deg == 70. || settings.fov_deg == 74. {
                settings.fov_deg = if self.viewport.0 / self.viewport.1 < 0.85 {
                    78.
                } else {
                    76.
                };
            }
            self.default_fov_pending = false;
        }
        if !settings.paused {
            settings.sim_time += dt * settings.time_scale;
        }
        if !settings.sim_time.is_finite() {
            settings.sim_time = 0.;
        }
        self.sim_time = settings.sim_time;
        self.positions = self.system.positions(self.sim_time);
        if let Some((target, elapsed)) = self.pending_view {
            if elapsed + dt >= 0.26 || settings.reduced_motion {
                settings.view = target;
                self.view = target;
                self.pending_view = None;
                self.settle = if settings.reduced_motion { 0. } else { 0.42 };
                self.velocity = DVec2::ZERO;
            } else {
                self.pending_view = Some((target, elapsed + dt));
            }
        } else {
            self.view = settings.view;
        }
        self.settle = (self.settle - dt).max(0.);
        if settings.focus != self.focus {
            self.begin_focus(settings.focus, settings.reduced_motion);
        }
        let (look, zoom, keys) = {
            let mut i = self.input.borrow_mut();
            let result = (i.look, i.zoom, i.keys);
            i.look = [0.; 2];
            i.zoom = 0.;
            result
        };
        if self.view == ViewMode::Surface {
            self.yaw_target += look[0] * 0.16;
            self.pitch_target = (self.pitch_target - look[1] * 0.14).clamp(-85., 85.);
            let damping = if settings.reduced_motion {
                1.
            } else {
                1. - (-dt / 0.035).exp()
            };
            self.yaw_deg += (self.yaw_target - self.yaw_deg) * damping;
            self.pitch_deg += (self.pitch_target - self.pitch_deg) * damping;
            self.walk(dt, keys, settings.reduced_motion);
        } else {
            self.orbit_az_deg -= look[0] * 0.20;
            self.orbit_el_deg = (self.orbit_el_deg + look[1] * 0.17).clamp(-85., 85.);
            if zoom != 0. {
                self.orbit_distance_target *= zoom.clamp(-2., 2.).exp();
            }
        }
        let minimum = match self.focus {
            Focus::Ossa => 6000.,
            Focus::Vhal => RADII[1] * 1.15,
            Focus::Kesh | Focus::System => RADII[0] * 1.15,
        };
        self.orbit_distance_target = self.orbit_distance_target.clamp(minimum, 40_000_000.);
        let target = self.positions[focus_body(self.focus).index()];
        if self.focus_elapsed < 2.2 {
            self.focus_elapsed = (self.focus_elapsed + dt).min(2.2);
            let t = smoothstep(self.focus_elapsed / 2.2);
            self.focus_center = self.focus_from.lerp(target, t);
            self.orbit_distance_km = self.focus_distance_from
                + (self.orbit_distance_target - self.focus_distance_from) * t;
        } else {
            self.focus_center = target;
            let damping = if settings.reduced_motion {
                1.
            } else {
                1. - (-dt / 0.12).exp()
            };
            self.orbit_distance_km +=
                (self.orbit_distance_target - self.orbit_distance_km) * damping;
        }
        self.orbit_distance_km = self.orbit_distance_km.max(minimum);
    }
    fn walk(&mut self, dt: f64, keys: [bool; 5], reduced: bool) {
        let mut axis = self.joystick
            + DVec2::new(
                (keys[3] as u8 as f64) - (keys[2] as u8 as f64),
                (keys[0] as u8 as f64) - (keys[1] as u8 as f64),
            );
        if axis.length_squared() > 1. {
            axis = axis.normalize();
        }
        let yaw = self.yaw_deg.to_radians();
        let desired = DVec2::new(
            axis.x * yaw.cos() + axis.y * yaw.sin(),
            -axis.x * yaw.sin() + axis.y * yaw.cos(),
        ) * if keys[4] { 6.4 } else { 2.1 };
        self.velocity = self.velocity.lerp(
            desired,
            1. - (-dt
                / if axis.length_squared() > 0. {
                    0.20
                } else {
                    0.13
                })
            .exp(),
        );
        let next = [
            self.surface_pos[0] + self.velocity.x * dt,
            self.surface_pos[1] + self.velocity.y * dt,
        ];
        let normal = terrain_field::normal_m(next[0], next[1], 0.7);
        if normal.y >= terrain_field::MAX_WALK_SLOPE_DEG.to_radians().cos() {
            self.surface_pos = next;
        } else {
            // Slide along a steep contour instead of climbing or flying.
            let gradient = DVec2::new(-normal.x as f64, -normal.z as f64).normalize_or_zero();
            self.velocity -= gradient * self.velocity.dot(gradient).max(0.);
            let slide = [
                self.surface_pos[0] + self.velocity.x * dt,
                self.surface_pos[1] + self.velocity.y * dt,
            ];
            if terrain_field::normal_m(slide[0], slide[1], 0.7).y
                >= terrain_field::MAX_WALK_SLOPE_DEG.to_radians().cos()
            {
                self.surface_pos = slide;
            }
        }
        self.walk_phase += self.velocity.length() * dt * 3.1;
        let bob = if reduced {
            0.
        } else {
            (self.walk_phase * 2.).sin() as f32
                * 0.018
                * (self.velocity.length() / 2.1).min(1.) as f32
        };
        let ground = terrain_field::height_at_m(self.surface_pos);
        let target = ground + 1.7 + bob;
        self.eye_height_m += (target - self.eye_height_m) * (1. - (-dt / 0.085).exp()) as f32;
        self.eye_height_m = self.eye_height_m.max(ground + 1.52);
    }
    pub fn apply_event(&mut self, event: UiEvent, settings: &mut Settings) {
        match event {
            UiEvent::SetView(v) => {
                if v == self.view {
                    self.pending_view = None;
                } else if settings.reduced_motion {
                    settings.view = v;
                    self.set_view_instant(v);
                } else if self.pending_view.map(|p| p.0) != Some(v) {
                    self.pending_view = Some((v, 0.));
                }
            }
            UiEvent::SetFocus(f) => {
                settings.focus = f;
                self.begin_focus(f, settings.reduced_motion);
            }
            UiEvent::SetQuality(q) => settings.quality = q,
            UiEvent::SetPaused(p) => settings.paused = p,
            UiEvent::SetTimeScale(s) => {
                if s.is_finite() {
                    settings.time_scale = s.clamp(0., 100_000.);
                }
            }
            UiEvent::ScrubTime(f) => self.scrub(f, settings),
            UiEvent::SetExposure(v) => settings.exposure = v.clamp(0.6, 1.6),
            UiEvent::SetHaze(v) => settings.haze = v.clamp(0., 1.5),
            UiEvent::SetRings(v) => settings.ring_brightness = v.clamp(0., 1.5),
            UiEvent::SetVhalLight(v) => settings.vhal_light = v.clamp(0., 1.5),
            UiEvent::SetFov(v) => {
                settings.fov_deg = v.clamp(55., 95.);
                self.default_fov_pending = false;
            }
            UiEvent::ToggleMarkers => settings.show_markers = !settings.show_markers,
            UiEvent::ToggleOrbits => settings.show_orbits = !settings.show_orbits,
            UiEvent::ToggleUi => settings.show_ui = !settings.show_ui,
            UiEvent::ResetCamera => self.reset_camera(),
            UiEvent::SetMoveAxis(x, y) => self.set_move_axis(x, y),
            UiEvent::Fullscreen => {}
        }
    }
    fn begin_focus(&mut self, f: Focus, instant: bool) {
        if instant {
            self.set_focus_instant(f);
            return;
        }
        self.focus_from = self.focus_center;
        self.focus_distance_from = self.orbit_distance_km;
        self.focus = f;
        self.focus_elapsed = 0.;
        self.orbit_distance_target = preset_distance(f);
    }
    pub fn set_view_instant(&mut self, v: ViewMode) {
        self.view = v;
        self.pending_view = None;
        self.settle = 0.;
        self.velocity = DVec2::ZERO;
    }
    pub fn set_focus_instant(&mut self, f: Focus) {
        self.focus = f;
        self.focus_center = self.positions[focus_body(f).index()];
        self.focus_from = self.focus_center;
        self.focus_elapsed = 2.2;
        self.orbit_distance_km = preset_distance(f);
        self.orbit_distance_target = self.orbit_distance_km;
    }
    pub fn set_sim_time(&mut self, t: f64, settings: &mut Settings) {
        settings.sim_time = if t.is_finite() { t } else { 0. };
        self.sim_time = settings.sim_time;
        self.positions = self.system.positions(self.sim_time);
        if self.focus_elapsed >= 2.2 {
            self.focus_center = self.positions[focus_body(self.focus).index()];
        }
    }
    pub fn scrub(&mut self, f: f32, settings: &mut Settings) {
        // Epoch is day 0001, 00:00:00. Scrubbing never changes that origin.
        if f.is_finite() {
            self.set_sim_time(f.clamp(0., 1.) as f64 * self.system.period(), settings);
        }
    }
    pub fn set_surface_look(&mut self, yaw: f32, pitch: f32) {
        if yaw.is_finite() {
            self.yaw_deg = yaw as f64;
            self.yaw_target = self.yaw_deg;
        }
        if pitch.is_finite() {
            self.pitch_deg = pitch.clamp(-85., 85.) as f64;
            self.pitch_target = self.pitch_deg;
        }
    }
    pub fn set_orbit_camera(&mut self, az: f32, el: f32, distance: f64) {
        if az.is_finite() {
            self.orbit_az_deg = az as f64;
        }
        if el.is_finite() {
            self.orbit_el_deg = el.clamp(-85., 85.) as f64;
        }
        if distance.is_finite() {
            let minimum = RADII[focus_body(self.focus).index()] * 1.15;
            self.orbit_distance_km = distance.clamp(minimum.max(6000.), 40_000_000.);
            self.orbit_distance_target = self.orbit_distance_km;
        }
    }
    pub fn set_move_axis(&mut self, x: f32, y: f32) {
        self.joystick = if x.is_finite() && y.is_finite() {
            DVec2::new(x as f64, y as f64).clamp_length_max(1.)
        } else {
            DVec2::ZERO
        };
    }
    pub fn reset_camera(&mut self) {
        let portrait = self.viewport.0 / self.viewport.1 < 0.85;
        self.set_surface_look(
            if portrait { 32. } else { 37. },
            if portrait { 15. } else { 17.5 },
        );
        self.surface_pos = [0.; 2];
        self.velocity = DVec2::ZERO;
        self.joystick = DVec2::ZERO;
        self.walk_phase = 0.;
        self.eye_height_m = terrain_field::height_m(0., 0.) + 1.7;
        self.orbit_az_deg = 90.;
        self.orbit_el_deg = 18.;
        self.set_focus_instant(self.focus);
        let mut input = self.input.borrow_mut();
        input.look = [0.; 2];
        input.zoom = 0.;
        input.keys = [false; 5];
    }
    fn observer(&self, settings: &Settings) -> (DVec3, [DVec3; 3], CameraView) {
        let (position, basis, forward, right, up) = if settings.view == ViewMode::Surface {
            let (east, vertical, south) =
                self.system.surface_basis(&self.positions, self.surface_pos);
            let position = self.positions[BodyId::Ossa.index()]
                + vertical * (1900. + self.eye_height_m as f64 / 1000.);
            let (f, r, u) = camera_basis(self.yaw_deg, self.pitch_deg);
            (position, [east, vertical, south], f, r, u)
        } else {
            let az = self.orbit_az_deg.to_radians();
            let el = self.orbit_el_deg.to_radians();
            // Observe the sunward (southern) face of the equatorial plane.
            // A plane has two sides; positive camera elevation here is above
            // its -spin-axis face, allowing a gibbous, not backlit, Vhal.
            let direction = (self.system.orbit_x * az.sin() - self.system.orbit_z * az.cos())
                * el.cos()
                - self.system.ring_normal * el.sin();
            let position = self.focus_center + direction * self.orbit_distance_km;
            let forward = (-direction).as_vec3();
            let right = forward.cross(Vec3::Y).normalize();
            let up = right.cross(forward).normalize();
            (position, [DVec3::X, DVec3::Y, DVec3::Z], forward, right, up)
        };
        let settle = if settings.view == ViewMode::Surface {
            6. * smoothstep(self.settle / 0.42) as f32
        } else {
            0.
        };
        let camera = CameraView {
            forward,
            right,
            up,
            fov_y_deg: settings.fov_deg + settle,
            aspect: (self.viewport.0 / self.viewport.1) as f32,
            surface: settings.view == ViewMode::Surface,
        };
        (position, basis, camera)
    }
    fn frame(&self, settings: &Settings, include_orbits: bool) -> RenderFrame {
        let (eye, basis, camera) = self.observer(settings);
        let local = |v: DVec3| {
            Vec3::new(
                v.dot(basis[0]) as f32,
                v.dot(basis[1]) as f32,
                v.dot(basis[2]) as f32,
            )
        };
        let kinds = [
            BodyKind::Star,
            BodyKind::GasGiant,
            BodyKind::Desert,
            BodyKind::Rock,
            BodyKind::Ice,
            BodyKind::Dark,
            BodyKind::Rock,
            BodyKind::Rock,
        ];
        let tints = [
            Vec3::ONE,
            Vec3::ONE,
            Vec3::new(0.48, 0.42, 0.34),
            Vec3::new(0.43, 0.42, 0.39),
            Vec3::new(0.58, 0.65, 0.67),
            Vec3::new(0.25, 0.29, 0.32),
            Vec3::new(0.36, 0.34, 0.31),
            Vec3::new(0.47, 0.44, 0.38),
        ];
        let bodies = std::array::from_fn(|i| {
            let id = BodyId::ALL[i];
            let e = self.system.elements[i];
            let axis = if id == BodyId::Vhal {
                self.system.ring_normal
            } else {
                e.map(|e| e.normal()).unwrap_or(DVec3::Y)
            };
            let spin = if id == BodyId::Vhal {
                (self.sim_time.rem_euclid(57_600.) * TAU / 57_600.) as f32
            } else if matches!(
                id,
                BodyId::Ossa | BodyId::Thren | BodyId::Sable | BodyId::Cinder
            ) {
                e.unwrap().mean_anomaly(self.sim_time) as f32
            } else {
                (self.sim_time.rem_euclid(86_400.) * TAU / 86_400.) as f32
            };
            BodyView {
                id,
                rel_pos_km: local(self.positions[i] - eye),
                radius_km: RADII[i] as f32,
                kind: kinds[i],
                axis: local(axis),
                spin_rad: spin,
                tint: tints[i],
                enabled: !(settings.view == ViewMode::Surface && id == BodyId::Ossa),
            }
        });
        let ring = RingView {
            center_rel_km: bodies[1].rel_pos_km,
            normal: local(self.system.ring_normal),
            inner_km: 81_000.,
            outer_km: 141_000.,
            // The orbit view is more face-on, so the same optical depth is
            // dimmer; lift the material slightly for legibility. The UI
            // slider remains the authority.
            brightness: if settings.view == ViewMode::Orbit {
                settings.ring_brightness * 1.5
            } else {
                settings.ring_brightness
            },
            enabled: true,
        };
        let mut orbits = Vec::new();
        if include_orbits && settings.view == ViewMode::Orbit && settings.show_orbits {
            orbits.reserve(7);
            for id in [
                BodyId::Sur,
                BodyId::Vhal,
                BodyId::Nadir,
                BodyId::Thren,
                BodyId::Ossa,
                BodyId::Sable,
                BodyId::Cinder,
            ] {
                let e = self.system.elements[id.index()].unwrap();
                let origin = self.positions[e.parent.index()] - eye;
                let moon = e.parent != BodyId::Kesh;
                let focused = id == focus_body(settings.focus);
                orbits.push(OrbitCurve {
                    points_rel: self.system.curves[id.index()]
                        .iter()
                        .map(|p| local(origin + *p))
                        .collect(),
                    color: if moon {
                        Vec3::new(0.38, 0.54, 0.55)
                    } else {
                        Vec3::new(0.32, 0.40, 0.43)
                    },
                    alpha: if focused {
                        0.40
                    } else if moon {
                        0.31
                    } else {
                        0.22
                    },
                    width_px: 1.,
                });
            }
        }
        let sun_visibility = visibility::sun_visibility(&bodies);
        RenderFrame {
            camera,
            bodies,
            ring,
            orbits,
            view: settings.view,
            quality: settings.quality,
            exposure: settings.exposure,
            haze: settings.haze,
            vhal_light: settings.vhal_light,
            sim_time_s: self.sim_time,
            sun_visibility,
            surface_pos_m: self.surface_pos,
            eye_height_m: self.eye_height_m,
        }
    }
    pub fn build_frame(&self, settings: &Settings) -> RenderFrame {
        self.frame(settings, true)
    }

    pub fn build_hud(&self, settings: &Settings, viewport: (f32, f32), fps: f32) -> HudFrame {
        // Do not generate a second copy of the 1351 orbit vertices for the HUD.
        let frame = self.frame(settings, false);
        let viewport = Vec2::new(viewport.0.max(1.), viewport.1.max(1.));
        let mut markers = Vec::with_capacity(9);
        let mut arrows = Vec::with_capacity(3);
        let deck = self.hud_bounds(settings, viewport);
        if settings.show_markers {
            let extra = focus_body(settings.focus);
            let ids = if settings.view == ViewMode::Orbit
                && extra != BodyId::Vhal
                && extra != BodyId::Kesh
            {
                [Some(BodyId::Vhal), Some(BodyId::Kesh), Some(extra)]
            } else {
                [Some(BodyId::Vhal), Some(BodyId::Kesh), None]
            };
            for id in ids.into_iter().flatten() {
                let body = frame.bodies[id.index()];
                let p = frame.camera.project(body.rel_pos_km, viewport);
                let state = visibility::marker_state(&frame, id);
                let mut detail = format_distance_km(body.rel_pos_km.length());
                if state.is_warning() {
                    detail.push_str(" · ");
                    detail.push_str(state.detail());
                }
                if p.in_front
                    && p.x >= 12.
                    && p.x <= viewport.x - 12.
                    && p.y >= 12.
                    && p.y <= viewport.y - 12.
                {
                    let (x, y) = safe_anchor(p.x, p.y, viewport, false, deck);
                    markers.push(HudMarker {
                        body: id,
                        label: id.name(),
                        detail,
                        x,
                        y,
                        state,
                    });
                } else {
                    let a = frame.camera.edge_arrow(body.rel_pos_km, viewport, 28.);
                    let (x, mut y) = safe_anchor(a.x, a.y, viewport, true, deck);
                    for other in &arrows {
                        let other: &HudArrow = other;
                        if (other.x - x).abs() < 150. && (other.y - y).abs() < 24. {
                            y = if y > 190. { y - 28. } else { y + 28. };
                        }
                    }
                    arrows.push(HudArrow {
                        body: id,
                        label: id.name(),
                        detail,
                        x,
                        y,
                        angle_deg: a.angle_deg,
                        state,
                    });
                }
            }
        }
        // Orbit view: quiet labels for the rest of the system. Surface view
        // keeps only the required Vhal/Kesh markers to protect the composition.
        if settings.view == ViewMode::Orbit {
            let primary = [BodyId::Vhal, BodyId::Kesh, focus_body(settings.focus)];
            for id in BodyId::ALL {
                if primary.contains(&id) {
                    continue;
                }
                let body = frame.bodies[id.index()];
                if !body.enabled {
                    continue;
                }
                let p = frame.camera.project(body.rel_pos_km, viewport);
                if !p.in_front
                    || p.x < 18.
                    || p.x > viewport.x - 18.
                    || p.y < 18.
                    || p.y > viewport.y - 18.
                {
                    continue;
                }
                // Keep the system legible rather than labelling every overlap.
                if markers
                    .iter()
                    .any(|m| (m.x - p.x).abs() < 30. && (m.y - p.y).abs() < 18.)
                {
                    continue;
                }
                let (x, y) = safe_anchor(p.x, p.y, viewport, false, deck);
                markers.push(HudMarker {
                    body: id,
                    label: id.name(),
                    detail: format_distance_km(body.rel_pos_km.length()),
                    x,
                    y,
                    state: visibility::marker_state(&frame, id),
                });
            }
        }
        let vhal = frame.bodies[1].rel_pos_km.as_dvec3();
        let sun = frame.bodies[0].rel_pos_km.as_dvec3();
        let phase = 0.5 * (1. + (sun - vhal).normalize().dot((-vhal).normalize()));
        let ring_tilt = frame
            .ring
            .normal
            .as_dvec3()
            .dot(vhal.normalize())
            .abs()
            .clamp(0., 1.)
            .asin()
            .to_degrees();
        let elapsed = self.sim_time.rem_euclid(self.system.period()).floor() as u64;
        let day = (self.sim_time / self.system.period()).floor() as i64 + 1;
        let scale = if settings.view == ViewMode::Orbit {
            let width = 2. * self.orbit_distance_km * frame.camera.tan_half_fov_x() as f64;
            let desired = width * 0.13;
            let decade = 10_f64.powf(desired.max(1.).log10().floor());
            let leading = desired / decade;
            let nice = if leading >= 5. {
                5.
            } else if leading >= 2. {
                2.
            } else {
                1.
            } * decade;
            Some(format!("{} KM", format_thousands_int(nice)))
        } else {
            None
        };
        let telemetry = Telemetry {
            mode_line: settings.view.label().into(),
            location_line: if settings.view == ViewMode::Surface {
                format!(
                    "{:.1}°N · VHAL MERIDIAN",
                    (ephemeris::LATITUDE - self.surface_pos[1] / 1_900_000.).to_degrees()
                )
            } else {
                settings.focus.label().into()
            },
            vhal_phase: format!("{:.1}%", phase * 100.),
            kesh_elevation: if settings.view == ViewMode::Surface {
                format!("{:+.1}°", sun.normalize().y.asin().to_degrees())
            } else {
                "--".into()
            },
            ring_tilt: format!("{:.1}°", ring_tilt),
            clock: format!(
                "DAY {day:04} · {:02}:{:02}:{:02}",
                elapsed / 3600,
                (elapsed / 60) % 60,
                elapsed % 60
            ),
            perf: format!("{:.0} · {}", fps.max(0.), settings.quality.label()),
            scale,
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
    /// Read layout only after viewport/view/deck-collapse changes, never a
    /// per-frame forced layout. Both left- and right-docked UI are supported.
    #[cfg(target_arch = "wasm32")]
    fn hud_bounds(&self, settings: &Settings, viewport: Vec2) -> Option<[f32; 4]> {
        if !settings.show_ui {
            return None;
        }
        let deck = web_sys::window()?.document()?.get_element_by_id("deck")?;
        let key = [
            viewport.x as u64,
            viewport.y as u64,
            (settings.view == ViewMode::Orbit) as u64,
            deck.class_list().contains("collapsed") as u64,
            settings.show_ui as u64,
        ];
        let mut cache = self.hud_layout.borrow_mut();
        if key != cache.key {
            cache.key = key;
            cache.settle_frames = 3;
        }
        if cache.settle_frames > 0 {
            cache.settle_frames -= 1;
            let rect = deck.get_bounding_client_rect();
            cache.deck = if rect.width() > 1. && rect.height() > 1. {
                Some([
                    rect.left() as f32,
                    rect.top() as f32,
                    rect.right() as f32,
                    rect.bottom() as f32,
                ])
            } else {
                None
            };
        }
        cache.deck
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn hud_bounds(&self, _settings: &Settings, _viewport: Vec2) -> Option<[f32; 4]> {
        None
    }

    fn log_epoch(&self) {
        // Init already holds t=0 unless the URL explicitly selects another
        // epoch. Report the canonical solution independently of URL QA state.
        let positions = self.system.positions(0.);
        let (e, u, s) = self.system.surface_basis(&positions, [0.; 2]);
        let eye =
            positions[2] + u * (1900. + (terrain_field::height_m(0., 0.) + 1.7) as f64 / 1000.);
        let local = |v: DVec3| DVec3::new(v.dot(e), v.dot(u), v.dot(s));
        let vhal = local(positions[1] - eye);
        let sun = local(-eye);
        let elevation = |v: DVec3| v.normalize().y.asin().to_degrees();
        let azimuth = |v: DVec3| v.x.atan2(-v.z).to_degrees();
        let elongation = vhal.normalize().dot(sun.normalize()).acos().to_degrees();
        let phase = 0.5 * (1. + (sun - vhal).normalize().dot(-vhal.normalize()));
        let normal = local(self.system.ring_normal);
        let opening = normal.dot(vhal.normalize()).abs().asin().to_degrees();
        let (f, r, up) = camera_basis(37., 17.5);
        let camera = CameraView {
            forward: f,
            right: r,
            up,
            fov_y_deg: 76.,
            aspect: 1.6,
            surface: true,
        };
        let vp = Vec2::new(1440., 900.);
        let pv = camera.project(vhal.as_vec3(), vp);
        let ps = camera.project(sun.as_vec3(), vp);
        // Conic of a projected physical circle, in camera x/right,y/down,z.
        let c = DVec3::new(
            vhal.dot(r.as_dvec3()),
            -vhal.dot(up.as_dvec3()),
            vhal.dot(f.as_dvec3()),
        );
        let n = DVec3::new(
            normal.dot(r.as_dvec3()),
            -normal.dot(up.as_dvec3()),
            normal.dot(f.as_dvec3()),
        );
        let k = n.dot(c);
        let g = c.length_squared() - 141_000_f64.powi(2);
        let a = k * k - 2. * k * c.x * n.x + g * n.x * n.x;
        let b = -k * (c.x * n.y + c.y * n.x) + g * n.x * n.y;
        let d = k * k - 2. * k * c.y * n.y + g * n.y * n.y;
        let pa = (0.5 * (2. * b).atan2(a - d).to_degrees() + 180.).rem_euclid(180.) - 90.;
        let mut report = format!(
            "[ossa scene] EPOCH t=0: Vhal el={:.5} az={:.5} distance={:.3}km diameter={:.4}deg; Kesh el={:.5} az={:.5} diameter={:.4}deg; elongation={:.5} phase={:.5}%; ring opening={:.5} screen PA={:.5}deg; pixels 1440x900 Vhal=({:.2},{:.2}) Kesh=({:.2},{:.2}) both_on_screen={};",
            elevation(vhal),
            azimuth(vhal),
            vhal.length(),
            2. * (60_000. / vhal.length()).asin().to_degrees(),
            elevation(sun),
            azimuth(sun),
            2. * (140_000. / sun.length()).asin().to_degrees(),
            elongation,
            phase * 100.,
            opening,
            pa,
            pv.x,
            pv.y,
            ps.x,
            ps.y,
            pv.on_screen && ps.on_screen
        );
        for id in BodyId::ALL {
            if let Some(e) = self.system.elements[id.index()] {
                report.push_str(&format!(" {}={:.8}h", id.name(), e.period() / 3600.));
            }
        }
        web_sys::console::log_1(&report.into());
    }
}

/// Move label anchors away from fixed UI, preserving an arrow's true angle.
fn safe_anchor(mut x: f32, mut y: f32, v: Vec2, arrow: bool, deck: Option<[f32; 4]>) -> (f32, f32) {
    let wide = v.x > 700.;
    let left = if arrow { 28. } else { 38. };
    x = x.clamp(left, (v.x - 28.).max(left));
    y = y.clamp(28., (v.y - 46.).max(28.));
    if y < 158. && (x < 260. || x > v.x - 280.) {
        y = 158_f32.min(v.y - 48.);
    }
    if let Some([left, top, right, bottom]) = deck {
        let text_left = if x > v.x - 260. { x - 235. } else { x - 8. };
        let text_right = if x > v.x - 260. { x + 8. } else { x + 235. };
        if text_right > left - 12. && text_left < right + 12. && y + 22. > top && y - 10. < bottom {
            y = (top - 30.).max(158.);
        }
    }
    if !wide && y > v.y - 100. {
        y = v.y - 110.;
    }
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Scene, Settings) {
        let settings = Settings {
            fov_deg: 76.,
            paused: true,
            ..Settings::default()
        };
        let eye = terrain_field::height_m(0., 0.) + 1.7;
        let system = System::new(eye);
        let positions = system.positions(0.);
        let center = positions[1];
        (
            Scene {
                viewport: (1440., 900.),
                system,
                positions,
                sim_time: 0.,
                input: Rc::new(RefCell::new(input::Input::default())),
                surface_pos: [0.; 2],
                velocity: DVec2::ZERO,
                joystick: DVec2::ZERO,
                eye_height_m: eye,
                walk_phase: 0.,
                yaw_deg: 37.,
                pitch_deg: 17.5,
                yaw_target: 37.,
                pitch_target: 17.5,
                orbit_az_deg: 90.,
                orbit_el_deg: 18.,
                orbit_distance_km: 420_000.,
                orbit_distance_target: 420_000.,
                focus: Focus::Vhal,
                focus_center: center,
                focus_from: center,
                focus_distance_from: 420_000.,
                focus_elapsed: 2.2,
                view: ViewMode::Surface,
                pending_view: None,
                settle: 0.,
                default_fov_pending: false,
                #[cfg(target_arch = "wasm32")]
                hud_layout: RefCell::new(HudLayout::default()),
            },
            settings,
        )
    }
    #[test]
    fn frame_data_cost() {
        let (mut scene, mut settings) = fixture();
        for view in [ViewMode::Surface, ViewMode::Orbit] {
            settings.view = view;
            scene.set_view_instant(view);
            let start = std::time::Instant::now();
            for _ in 0..1000 {
                std::hint::black_box(scene.build_frame(&settings));
                std::hint::black_box(scene.build_hud(&settings, (1440., 900.), 60.));
            }
            println!(
                "Native optimized {:?} frame+HUD CPU: {:.3}ms (not browser/GPU timing)",
                view,
                start.elapsed().as_secs_f64()
            );
        }
    }
    #[test]
    fn controls_frames_and_hud() {
        let (mut scene, mut settings) = fixture();
        let hud = scene.build_hud(&settings, (1440., 900.), 60.);
        assert_eq!(hud.markers.len(), 2);
        assert!(hud.arrows.is_empty());
        assert_eq!(hud.telemetry.clock, "DAY 0001 · 00:00:00");
        assert!(hud.markers.iter().all(|m| m.state == MarkerState::Visible));
        assert!(scene.build_frame(&settings).sun_visibility > 0.999);
        scene.set_surface_look(180., 0.);
        let hud = scene.build_hud(&settings, (1440., 900.), 60.);
        assert_eq!(hud.arrows.len(), 2);
        for a in &hud.arrows {
            assert!(a.angle_deg.is_finite() && a.x.is_finite() && a.y.is_finite());
        }
        scene.scrub(1., &mut settings);
        assert!((settings.sim_time - scene.system.period()).abs() < 1e-8);
        assert_eq!(
            scene
                .build_hud(&settings, (1440., 900.), 60.)
                .telemetry
                .clock,
            "DAY 0002 · 00:00:00"
        );
        scene.set_sim_time(0., &mut settings);
        scene.reset_camera();
        settings.view = ViewMode::Orbit;
        scene.set_view_instant(ViewMode::Orbit);
        let frame = scene.build_frame(&settings);
        assert_eq!(frame.orbits.len(), 7);
        assert!(frame.bodies.iter().all(|b| b.enabled));
        assert!(frame.orbits.iter().all(|c| c.points_rel.len() == 193));
        assert!((frame.camera.forward.dot(frame.camera.up)).abs() < 1e-6);
        for id in [
            BodyId::Vhal,
            BodyId::Ossa,
            BodyId::Thren,
            BodyId::Sable,
            BodyId::Kesh,
        ] {
            let p = frame
                .camera
                .project(frame.bodies[id.index()].rel_pos_km, Vec2::new(1440., 900.));
            println!(
                "Orbit {} pixels {},{} visible={}",
                id.name(),
                p.x,
                p.y,
                p.on_screen
            );
        }
        let tilt = frame
            .ring
            .normal
            .dot(frame.bodies[1].rel_pos_km.normalize())
            .abs()
            .asin()
            .to_degrees();
        println!(
            "Default orbit phase {} ring tilt {}",
            scene
                .build_hud(&settings, (1440., 900.), 60.)
                .telemetry
                .vhal_phase,
            tilt
        );
        assert!((tilt - 18.).abs() < 0.001);
        let phase: f32 = scene
            .build_hud(&settings, (1440., 900.), 60.)
            .telemetry
            .vhal_phase
            .trim_end_matches('%')
            .parse()
            .unwrap();
        assert!((25.0..=90.0).contains(&phase), "orbit phase {phase}");
        assert!(
            scene
                .build_hud(&settings, (1440., 900.), 60.)
                .telemetry
                .scale
                .is_some()
        );
        scene.apply_event(UiEvent::SetFocus(Focus::Ossa), &mut settings);
        for _ in 0..22 {
            scene.update(0.1, &mut settings);
        }
        assert!((scene.orbit_distance_km - 8000.).abs() < 1e-6);
        assert!((scene.focus_center - scene.positions[2]).length() < 1e-5);
        settings.show_markers = false;
        settings.show_orbits = false;
        let hud = scene.build_hud(&settings, (1440., 900.), 60.);
        assert!(hud.markers.is_empty() && hud.arrows.is_empty());
        assert!(scene.build_frame(&settings).orbits.is_empty());
        settings.view = ViewMode::Surface;
        scene.set_view_instant(ViewMode::Surface);
        scene.reset_camera();
        scene.set_move_axis(0., 1.);
        for _ in 0..100 {
            scene.update(0.05, &mut settings);
        }
        assert!(scene.surface_pos[0].hypot(scene.surface_pos[1]) > 8.);
        let ground = terrain_field::height_at_m(scene.surface_pos);
        assert!((scene.eye_height_m - ground - 1.7).abs() < 0.15);
        scene.set_move_axis(0., 0.);
        for _ in 0..30 {
            scene.update(0.05, &mut settings);
        }
        assert!(scene.velocity.length() < 0.001);
        assert!(visibility::terrain_blocked(
            Vec3::new(80_f32.to_radians().sin(), 0., -80_f32.to_radians().cos()),
            [0.; 2],
            terrain_field::height_m(0., 0.) + 1.7,
            1e6
        ));
    }
    #[test]
    fn physical_occlusion_epochs() {
        let (mut scene, mut settings) = fixture();
        let mut eclipse = None;
        let mut ridge = None;
        for j in 0..2048 {
            let t = j as f64 * scene.system.period() / 2048.;
            scene.set_sim_time(t, &mut settings);
            let frame = scene.build_frame(&settings);
            let sun = frame.bodies[0].rel_pos_km.normalize();
            let elevation = sun.y.asin().to_degrees();
            if eclipse.is_none() && frame.sun_visibility < 0.02 {
                eclipse = Some((t, frame.sun_visibility, elevation));
            }
            if ridge.is_none()
                && elevation > 0.3
                && elevation < 4.
                && visibility::marker_state(&frame, BodyId::Kesh) == MarkerState::Occluded
            {
                ridge = Some((t, elevation, sun.x.atan2(-sun.z).to_degrees()));
            }
        }
        println!(
            "Physical eclipse epoch {:?}; terrain-occluded Kesh {:?}",
            eclipse, ridge
        );
        assert!(eclipse.is_some());
        assert!(ridge.is_some());
    }
    #[test]
    fn physical_opening() {
        let eye_height = terrain_field::height_m(0., 0.) + 1.7;
        let s = System::new(eye_height);
        let positions = s.positions(0.);
        let (e, u, z) = s.surface_basis(&positions, [0.; 2]);
        assert!((e.cross(u) - z).length() < 1e-12);
        let eye = positions[2] + u * (1900. + eye_height as f64 / 1000.);
        let local = |v: DVec3| DVec3::new(v.dot(e), v.dot(u), v.dot(z));
        let v = local(positions[1] - eye);
        let k = local(-eye);
        let el = |v: DVec3| v.normalize().y.asin().to_degrees();
        let az = |v: DVec3| v.x.atan2(-v.z).to_degrees();
        let elong = v.normalize().dot(k.normalize()).acos().to_degrees();
        let phase = 0.5 * (1. + (k - v).normalize().dot(-v.normalize()));
        let opening = s
            .ring_normal
            .dot((positions[1] - eye).normalize())
            .abs()
            .asin()
            .to_degrees();
        let (forward, right, up) = camera_basis(37., 17.5);
        let camera = CameraView {
            forward,
            right,
            up,
            fov_y_deg: 76.,
            aspect: 1.6,
            surface: true,
        };
        let pv = camera.project(v.as_vec3(), Vec2::new(1440., 900.));
        let pk = camera.project(k.as_vec3(), Vec2::new(1440., 900.));
        println!(
            "Opening: Vhal elevation={} az={} diameter={} Kesh elevation={} az={} elongation={} phase={} ring={} Vhal px {},{} Kesh px {},{}",
            el(v),
            az(v),
            2. * (60_000. / v.length()).asin().to_degrees(),
            el(k),
            az(k),
            elong,
            phase * 100.,
            opening,
            pv.x,
            pv.y,
            pk.x,
            pk.y
        );
        assert!((el(v) - 30.).abs() < 1.);
        assert!((el(k) - 4.6).abs() < 0.00001);
        assert!((az(k) - 86.5).abs() < 0.00001);
        assert!((84. ..91.).contains(&elong));
        assert!((0.44..0.52).contains(&phase));
        assert!((opening - 12.).abs() < 1e-7);
        assert!(pv.on_screen && pk.on_screen);
        assert!(!visibility::terrain_blocked(
            v.normalize().as_vec3(),
            [0.; 2],
            eye_height,
            v.length() as f32
        ));
        assert!(!visibility::terrain_blocked(
            k.normalize().as_vec3(),
            [0.; 2],
            eye_height,
            k.length() as f32
        ));
        // Ossa returns to the same sub-Vhal elevation over a complete orbit.
        for fraction in [0.1, 0.3, 0.75, 1., 14.7] {
            let positions = s.positions(s.period() * fraction);
            let (_, u, _) = s.surface_basis(&positions, [0.; 2]);
            let eye = positions[2] + u * (1900. + eye_height as f64 / 1000.);
            let angle = (positions[1] - eye).normalize().dot(u).asin().to_degrees();
            assert!((angle - el(v)).abs() < 1e-9);
        }
    }
}
