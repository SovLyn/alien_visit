# OSSA — Delivery Note

**Live site:** https://sovlyn.github.io/alien_visit/
**Repo:** https://github.com/SovLyn/alien_visit
**Built:** 2026-11-08

This note records what was built, how the work was divided, which spec changes were
made and why, and how the final result was verified.

---

## 1. What the viewer gets

A single self-contained page: **OSSA**, a desert moon beneath the gas giant **Vhal**,
orbiting the cold star **Kesh**. Two views:

- **Surface** — a 1.7 m eye at 60°N on Ossa, low-frequency eroded desert, a long
  hard shadow from the anchor ridge, Vhal half-lit overhead with rings, a small
  cold Kesh low on the right, stars, cold haze. Walk, look, run; the terrain follows.
- **Orbit** — an observational view of the Vhal local system (Ossa, Thren, Sable,
  orbit hairlines, labels, ring shadows), with **SYSTEM** and **KESH** presets for
  the star and the planets. Orbit lines are occluded by bodies.
- **HUD** — always-on Vhal/Kesh markers, 28px bracket for Kesh's bright disc,
  `OCCLUDED` / `ECLIPSED` states, edge chevrons for off-screen/behind-camera
  bodies, quiet minor-body labels in orbit, telemetry and a scale bar.
- **OPTICS deck** — time flow/pause/scrub, exposure, haze, rings, Vhal-shine,
  lens/FOV, quality, focus, orbit key, overlays. Every control was exercised.

Everything is procedural; there are no texture, font, analytics or CDN requests.

---

## 2. Technical shape

- Rust 2024 → `wasm32-unknown-unknown`, Trunk 0.21, `wasm-bindgen`, `glow` 0.16,
  `glam`, minimal `web-sys`.
- WebGL2 only; graceful `SIGNAL LOST` / WebGL-unavailable path.
- Scene: f64 hierarchical Kepler elements, Newton solve, tidal lock, 193-point
  cached orbit curves, camera-relative f32 render frames.
- Sky: analytic per-pixel ray–sphere/ray–plane impostor pass for 8 bodies, rings,
  gas-giant belts, terminator, planet/ring/moon shadows, starfield, atmosphere.
- Terrain: camera-following polar mesh (30 km radius), CPU height field, normal
  from neighbours, sun shadow map (PCF), Vhal-shine fill, haze.
- HUD/UI: DOM overlay with element pooling and a deck-aware collision helper;
  no external fonts.
- Deploy: GitHub Actions → Pages artifact; Pages `build_type: workflow`.

The camera-relative design avoids f32 solar-system precision issues; the surface
local frame is +Y up / −Z grid north. Expedition grid north points toward Vhal,
which at 60°N is geodetic south. That is a **compass convention**, documented in
`src/scene/ephemeris.rs`, not a mirror of the celestial geometry.

---

## 3. Work division and models

The director defined the visual target and interface contracts, then delegated
the implementation slices in parallel. Main model: `gpt-6.1-sol`.

| Task | Subagent / model | Result |
|---|---|---|
| P0 toolchain probe | `gpt-6.1-terra` | Trunk + glow 0.16 + WASM + headless Chrome WebGL2 verified |
| Sky / celestial impostors | `renderer-sky`, `gpt-6.1-sol` | `sky.glsl` (bodies, rings, shadows, starfield, atmosphere), orbit-line pass; later polish pass by the same model |
| Terrain / shadows | `renderer-terrain`, `gpt-6.1-terra` | camera-following mesh, shadow map, mineral shading, haze |
| Scene / cameras / HUD | `scene`, `gpt-6.1-sol` | ephemerides, tidal lock, surface/orbit cameras, input, occlusion, HUD data, tests |
| UI / HUD DOM | `ui`, `gpt-6.1-terra` | index.html, CSS, control bindings, marker pool, mobile layout |
| QA harness | `qa`, `gpt-6-luna` | Playwright shot/perf/smoke tools; director finished the acceptance suite |
| Visual spec / integration / deploy | director (`gpt-6.1-sol`) | contracts, scaffold, renderer orchestration, integration, visual acceptance, Pages |

A first scene attempt stopped after discovering that the v1.0 opening shot was
physically impossible (low sun in frame + 76% lit giant). That finding drove spec
v1.1; a second scene agent delivered the working solution. The sky, UI and QA
agents were cleaned up mid-stream by the harness; the director completed their
remaining polish and tests.

### Code share

- Subagents produced the bulk of the rendering, simulation, UI and QA code:
  sky shaders (~760 lines), terrain pass/shaders (~220), scene (~1 880), UI
  (~900 including HTML/CSS), QA tools.
- Director-owned glue and contracts: `main.rs`, `frame.rs`, `settings.rs`,
  `terrain_field.rs`, `render/mod.rs`, `render/gl_util.rs`, plus integration
  edits, the visual spec, deployment and acceptance. This is roughly one third
  of the core source and is the reason it is called out here.
