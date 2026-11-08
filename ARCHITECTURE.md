# OSSA — Architecture & Interface Contract v1.0

Director-owned. Read `VISUAL_SPEC.md` first. Do not change public types or the
module ownership boundaries without reporting it as a blocker.

## Build / run

```bash
# from repo root
cargo check --target wasm32-unknown-unknown
trunk build --release
python -m http.server 8765 --directory dist
# local preview needs a relative public URL:
trunk build --release --public-url ./
```

Deployment is GitHub Pages via `.github/workflows/pages.yml`; do not edit the
workflow or push/commit. The director handles git and deployment.

## Module ownership

| Owner | Files |
|---|---|
| Director | `src/main.rs`, `src/frame.rs`, `src/settings.rs`, `src/terrain_field.rs`, `src/render/mod.rs`, `src/render/gl_util.rs`, `VISUAL_SPEC.md`, `ARCHITECTURE.md`, deployment |
| `renderer-sky` | `src/render/sky.rs`, `src/render/sky.glsl`, `src/render/orbit_lines.rs` |
| `renderer-terrain` | `src/render/terrain.rs`, `src/render/terrain.glsl` |
| `scene` | `src/scene/**` (replace `src/scene/mod.rs`) |
| `ui` | `src/ui/**`, `index.html`, `styles.css` |
| `qa` | `tools/qa/**`, `docs/qa-notes.md` |

Do not edit files outside your ownership. Do not run `git commit`, `git push`,
`git checkout`, `git reset`, or install global tools. You may add files in your
own directories.

## Coordinate & units contract

- Distances: kilometres (km), f64 on CPU; camera-relative f32 for renderer.
- Time: seconds.
- Surface local frame: +X east, +Y up, −Z north. Terrain horizontal plane.
- Camera-relative: camera is at `(0,0,0)`. Every `BodyView.rel_pos_km`,
  `RingView.center_rel_km` and `OrbitCurve.points_rel` is relative to the camera.
- Screen: CSS pixels, origin top-left. `CameraView::project` does this.
- Surface terrain field coordinates: `terrain_field::height_m(east_m, north_m)`
  in metres. Renderer converts north to `-Z` and subtracts
  `RenderFrame.eye_height_m` from y.

## Fixed shared types

`src/frame.rs` and `src/settings.rs` are authoritative. Key points:

- `BodyId` order: Kesh=0, Vhal=1, Ossa=2, Thren=3, Sable=4, Nadir=5, Cinder=6, Sur=7.
- `RenderFrame` carries `camera`, `bodies: [BodyView; 8]`, `ring`, `orbits`,
  `view`, `quality`, `exposure`, `haze`, `vhal_light`, `sim_time_s`,
  `sun_visibility`, `surface_pos_m`, `eye_height_m`.
- `HudFrame` carries ready-to-display marker/arrow positions + telemetry strings.
- UI settings live in `Settings`; UI emits `UiEvent`; the scene applies them.

## Scene API (must remain)

`src/scene/mod.rs` must expose:

```rust
impl Scene {
    pub fn new(canvas: &HtmlCanvasElement, settings: &Settings, viewport: (f64, f64)) -> Result<Self, JsValue>;
    pub fn set_viewport(&mut self, w: f64, h: f64);
    pub fn update(&mut self, dt: f64, settings: &mut Settings);
    pub fn apply_event(&mut self, event: UiEvent, settings: &mut Settings);
    pub fn set_view_instant(&mut self, view: ViewMode);
    pub fn set_focus_instant(&mut self, focus: Focus);
    pub fn set_sim_time(&mut self, t: f64, settings: &mut Settings);
    pub fn scrub(&mut self, fraction: f32, settings: &mut Settings);
    pub fn set_surface_look(&mut self, yaw_deg: f32, pitch_deg: f32);
    pub fn set_orbit_camera(&mut self, az_deg: f32, el_deg: f32, distance_km: f64);
    pub fn set_move_axis(&mut self, x: f32, y: f32);
    pub fn reset_camera(&mut self);
    pub fn build_frame(&self, settings: &Settings) -> RenderFrame;
    pub fn build_hud(&self, settings: &Settings, viewport: (f32, f32), fps: f32) -> HudFrame;
}
```

The scene installs its own canvas input listeners (pointer look, wheel, WASD)
on the canvas, not on UI elements. UI owns global keyboard shortcuts and touch
joystick and emits events.

### Scene responsibilities

- Keplerian multi-level system per spec §5, deterministic initial epoch that
  satisfies the opening composition. Positions in km, f64.
- Tidal lock of Ossa; surface local frame with +Y up / −Z north; Vhal at ~42°
  elevation, Kesh at ~5° elevation, ~58° separation, opening-shot framing.
- Surface walking controller (WASD/arrows, sprint, terrain follow, slope limit,
  mouse/touch look, pointer lock, camera weight, no fly).
- Orbit camera (drag orbit, wheel zoom, focus presets 6e3..4e7 km, 2.2s smooth
  focus transitions). No auto-drift.
- Occlusion + visibility:
  - Vhal/Kesh on-screen/off-screen projection with correct arrows for behind
    camera (use `CameraView::project`/`edge_arrow`).
  - Surface terrain occlusion via `terrain_field::height_m` ray march.
  - Orbit body occlusion via ray-sphere tests; Kesh blocked by Vhal = Eclipsed.
