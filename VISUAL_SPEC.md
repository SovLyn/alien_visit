# OSSA — Scene Visual Spec v1.0

**Owner:** Director. This document is the single source of visual truth. Subagents may not edit it; they may propose changes with evidence.
**One-line anchor:** make the viewer forget, for a moment, that this is a WASM render, and believe they are standing on a planet no human should stand on.

---

## 1. Premise

**OSSA** is a tidally locked desert moon of the gas giant **VHAL**, which orbits the cold blue-white star **KESH**.

The player is alone on the moon's high-latitude desert. Vhal hangs almost motionless in the sky, too large to take in at once; its rings cut the sky as a thin, hard arc. Kesh is a small, cold, low sun. There is no lushness, no warmth, no spectacle for its own sake. The scene is a study of scale, silence, and exposure.

Names/copy:

| Thing | Name | Notes |
|---|---|---|
| Moon (playable surface) | OSSA | title wordmark |
| Gas giant | VHAL | fixed in Ossa's sky due to tidal lock |
| Star | KESH | cold white-blue, low over the horizon at the opening shot |
| Second planet | NADIR | outer, with moon CINDER |
| Inner planet | SUR | small, close to Kesh |
| Other moons of Vhal | THREN, SABLE | small, fast, visible in sky and orbit view |

Title lockup: `OSSA` — `MOON OF VHAL`. All UI copy in English; a single Chinese line may appear in the loading screen (optional): `巨影之下`.

---

## 2. Visual grammar

### 2.1 Palette (sRGB reference)

| Role | Hex | Notes |
|---|---|---|
| Sky zenith | `#05080d` | near-black, faint cold blue |
| Sky horizon | `#26363d` | desaturated slate-teal dust haze |
| Haze highlight (Kesh side) | `#8fa4ac` | cold, not golden |
| Sunlight | `#d6e6ff` | blue-white; hard shadows |
| Night fill (Vhal-shine) | `#3d5b64` | cold reflected light, visibly alien |
| Terrain dust | `#7a6f60` | faded mineral |
| Terrain regolith | `#675d51` | mid brown-gray |
| Terrain pale mineral | `#91846f` | salt/quartz flats |
| Terrain basalt | `#45443f` | shadowed rock |
| Terrain slate | `#555c5c` | cool strata |
| Vhal zone (light cloud) | `#d3c9b2` | cream, low saturation |
| Vhal belt (dark cloud) | `#7d6f60` | taupe/rust-gray |
| Vhal pale polar | `#b9c3c0` | greenish gray |
| Rings | `#c9cec8` lit / `#4b5453` shadow | icy gray, no gold |
| UI text | `#c9d2d0` | 80% opacity default |
| UI dim | `#7e8c8b` | labels, secondary |
| UI accent | `#9fc3c4` | cold cyan; active states |
| UI occlusion | `#c8a978` | single warm accent, only for occlusion warnings |
| UI warning | `#c8a978` | same |

Rules:
- No saturated orange/red Mars palette. No warm yellow sunlight. No teal-and-orange grade.
- Contrast comes from luminance, not saturation.
- UI is monochrome except for the occlusion accent.
- No bloom filters; all glow is analytic and restrained.

### 2.2 Typography

- UI: `ui-monospace, "SFMono-Regular", "Cascadia Mono", Consolas, monospace`, weights 400/500 only.
- Sizes: 8.5–10px labels, 11px values, 18px wordmark with 0.34em tracking.
- Uppercase, generous letter-spacing (0.14–0.34em), tabular numerals.
- Hairlines: 1px at 14–18% white.
- No rounded pills except 2px-radius toggles; no shadows except a soft panel backdrop blur.

### 2.3 Motion

- Restrained, physical, slow. UI transitions 160–220ms `cubic-bezier(.2,.7,.2,1)`.
- Camera has weight: surface movement accelerates smoothly, no instant stops; eye height follows terrain with a short damped lag.
- No camera shake, no lens flare, no twinkle, no gratuitous auto-orbit.
- View switching is a soft "exposure blink" (see §7.3), never a sci-fi wipe.

