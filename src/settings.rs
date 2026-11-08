//! Shared settings and UI event vocabulary.
//!
//! Owned by the director. Subagents may read it, but must not change the
//! public names without a written request.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Surface,
    Orbit,
}

impl ViewMode {
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Surface => "SURFACE",
            ViewMode::Orbit => "ORBIT",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Low,
    Medium,
    High,
}

impl Quality {
    pub fn label(self) -> &'static str {
        match self {
            Quality::Low => "LOW",
            Quality::Medium => "MED",
            Quality::High => "HIGH",
        }
    }

    pub fn render_scale(self) -> f32 {
        match self {
            Quality::Low => 0.75,
            Quality::Medium => 0.9,
            Quality::High => 1.0,
        }
    }

    pub fn dpr_cap(self) -> f32 {
        match self {
            Quality::Low => 1.25,
            Quality::Medium => 1.5,
            Quality::High => 2.0,
        }
    }

    /// Terrain grid: angular segments x radial rings.
    pub fn terrain_grid(self) -> (u32, u32) {
        match self {
            Quality::Low => (192, 128),
            Quality::Medium => (288, 160),
            Quality::High => (384, 200),
        }
    }

    pub fn shadow_size(self) -> i32 {
        match self {
            Quality::Low => 1024,
            Quality::Medium => 1536,
            Quality::High => 2048,
        }
    }

    pub fn sky_octaves(self) -> i32 {
        match self {
            Quality::Low => 3,
            Quality::Medium => 4,
            Quality::High => 5,
        }
    }

    pub fn ring_steps(self) -> i32 {
        match self {
            Quality::Low => 2,
            Quality::Medium => 3,
            Quality::High => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Ossa,
    Vhal,
    Kesh,
    System,
}

impl Focus {
    pub fn label(self) -> &'static str {
        match self {
            Focus::Ossa => "OSSA",
            Focus::Vhal => "VHAL",
            Focus::Kesh => "KESH",
            Focus::System => "SYSTEM",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub view: ViewMode,
    pub paused: bool,
    /// Simulated seconds per real second.
    pub time_scale: f64,
    /// Simulation time in seconds since the epoch. Mutated by the scene clock.
    pub sim_time: f64,
    pub exposure: f32,
    pub haze: f32,
    pub ring_brightness: f32,
    pub vhal_light: f32,
    pub fov_deg: f32,
    pub quality: Quality,
    pub show_markers: bool,
    pub show_orbits: bool,
    pub show_ui: bool,
    pub focus: Focus,
    pub reduced_motion: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            view: ViewMode::Surface,
            paused: false,
            time_scale: 60.0,
            sim_time: 0.0,
            exposure: 1.0,
            haze: 1.0,
            ring_brightness: 1.0,
            vhal_light: 1.0,
            fov_deg: 70.0,
            quality: Quality::High,
            show_markers: true,
            show_orbits: true,
            show_ui: true,
            focus: Focus::Vhal,
            reduced_motion: false,
        }
    }
}

impl Settings {
    pub fn auto_quality() -> Quality {
        let Some(win) = web_sys::window() else {
            return Quality::Medium;
        };
        let nav = win.navigator();
        let ua = nav.user_agent().unwrap_or_default().to_lowercase();
        let mobile = ua.contains("android")
            || ua.contains("iphone")
            || ua.contains("ipad")
            || ua.contains("mobile");
        let cores = js_sys::Reflect::get(&nav, &"hardwareConcurrency".into())
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(8.0);
        if mobile || cores <= 4.0 {
            Quality::Low
        } else if cores <= 8.0 {
            Quality::Medium
        } else {
            Quality::High
        }
    }

    pub fn prefers_reduced_motion() -> bool {
        web_sys::window()
            .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok())
            .flatten()
            .map(|m| m.matches())
            .unwrap_or(false)
    }
}

/// Events emitted by the DOM UI. The main loop applies these to `Settings`
/// and/or forwards them to the scene.
#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    SetView(ViewMode),
    SetFocus(Focus),
    SetQuality(Quality),
    SetPaused(bool),
    SetTimeScale(f64),
    ScrubTime(f32),
    SetExposure(f32),
    SetHaze(f32),
    SetRings(f32),
    SetVhalLight(f32),
    SetFov(f32),
    ToggleMarkers,
    ToggleOrbits,
    ToggleUi,
    ResetCamera,
    Fullscreen,
    SetMoveAxis(f32, f32),
}