- Lighting data: `sun_visibility` for a solar eclipse seen from the surface
  (Vhal disc over Kesh), ring tilt, phase.
- `build_frame` fills all 8 bodies + ring + orbit curves. Orbit curves are
  camera-relative (sample with f64 then cast to f32); include planet orbits and
  moon orbits when `show_orbits` and in orbit view. Renderer culls line segments
  behind bodies.
- `build_hud` generates Vhal/Kesh markers or arrows and all telemetry strings.
  Distance strings via `frame::format_distance_km`.
- URL/debug setters are used by QA.

## Renderer API

`src/render/mod.rs` is director-owned and calls:

```rust
SkyPass::new(&Gpu) -> Result<SkyPass, String>
SkyPass::set_quality(&mut self, gpu: &Gpu, q: Quality)
SkyPass::render(&mut self, gpu: &Gpu, frame: &RenderFrame)

TerrainPass::new(&Gpu, q: Quality) -> Result<TerrainPass, String>
TerrainPass::set_quality(&mut self, gpu: &Gpu, q: Quality, width: i32, height: i32, render_scale: f32)
TerrainPass::render(&mut self, gpu: &Gpu, frame: &RenderFrame, width: i32, height: i32)

OrbitPass::new(&Gpu) -> Result<OrbitPass, String>
OrbitPass::set_quality(&mut self, gpu: &Gpu, q: Quality)
OrbitPass::render(&mut self, gpu: &Gpu, frame: &RenderFrame, width: i32, height: i32)
```

`gl_util::Gpu` owns `Rc<glow::Context>`; use `gl_util` helpers when convenient
(Program, DepthTarget, fullscreen VAO, buffer upload). Do not change `gl_util`
or `render/mod.rs`; if an API is missing, report it to the director.

Renderer state:
- Viewport is set by `Renderer::resize`; drawing buffer size = CSS size ×
  effective DPR × `quality.render_scale()`.
- The director clears color/depth. Sky draws first with no depth write; terrain
  draws next with depth; orbit lines only in orbit view.
- Use `frame.quality` for quality-dependent work. On quality change the pass
  receives `set_quality` before the next render.
- Log shader errors with line numbers; return `Err(String)` from `new`.

Sky pass requirements: analytic impostor renderer (ray-sphere per pixel) for
all bodies, rings, atmospheres, star, starfield, and orbital-view background.
No textures. See spec §6.1/§6.2. Must handle surface vs orbit (`frame.view`).
Orbit lines: project `OrbitCurve.points_rel`, draw 1px additive/alpha lines,
cull segments that pass behind bodies. No texture assets.

Terrain pass: CPU radial mesh from `terrain_field::height_m`, camera-following,
rebuild when `surface_pos_m` moves >32m or quality changes; sun shadow map;
procedural mineral shading + haze; use `frame.sun_visibility`, `frame.haze`,
`frame.exposure`, `frame.vhal_light`. See spec §6.3/§6.4.

## UI API

`src/ui/mod.rs` must expose:

```rust
pub struct Ui;
impl Ui {
    pub fn new(settings: &Settings) -> Result<Self, JsValue>;
    pub fn poll(&mut self) -> Vec<UiEvent>;
    pub fn reflect(&mut self, settings: &Settings);
    pub fn update_hud(&mut self, hud: &HudFrame);
    pub fn set_ready(&mut self);
    pub fn notify_error(&self, msg: &str);
}
```

UI owns `index.html` and `styles.css`; keep `<canvas id="scene">`, `#loader`,
`#fatal`, `#fatal-message`, and `#ui-root`. Build the UI inside `#ui-root` from
`Ui::new` or static HTML. Keep the bootstrap loading/fatal behavior. No external
fonts/assets/CDNs. Every control must emit `UiEvent` and be reflected from
`Settings`.

## QA hooks

URL parameters (implemented in `src/main.rs`):
`view=surface|orbit`, `focus=ossa|vhal|kesh|system`, `t=<seconds>`,
`time_scale=<f64>`, `paused=1`, `yaw=<deg>`, `pitch=<deg>`, `orbit_az=<deg>`,
`orbit_el=<deg>`, `distance=<km>`, `quality=low|med|high`, `ui=0`, `markers=0|1`,
`orbits=0|1`, `exposure=`, `haze=`, `rings=`, `vhal_light=`, `fov=`.

JS debug API on `window.__ossa`:
`snapshot()`, `set(obj)`, `setTime(t)`, `setCamera({yaw,pitch,orbitAz,orbitEl,distance})`,
`fps()`, `state()`, `hideUI(bool)`, `ready()`. `snapshot()` returns JSON string.

Screenshot workflow: build with `--public-url ./`, serve `dist`, open
`http://127.0.0.1:8765/?view=surface&t=...&ui=0`, wait 2 frames after
`__ossa.ready()`, screenshot at 1440×900. Use headless Chrome via Playwright
Core (`tools/qa`).

## Model / escalation policy

- Work within your assigned model. If a task needs a different model or an
  interface change, stop and report; do not silently change architecture.
- If the API returns quota/billing errors, stop that task and report; do not
  retry blindly. Rate-limit errors: wait and retry.
- No git operations. No global installs. No network assets.

## Report format

Return: files changed; what works; self-test results (`cargo check`, `trunk
build`, screenshots if applicable); known limitations; exact blockers; and
anything the director must decide. Keep it under 500 words.