---

## 3. Opening shot (surface, desktop)

The default state must already be a composed photograph, not a debug view.

Camera at `lat 48.0°N` on Ossa, eye height 1.7 m, standing on a gentle rise at the edge of a shallow basin.

| Element | Target |
|---|---|
| Vertical FOV | 70° default (UI 55–95°) |
| Camera pitch | +20° (horizon visible in lower fifth; ground fills ~20–25% of frame) |
| Camera yaw | 18° east of the Vhal meridian |
| Vhal center | elevation +42°, azimuth −18° from view center (left of center) |
| Vhal angular diameter | 27.7° (radius 60 000 km at 250 000 km; ±1°) |
| Vhal illuminated fraction | 72–80% (phase angle 52–66°); terminator on the lower-left limb |
| Ring opening angle | 10–14° (thin, sharp); ring plane strikes a 18–28° screen angle |
| Ring outer angular radius | ~34° (rings extend past both sides of Vhal, clipping the frame edges) |
| Kesh center | elevation +5° ± 2°, azimuth +55° ± 8° east of the Vhal meridian (right of center) |
| Kesh angular diameter | 2.1° ± 0.2° |
| Kesh separation from Vhal | 50–62° on screen |
| Foreground | dark eroded ridge at right edge, silhouette 1–3° above horizon, ~250–450 m away; a low ridge/spire anchor at left third, 1.2–2.0 km away |
| Vhal top | may clip at the frame top by 0–3°; never clip the whole upper half |
| Terrain horizon | visible as a clean, slightly hazed line; no mesh edge, no void |

Composition notes:
- Vhal is left of center and above; Kesh is low right. Their separation is the picture's diagonal.
- The ring arc should cross in front of Vhal and continue into the sky as a hairline. It is the sharpest line in the image.
- The foreground ridge must not block Vhal. It frames the right side and gives the low camera a sense of weight.
- Ground in shadow stays legible: cold Vhal-shine fill, never crushed black.
- The default scene is **not** in eclipse; the star is visible.

Mobile (portrait, 390×844):
- Same world state, FOV_y 74°, pitch +17°, yaw 10° east of the Vhal meridian.
- Vhal dominates ~70% of screen width. Kesh may be off-screen: that is acceptable and must exercise the edge-arrow system.
- Telemetry condenses to two rows; control panel starts collapsed.

### 3.1 Surface lighting checkpoints

1. Sunlight is directional, cold blue-white; shadows are hard with a 1–2 px penumbra.
2. Shadow direction follows Kesh; at the opening shot shadows run from right to left, long and low.
3. Terrain on the night side is lit only by sky + Vhal-shine (`#3d5b64`-family), still showing form.
4. Vhal's lit side is the brightest large surface; it must not blow out to flat white. Cloud bands remain visible in highlights (ACES shoulder).
5. The star disc is small, hard-edged, with a thin cool corona; no giant romantic sun.
6. Dust haze near the horizon is subtle; it reduces contrast with distance, does not fog the whole frame.

---

## 4. Orbit view (default)

A cold, observational view of the Vhal system. Not a diagram, not a screensaver.

- Focus: Vhal. Camera distance 850 000 km (±10%), elevation 28° above the ring plane, azimuth such that the terminator and ring shadows are both readable.
- Vhal angular radius ~4.0°; ring outer radius ~9.5°; rings clearly open (not edge-on) for legibility.
- Ossa is visible as a small desert crescent (angular radius 0.13°) on its orbit, lower-left of Vhal; Thren and Sable are small lit dots on their orbit lines.
- Kesh is a small, intense 2° disc at upper right, 7.5 million km away.
- Orbit lines: 1px hairlines at 22–40% opacity, accent-tinted, with labels at the body. Moons' orbits and planets' orbits are both legible; hierarchical nesting must be obvious.
- No terrain in orbit view. No lens flare. No motion blur. Starfield static.
- The ring plane is the stage; Vhal's shadow falls across the rings; the rings' shadow falls across the cloud tops; moon shadows may transit.
- Scale bar (bottom center): hairline + text, e.g. `100 000 KM`, computed from camera and viewport.
- Camera interaction: drag to orbit, wheel to zoom, no auto-drift. Focus presets **OSSA / VHAL / KESH / SYSTEM** transition over 2.2s ease-in-out.
- Zoom range: 6 000 km (close to Ossa) to 4.0e7 km (system). Camera is clamped outside the focused body.

