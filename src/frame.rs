//! Render/HUD frame contract between scene, renderer and UI.
//! Owned by the director. Coordinates are camera-relative kilometres so the
//! web shader never sees absolute solar-system positions in f32.

use crate::settings::{Focus, Quality, ViewMode};
use glam::{Vec2, Vec3};

pub const MAX_BODIES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum BodyId {
    Kesh = 0,
    Vhal = 1,
    Ossa = 2,
    Thren = 3,
    Sable = 4,
    Nadir = 5,
    Cinder = 6,
    Sur = 7,
}

impl BodyId {
    pub const ALL: [BodyId; MAX_BODIES] = [
        BodyId::Kesh,
        BodyId::Vhal,
        BodyId::Ossa,
        BodyId::Thren,
        BodyId::Sable,
        BodyId::Nadir,
        BodyId::Cinder,
        BodyId::Sur,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn name(self) -> &'static str {
        match self {
            BodyId::Kesh => "KESH",
            BodyId::Vhal => "VHAL",
            BodyId::Ossa => "OSSA",
            BodyId::Thren => "THREN",
            BodyId::Sable => "SABLE",
            BodyId::Nadir => "NADIR",
            BodyId::Cinder => "CINDER",
            BodyId::Sur => "SUR",
        }
    }

    pub const fn is_star(self) -> bool {
        matches!(self, BodyId::Kesh)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    Star,
    GasGiant,
    Desert,
    Rock,
    Ice,
    Dark,
}

#[derive(Clone, Copy, Debug)]
pub struct BodyView {
    pub id: BodyId,
    /// Position relative to the camera, kilometres. f32 is safe at this range.
    pub rel_pos_km: Vec3,
    pub radius_km: f32,
    pub kind: BodyKind,
    /// World-space spin axis, unit length.
    pub axis: Vec3,
    /// Rotation angle about `axis`, radians.
    pub spin_rad: f32,
    /// Linear RGB tint for minor bodies (0..1).
    pub tint: Vec3,
    /// False for bodies hidden in this view (e.g. Ossa when standing on it).
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct RingView {
    pub center_rel_km: Vec3,
    pub normal: Vec3,
    pub inner_km: f32,
    pub outer_km: f32,
    /// 0..1.5, multiplied by the scene's material response.
    pub brightness: f32,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct CameraView {
    /// Camera-relative origin is always (0,0,0); these are world-space basis
    /// vectors of the render frame.
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub fov_y_deg: f32,
    pub aspect: f32,
    pub surface: bool,
}

impl CameraView {
    pub fn tan_half_fov_y(&self) -> f32 {
        (self.fov_y_deg.to_radians() * 0.5).tan().max(1e-4)
    }

    pub fn tan_half_fov_x(&self) -> f32 {
        self.tan_half_fov_y() * self.aspect.max(1e-3)
    }

    /// Perspective projection of a camera-relative point into CSS pixels.
    pub fn project(&self, rel: Vec3, viewport: Vec2) -> Projected {
        let z = rel.dot(self.forward);
        if z <= 1e-4 {
            return Projected {
                x: 0.0,
                y: 0.0,
                ndc: Vec2::ZERO,
                depth_km: z,
                in_front: false,
                on_screen: false,
            };
        }
        let t = self.tan_half_fov_y();
        let ndc_x = rel.dot(self.right) / (z * t * self.aspect.max(1e-3));
        let ndc_y = rel.dot(self.up) / (z * t);
        let x = (ndc_x * 0.5 + 0.5) * viewport.x;
        let y = (0.5 - ndc_y * 0.5) * viewport.y;
        Projected {
            x,
            y,
            ndc: Vec2::new(ndc_x, ndc_y),
            depth_km: z,
            in_front: true,
            on_screen: (-1.0..=1.0).contains(&ndc_x) && (-1.0..=1.0).contains(&ndc_y),
        }
    }

    /// Directional edge-arrow anchor for a body that is outside the frame
    /// (or behind the camera). Returns CSS position and screen-space angle
    /// (0 = +X, clockwise, CSS `rotate` convention).
    pub fn edge_arrow(&self, rel: Vec3, viewport: Vec2, inset_px: f32) -> EdgeArrow {
        let sx = rel.dot(self.right);
        let sy = rel.dot(self.up);
        let (mut dx, mut dy) = if sx.abs() + sy.abs() > 1e-6 {
            (sx, -sy) // CSS y grows downward
        } else {
            // Directly behind: choose "up" deterministically.
            (0.0, -1.0)
        };
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        dx /= len;
        dy /= len;

        let w = viewport.x.max(1.0);
        let h = viewport.y.max(1.0);
        let inset = inset_px.min(w * 0.45).min(h * 0.45);
        let half_w = w * 0.5 - inset;
        let half_h = h * 0.5 - inset;
        let tx = if dx.abs() > 1e-6 {
            half_w / dx.abs()
        } else {
            f32::INFINITY
        };
        let ty = if dy.abs() > 1e-6 {
            half_h / dy.abs()
        } else {
            f32::INFINITY
        };
        let t = tx.min(ty);
        let x = w * 0.5 + dx * t;
        let y = h * 0.5 + dy * t;
        EdgeArrow {
            x,
            y,
            angle_deg: dy.atan2(dx).to_degrees(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Projected {
    pub x: f32,
    pub y: f32,
    pub ndc: Vec2,
    pub depth_km: f32,
    pub in_front: bool,
    pub on_screen: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct EdgeArrow {
    pub x: f32,
    pub y: f32,
    pub angle_deg: f32,
}

#[derive(Clone, Debug)]
pub struct OrbitCurve {
    /// Camera-relative sampled points (km).
    pub points_rel: Vec<Vec3>,
    /// Linear RGB.
    pub color: Vec3,
    pub alpha: f32,
    pub width_px: f32,
}

#[derive(Clone, Debug)]
pub struct RenderFrame {
    pub camera: CameraView,
    pub bodies: [BodyView; MAX_BODIES],
    pub ring: RingView,
    pub orbits: Vec<OrbitCurve>,
    pub view: ViewMode,
    pub quality: Quality,
    pub exposure: f32,
    pub haze: f32,
    pub vhal_light: f32,
    pub sim_time_s: f64,
    /// 0..1, direct sunlight remaining after Vhal's disc covers Kesh.
    pub sun_visibility: f32,
    /// Player position in the surface-local plane (metres, east/north).
    pub surface_pos_m: [f64; 2],
    /// Absolute eye height in the surface-local height field (metres).
    pub eye_height_m: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerState {
    Visible,
    Occluded,
    Eclipsed,
}

impl MarkerState {
    pub fn is_warning(self) -> bool {
        !matches!(self, MarkerState::Visible)
    }

    pub fn detail(self) -> &'static str {
        match self {
            MarkerState::Visible => "",
            MarkerState::Occluded => "OCCLUDED",
            MarkerState::Eclipsed => "ECLIPSED",
        }
    }
}

#[derive(Clone, Debug)]
pub struct HudMarker {
    pub body: BodyId,
    pub label: &'static str,
    /// Distance/occlusion detail, ready for display. e.g. "250 000 KM · OCCLUDED".
    pub detail: String,
    /// CSS pixels.
    pub x: f32,
    pub y: f32,
    pub state: MarkerState,
}

#[derive(Clone, Debug)]
pub struct HudArrow {
    pub body: BodyId,
    pub label: &'static str,
    /// Edge anchor in CSS pixels.
    pub x: f32,
    pub y: f32,
    /// CSS rotate angle, degrees.
    pub angle_deg: f32,
    pub detail: String,
    pub state: MarkerState,
}

#[derive(Clone, Debug)]
pub struct Telemetry {
    pub mode_line: String,
    pub location_line: String,
    pub vhal_phase: String,
    pub kesh_elevation: String,
    pub ring_tilt: String,
    pub clock: String,
    pub perf: String,
    pub scale: Option<String>,
    pub focus: Focus,
    pub view: ViewMode,
    pub show_markers: bool,
    pub show_orbits: bool,
}

#[derive(Clone, Debug)]
pub struct HudFrame {
    pub markers: Vec<HudMarker>,
    pub arrows: Vec<HudArrow>,
    pub telemetry: Telemetry,
}

/// Small helpers shared by scene/HUD.
pub fn format_thousands_int(v: f64) -> String {
    let n = v.abs().round() as u64;
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let bytes = s.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(*b as char);
    }
    if v < 0.0 { format!("-{out}") } else { out }
}

pub fn format_distance_km(km: f32) -> String {
    if km >= 1_000_000.0 {
        format!("{:.2}M KM", km as f64 / 1_000_000.0)
    } else if km >= 10_000.0 {
        format!("{} KM", format_thousands_int(km as f64))
    } else if km >= 1000.0 {
        format!("{:.0} KM", km)
    } else if km >= 1.0 {
        format!("{:.1} KM", km)
    } else {
        format!("{:.0} M", (km as f64) * 1000.0)
    }
}