- The director also did the final art-direction passes on the terrain field
  (blade/shadow composition, warming), the UI layout (deck to bottom-left, Kesh
  bracket, orbit key, minor labels), orbit-preset tuning, and the QA fix loop.

---

## 4. Spec changes (recorded, not silent drift)

`VISUAL_SPEC.md` is the authority. The final version is **1.2**.

1. **v1.0 → v1.1 — opening phase.**
   Illuminated fraction is `(1 − cos elongation)/2`. A low Kesh near a +42° Vhal
   forces a crescent, not 76% lit. The shot was re-solved with Vhal at +30°,
   Kesh +5°, 85° azimuth separation, FOV 76°, start lat 60°N, Vhal spin tilt 50°.
   Achieved: Vhal +29.62°, Kesh +4.60°, Kesh azimuth 86.5°, elongation 84.69°,
   phase **47.06%**, ring opening **12.0°**, ring screen PA −24.24° (24.2° abs),
   Vhal angular diameter **27.88°**, Kesh **2.177°**; both bodies on screen at
   1440×900 (Vhal 348,263; Kesh 1400,556). Within the v1.1 tolerances.

2. **v1.1 → v1.2 — orbit framing and UI.**
   The initial orbit default (850k/28°) was sparse and backlit; a 420k camera made
   a full-phase white egg. The final default is **420 000 km / 18° elevation /
   ~50% phase** (50.5% in the shot), with Ossa/Thren/Sable labels and the star
   deliberately off-screen with an edge arrow. KESH is **12M km**, SYSTEM
   **18M km**; true-scale planets are tiny, so labels + ORBIT KEY carry the
   hierarchy. The OPTICS deck moved from bottom-right to bottom-left because it
   was covering Kesh at the right edge; the view rail moved to the top-left, the
   hint to the bottom-right, and Kesh's reticle became a 28px bracket around its
   bright disc. Minor-body labels were added to the orbit view.

3. **Terrain composition note.** The opening anchor is a narrow eroded blade
   left of centre (280–650 m, 8–10° silhouette) with a low east shelf under Kesh,
   rather than a tall ridge on the far right. The east ridge still exists for
   exploration; this keeps Kesh visible and gives the blade the long hard shadow
   that frames the lower-left. It is a composition choice, not a physics change.

---

## 5. Acceptance

### Deployment / robustness

- [x] GitHub Pages is configured with `build_type: legacy`, source branch
  `gh-pages`; the static build is at the branch root with relative asset paths
  and `.nojekyll`.
- [x] Real URL `https://sovlyn.github.io/alien_visit/` returns HTTPS 200; hashed
  JS/WASM/CSS assets load with valid SRI (CSS line endings normalized to LF), and
  `DEPLOY.txt`-free clean branch.
- [x] The full **28/28 live acceptance suite** passes against the real URL with
  zero console/page errors. The local suite passes identically.
- [x] `.github/workflows/pages.yml` contains the Trunk build (including a
  crates.io-installed, version-matched wasm-bindgen CLI) but is disabled because
  GitHub-hosted runners received repeated `504` from the wasm-bindgen GitHub
  Release download; the delivery build is the same source built locally.
- [x] No blocking console errors; favicon is a data URI; preload-integrity is a
  warning only. Tested with `tools/qa/acceptance.mjs` against live and local.
- [x] WebGL2 failure path shows `SIGNAL LOST`; loading veil hides on boot.
- [x] 1440×900, 1280×720, 390×844 and 844×390 layouts checked. Short heights
  (≤780 px) start with the deck folded; mobile user agent auto-selects LOW; deck
  collapsed; joystick present and moves.

### UI / controls

- [x] **28/28** local acceptance checks pass, console clean:
  sliders (exposure, haze, rings, Vhal light, FOV), quality, time scale, pause,
  scrub, markers/orbits toggles, view rail, focus presets, keyboard 1/2/H,
  deck collapse, drag-look, debug API, mobile auto-LOW, joystick.
- [x] Every control has a visible effect; default values reflect the render
  (1.00 exposure/haze/rings/Vhal light, 76° FOV, 60× time).
- [x] HUD labels never overlap the deck: the scene reads the live deck rect and
  flips labels at the left edge.

### Markers / arrows / occlusion

- [x] Vhal and Kesh on-screen markers; Kesh 28px bracket frames the disc.
- [x] Off-screen: `arrows.png` (both bodies behind camera) shows chevrons and
  distances correctly.
- [x] Terrain occlusion: `occluded.png` — Kesh behind the east shelf with
  `KESH · 7.34M KM · OCCLUDED`, and Vhal's off-screen arrow.
- [x] Eclipse: `eclipse.png` — t=64 287.491 s, Vhal silhouette, Kesh marker
  `ECLIPSED`, clock DAY 0001 · 17:51:27.