### 4.1 Orbit-specific checkpoints

1. Vhal's rings cast a banded shadow on the planet; the shadow edge is soft by ~0.2° but clearly banded.
2. Vhal casts a hard shadow on the rings, with the planet's atmospheric refraction rim faintly visible.
3. Moons cast small shadows on Vhal when aligned; moons are eclipsed by Vhal's shadow cone.
4. The moon system around Vhal and the planets around Kesh are both visible via labels + orbit lines; the hierarchy reads without explanation.
5. Orbit line occlusion: lines passing behind a body are not drawn over it.
6. The surface detail on Ossa reads as the same desert seen from the ground, at a smaller scale.

---

## 5. Celestial system (authoritative parameters)

Units: kilometres, seconds. World up for the surface frame is +Y; east +X; north −Z. Positions are computed with Keplerian elements (eccentricity, inclination, node, mean anomaly) and a Kepler solver. All periods derive from the listed semi-major axes and GM values.

| Body | Radius (km) | Orbit center | a (km) | e | Inclination | Period |
|---|---|---|---|---|---|---|
| Kesh | 140 000 | — | — | — | — | — |
| Sur | 3 200 | Kesh | 2.60e6 | 0.08 | −1.4° | 1.04 d |
| Vhal | 60 000 | Kesh | 7.50e6 | 0.02 | +1.8° | 5.09 d |
| Nadir | 7 500 | Kesh | 1.05e7 | 0.05 | +2.6° | 8.42 d |
| Thren | 620 | Vhal | 1.62e5 | 0.01 | +2.5° | 10.66 h |
| Ossa | 1 900 | Vhal | 2.50e5 | 0.00 | +0.5° | 20.43 h |
| Sable | 1 050 | Vhal | 4.35e5 | 0.03 | −3.0° | 46.89 h |
| Cinder | 280 | Nadir | 6.0e4 | 0.01 | +4.0° | 3.07 h |

- `GM_kesh = 0.65 GM_sun`, `GM_vhal = 0.90 GM_jup`, `GM_nadir = 0.55 GM_jup`.
- Vhal's equatorial plane is tilted 45° from Ossa's orbital plane (a captured, high-inclination moon). Rings at 1.35–2.35 R_vhal (81 000–141 000 km), with the sharp outer edge at 2.35 R.
- Ossa is tidally locked to Vhal: its spin period equals its orbital period; the Vhal meridian stays fixed. Surface rotation is handled by the tidal-lock frame, not a free spin.
- Vhal spins slowly: period 16 h (visible cloud motion at 600× time, not distracting at 60×).
- All other moons are tidally locked for visual stability.
- Initial epoch must be solved so the opening-shot targets in §3 hold within tolerance. The solution must be deterministic and documented in code.
- Eclipses and transits emerge from the geometry; do not fake them. Do not place bodies at impossible positions for art.

### 5.1 Information semantics

- `VHAL PHASE` = illuminated fraction from Ossa's surface (or from the orbit camera when in orbit view), 0–100%.
- `KESH EL` = Kesh's elevation above the local horizon in surface view; use `--` in orbit view.
- `RING TILT` = angle between the camera's line of sight to Vhal and the ring plane, in degrees (0 = edge-on).
- `SIM` clock: `DAY 0001 · 06:42:10`; day length = Ossa's orbital period around Vhal (20.43 h). Time scale default 60×.

---

## 6. Rendering requirements

All geometry shading is analytic/procedural; do not depend on external image assets. The page must be self-contained and work offline after load.