- [x] Orbit view minor labels (Ossa, Thren, Sable; Sur/Nadir in SYSTEM) and
  ORBIT KEY.

### Kinematics / light / reflection

- [x] Hierarchical Kepler elements and Newton solve; derived periods:
  Ossa 20.431530 h, Thren 10.657698 h, Sable 46.894809 h, Cinder 3.072965 h,
  Vhal 122.055335 h, Sur 24.912935 h, Nadir 202.184948 h.
- [x] Ossa tidally locked; Vhal spin 16 h; ring plane 50° from Ossa's orbit.
- [x] Ring shadow on Vhal, planet shadow on rings, moon transits/shadows and
  eclipses are geometric in the sky shader; `sun_visibility` drives surface
  direct-light dimming during a solar eclipse.
- [x] Vhal-shine fill, hard terrain shadows, haze and cold palette per spec.
- [x] Orbit lines are culled when passing behind a body.

### Visual match

- [x] Opening: Vhal half-lit + bands, 12° rings, Kesh low right, blade + long
  shadow, Vhal-shine fill, starfield, cold horizon.
- [x] Orbit: half-lit Vhal with rings and labels; SYSTEM/KESH hierarchy; scale
  bar; ORBIT KEY.
- [x] `surface.png`, `surface-hud.png`, `orbit.png`, `orbit-hud.png`,
  `system.png`, `occluded.png`, `eclipse.png`, `arrows.png`, `mobile.png`.

### Performance

- [x] `PERF.md` records desktop/mobile, software and hardware-headless numbers,
  quality auto-downgrade, page weight and the terrain rebuild hitch.
  Headline: software 1440×900 HIGH ≈10 FPS; mobile LOW 4× CPU throttle ≈16 FPS
  average; hardware-headless at the ~240 Hz cap (≈4.2 ms/frame or better).
  These are labelled appropriately; no phone GPU was available.

---

## 6. Known limitations / honest notes

- **Mobile performance** is measured with a mobile viewport/UA on desktop silicon
  and with SwiftShader; it is not a phone thermal benchmark. The page auto-selects
  LOW on mobile and the joystick/mobile deck were verified.
- **Headless hardware** numbers hit the ~240 Hz rAF cap and cannot distinguish
  HIGH/LOW; treat them as headroom evidence, not a certified 240 fps claim.
- **Terrain rebuild** is ≈58 ms at HIGH every 96 m of movement; it is the largest
  frame spike. Normal walking rebuilds roughly every 45 s; standing/orbit is
  unaffected. A chunked async rebuild would be the next optimization.
- **System view** shows true-scale planets; they are sub-pixel/small. Labels,
  orbit lines and the ORBIT KEY carry the hierarchy rather than exaggerated
  planets. The OSSA focus shows the moon as a sphere; its texture is a procedural
  approximation of the ground palette, not the exact terrain heightfield.
- **Default orbit** leaves Kesh off-screen by design; the KESH/SYSTEM presets and
  the edge arrow cover it. This was a deliberate composition trade-off (v1.2).
- **Compass**: expedition grid north points toward Vhal; at 60°N this is geodetic
  south. Documented in code; the celestial geometry is not mirrored.
- **Time/clock** is an artistic simulated calendar (ONE DAY = Ossa's orbital
  period), not a real ephemeris date.
- **No audio, no external textures, no third-party fonts.** The look is procedural
  by design; the mission allowed textures but did not require them.
- The scene test helpers (`src/scene/test_native.py`, `native_tests.rs`,
  `qa.mjs`) are development-only and are not shipped in `dist/`.

---

## 7. Evidence index

| File | Content |
|---|---|
| `docs/shots/surface.png` | opening shot, UI hidden, 1440×900 |
| `docs/shots/surface-hud.png` | opening + HUD/markers/deck |
| `docs/shots/orbit.png` | default Vhal-local orbit, UI hidden |
| `docs/shots/orbit-hud.png` | orbit view + labels + ORBIT KEY + scale |
| `docs/shots/system.png` | SYSTEM focus, Kesh + planets + labels |
| `docs/shots/occluded.png` | Kesh terrain-occluded, Vhal arrow |
| `docs/shots/eclipse.png` | Kesh eclipsed by Vhal |
| `docs/shots/arrows.png` | both bodies behind camera, edge chevrons |
| `docs/shots/mobile.png` | 390×844 portrait, joystick, auto-LOW, Kesh arrow |
| `PERF.md` | methodology + all measured numbers |
| `VISUAL_SPEC.md` | visual authority, v1.2 changelog |
| `ARCHITECTURE.md` | ownership + interfaces |
| `tools/qa/` | shot/perf/smoke/acceptance harness |

The live page was verified with the same Playwright harness against the final
`https://sovlyn.github.io/alien_visit/` URL after deployment: 28/28 checks, no
console/page errors, and the live surface/orbit views match the committed
screenshots.