### 6.1 Vhal (character, not a textured ball)

- Cloud belts: 3 broad zones + ~11 finer belts, latitude-aligned, domain-warped with 3D fbm; longitudinal stretching; no repeating tiling.
- Palette per §2.1; low saturation; subtle ochre/rust belts, pale cream zones, pale polar hood.
- Terminator: a wide, layered transition (2–3° of angular softness) with a visible belt structure crossing it; nightside is not black — a very dim cold ring-shine/atmosphere.
- Limb: 1–2 px atmosphere rim, cold cyan-white on the lit side; no neon halo.
- One subtle storm oval in the southern belts, low contrast (optional; if present, keep it non-Earth-like).
- Rotation: slow enough to perceive at default speed only if watched; cloud bands drift at slightly different rates (differential rotation) with no obvious seams.
- Ring shadow on the cloud tops: banded, offset correctly for the star direction. Planet shadow on rings: dark, with a soft edge.
- Vhal's rings: 1D procedural density with 2–3 named gaps (visual, not labeled), sharp inner/outer edges, forward-scattering brightness; anti-aliased with `fwidth`; brightness slider 0–1.5.
- Anti-aliasing of the disc edge and rings is mandatory; no stair-stepped limb.

### 6.2 Ossa's sky

- Atmosphere: cold gradient, horizon haze, forward-scatter glow around Kesh; haze slider scales density and horizon band height.
- Starfield: sparse, ~500 stars, magnitudes 0.1–1.0, cool white to faint blue; a very faint galactic band is allowed, not required. No twinkle.
- Kesh: hard white-blue disc, limb darkening; thin corona. Visible through haze (dimmed, not blurred to mush).
- Other moons/planets: correct angular size and phase; Thren and Sable are visible as small discs when above the horizon; Nadir/Sur are tiny. Do not add labels to all of them.
- Terrain occludes sky bodies correctly (the sky shader is drawn first, terrain overwrites; HUD occlusion uses the same height function).

### 6.3 Terrain

- Low-frequency, eroded desert. No sharp alpine peaks, no dune-park playground.
- Height features: broad undulations 15–60 m over 1–4 km; occasional eroded ridge/spur 40–120 m; fine regolith detail under 1 m only as shading, not geometry.
- The opening-shot composition features: a shallow basin to the north, a dark ridge at the right/east, a low ridge anchor at the far left; deterministic.
- Color by height/slope/mineral noise within the palette; no saturated orange; strata/gravel detail in the fragment shader.
- Mesh: camera-following radial grid (polar), rebuilt when the player moves > 32 m; near spacing ≤ 2 m, radius 30 km; desktop 384×200, medium 288×160, low 192×128. Height sampled from the same `terrain_field` used for walking.
- Lighting: direct Kesh (with terrain shadow map, 2–3 tap PCF), sky ambient, Vhal-shine fill; fog/haze blends to the sky horizon color; no seams at the mesh edge.
- Shadows: long, hard, clean. Terrain self-shadowing is required at the opening shot (ridges throw shadows into the basin). Shadow map 2048² desktop, 1024² low.
- Walking: WASD/arrows, sprint Shift; mouse drag/pointer lock look; terrain-following with slope limit (walkable ≤ 34°); no jumping; soft clamp only if leaving the generated 30 km radius is unavoidable. Eye height 1.7 m.

### 6.4 Shadows / occlusion / reflection (correctness)

- Direct sunlight is blocked by any body between a point and Kesh (sphere/ring tests), producing eclipses on Vhal, moons, and rings.
- Rings cast banded shadows on Vhal; Vhal casts a shadow on the rings.
- Terrain casts shadows in surface view; sky bodies are occluded by terrain naturally.
- Reflected light: Vhal-shine (cold teal) contributes ambient/fill on Ossa's surface and on other moons' night sides; intensity controlled by `VHAL LIGHT`.
- Ring-shine may contribute a faint extra rim on Ossa's night side (optional).
- No fake shadow blobs; all shadow shapes must follow geometry.
- In surface view, if Vhal or Kesh is behind terrain, the HUD reports it; in orbit view, occlusion by another body is reported.

---

## 7. UI / HUD

### 7.1 Layout (desktop)

- Top-left at 28px: wordmark `OSSA` (18px, 0.34em), below `MOON OF VHAL` (9px, dim), a 1px hairline, then status: `SURFACE · 48.0°N 12.4°E` or `ORBIT · VHAL`.
- Top-right: telemetry, right-aligned, monospaced:
  - `VHAL PHASE 76.5%`
  - `KESH EL +5.0°`
  - `RING TILT 12.0°`
  - `DAY 0001 · 06:42:10`
  - `FPS 60 · HIGH`
- Left center rail: `01 SURFACE` / `02 ORBIT`, active item marked by a 20px accent hairline; clickable.
- Bottom-left: contextual hint line, 9px dim: surface `WASD MOVE · DRAG LOOK · SHIFT RUN`; orbit `DRAG ORBIT · SCROLL ZOOM`.
- Bottom-right: `OPTICS` control deck, 300–320px wide, backdrop blur 10px, hairline top; collapsible by clicking the header. Controls:
  1. **TIME** — play/pause, speed chips `1× 60× 600× 3600×`, and a thin scrub slider (0–1 of the current orbital period).
  2. **EXPOSURE** — 0.6–1.6 slider.
  3. **HAZE** — 0–1.5 slider (surface only; disabled with note in orbit).
  4. **RINGS** — 0–1.5 slider.
  5. **VHAL LIGHT** — 0–1.5 slider (reflected fill).
  6. **LENS** — FOV 55–95 slider (surface); in orbit, becomes a zoom readout/scale.
  7. **QUALITY** — LOW / MED / HIGH chips.
  8. **FOCUS** — OSSA / VHAL / KESH / SYSTEM chips (orbit only).
  9. **OVERLAYS** — MARKERS on/off, ORBITS on/off.
- Bottom-center in orbit view: scale bar.
- Keyboard: `1` surface, `2` orbit, `Space` pause, `H` hide UI, `M` markers, `O` orbits, `R` reset camera, `F` fullscreen, `Esc` release pointer lock.

Every control must have an immediately visible effect. No dead controls.

### 7.2 HUD markers and edge arrows

Required for **VHAL** and **KESH** in both views (plus any body the player focuses in orbit view).

- Visible + on-screen: a very quiet label/bracket (opacity ≤ 0.55), 9px uppercase + distance, no filled background. Example `VHAL · 250 000 KM`.
- Occluded + on-screen: brackets switch to dashed strokes and the occlusion accent `#c8a978`; add `OCCLUDED` (or `ECLIPSED` for Kesh behind Vhal) and the distance. A 1px center cross marks the projected point.
- Off-screen: a 1px chevron at the screen edge, inset 28px, pointing along the true direction; label beside it, horizontally readable; distance included. Occluded off-screen uses dashed chevron + accent.
- On-screen detection uses the body's projected center and a 12px margin; arrows must point correctly even when the body is behind the camera (mirror the projected direction, never show a random edge).
- Markers must not overlap the control deck or top-left lockup; clamp label anchor inward where needed.
- If both Vhal and Kesh are off-screen in the same direction, their labels must offset vertically to avoid collision.
- Markers update every frame; their CSS movement is smoothed over ~80ms, no lag beyond one frame.

### 7.3 View transition

- 260ms fade toward 92% darkness, switch view, 420ms fade back with a 6° FOV settle in surface view. No white flash, no zoom-blur, no star-wipe.
- During the fade, the HUD cross-fades; the wordmark stays put.
- On first load, a 900ms fade-in from black with the wordmark; no spinner after the wasm is ready.

### 7.4 Loading / failure states

- HTML/CSS loading screen (before WASM): black, wordmark, a 1px indeterminate line at 20% opacity, copy `MOON OF VHAL`. No logos.
- WebGL2 unavailable: a quiet centered message `WEBGL2 UNAVAILABLE` + one sentence; no stack trace. Console gets the real error.
- WASM panic: show `SIGNAL LOST` with a reload hint; don't leave a black page.

### 7.5 Mobile

- Portrait and landscape. Controls: drag to look, on-screen joystick (surface, bottom-left), view rail collapses to two icon-free text tabs at top-left, panel collapsed by default behind `OPTICS` tab.
- Touch targets ≥ 40px; no hover-only affordances.
- Performance defaults to LOW on mobile; HIGH remains selectable.

---

## 8. Performance / quality

Measured on the actual built page, with numbers recorded in `PERF.md`:
- Desktop: 1440×900, DPR 1.5 (or 1.0 headless), HIGH, both views, 10s average FPS.
- Mobile emulation: 390×844, DPR 2 (render scale capped), LOW, CPU throttle 4×, 10s average FPS.
- Headless SwiftShader numbers must be labeled as a software lower bound; if hardware GPU numbers can be obtained, report both.

Quality tiers:
| | LOW | MED | HIGH |
|---|---|---|---|
| Render scale | 0.75 | 0.9 | 1.0 |
| DPR cap | 1.25 | 1.5 | 2.0 |
| Terrain grid | 192×128 | 288×160 | 384×200 |
| Shadow map | 1024² | 1536² | 2048² |
| Sky noise octaves | 3 | 4 | 5 |
| Ring steps | 2 | 3 | 4 |

- If average frame time exceeds 22ms for 2s, show a one-time hint in the panel header `REDUCE QUALITY?` (no automatic switch that surprises the user). Optional.
- Page weight target: < 1.5 MB uncompressed total, excluding wasm (wasm target < 1.2 MB release, optimize later if possible). No texture downloads.
- Render loop pauses when the tab is hidden; delta time clamped to 0.1s.

---

## 9. Acceptance checklist (director's final gate)

Deployment / robustness:
- [ ] GitHub Pages URL loads over HTTPS with correct asset paths; reload on the page works.
- [ ] No blocking console errors. WebGL2 failure path is graceful.
- [ ] Works at 1440×900, 1280×720, 390×844, 844×390.
- [ ] UI text is legible; no overflow/clipping; safe-area insets respected.
- [ ] All controls visibly affect the render (test each one at least once).
- [ ] Markers + edge arrows work for Vhal and Kesh; correct on-screen/off-screen/occluded states; no overlap with UI.
- [ ] Orbit hierarchy readable: Vhal+moons and Kesh+planets; orbit lines don't draw over bodies.
- [ ] Kepler motion is continuous, deterministic, and periodic; no jumps when scrubbing time.
- [ ] Ring shadow on Vhal, Vhal shadow on rings, moon shadows, and eclipse dimming are visible in a scripted time-lapse.
- [ ] Surface: Vhal-shine fill, hard star shadows, horizon haze, terrain self-shadow.
- [ ] Gas giant: bands, layered terminator, ring arc, correct phase, anti-aliased limb.
- [ ] Opening shot matches §3 within tolerance (compare screenshot).
- [ ] Performance: desktop and mobile numbers recorded; LOW/MED/HIGH behave as specified.
- [ ] No external network requests after load (fonts, textures, analytics) — self-contained.

Visual proof:
- [ ] `docs/shots/surface.png` — opening shot, UI hidden, 1440×900.
- [ ] `docs/shots/orbit.png` — default orbit view, UI hidden.
- [ ] `docs/shots/surface-hud.png` — opening shot with HUD + markers.
- [ ] `docs/shots/occluded.png` — terrain-occluded Vhal + edge arrow for Kesh.
- [ ] `docs/shots/eclipse.png` — eclipse/time-lapse frame showing shadows.
- [ ] `docs/shots/mobile.png` — portrait with HUD.

---

## 10. Spec change log

| Version | Change | Reason |
|---|---|---|
| 1.0 | Initial spec | — |

Changes require director approval and a row here. Acceptance criteria do not drift silently.
