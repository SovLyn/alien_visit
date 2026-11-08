# OSSA — Performance

Measured on 2026-11-08 against the release build (`trunk build --release --public-url ./`).
The purpose is honest, reproducible evidence — not a marketing number.

## Environment

| | |
|---|---|
| Machine | Ryzen 7 7745HX, NVIDIA RTX 4070 Laptop + Radeon iGPU, Windows 11 Home |
| Browser | Chrome 156.0.8078.48, headless (`--headless=new`) |
| Tooling | Node 22.19, Playwright Core, Python static server |
| Build | Rust 1.95.0, Trunk 0.21.14, release + LTO |
| Display | 1440×900 / 390×844, DPR 1 / 2 as specified below |

Harness: `tools/qa/perf.mjs` — 10 seconds per case, one `window.__ossa.fps()` sample per
second after a 1.2 s settle. FPS is the app's smoothed frame rate (`src/main.rs`), not a
synthetic benchmark. The app's render loop is capped by headless Chrome's
requestAnimationFrame (~240 Hz), so any 239–240 value means "at or above the cap", not a
precise GPU ceiling.

## Results

### Software rasteriser (SwiftShader) — conservative lower bound

Chrome launched with `--use-angle=swiftshader --enable-unsafe-swiftshader`.

| Case | Drawing buffer | Quality | Min / avg / max FPS |
|---|---|---|---|
| Desktop, surface | 1440×900 | HIGH | 10.0 / **10.0** / 10.0 |
| Desktop, orbit | 1440×900 | HIGH | 10.0 / **10.0** / 10.0 |
| Mobile, surface | 366×791 (CSS 390×844, DPR 2 capped) | LOW, 4× CPU throttle | 13.9 / **16.3** / 25.6 |

After the layered-regolith pass, a re-run on the same SwiftShader setup measured
mobile LOW **12.6 / 13.2 / 15.1 FPS** (min / avg / max) and desktop HIGH steady
10.0 FPS after boot; the desktop 10-second average is distorted by a 137 FPS
first-second sample and is not used as a headline. Treat the newer mobile figure
as the current floor and the table above as the earlier baseline.

These are CPU-emulated GPU numbers. They are useful as a floor: even without a real GPU
the page is interactive, and a real GPU has a large margin.

### Hardware headless — uncertified, cap-limited

Chrome launched without the SwiftShader flags; the renderer string is masked by Chrome.

| Case | Quality | Min / avg / max FPS |
|---|---|---|
| Desktop, surface | HIGH | 238.3 / **239.5** / 239.8 |
| Desktop, orbit | HIGH | 239.2 / **239.6** / 239.8 |
| Mobile emulation, surface | LOW, 4× CPU throttle | 239.4 / **239.6** / 239.8 |

All three sit at the ~240 Hz headless cap. This means the static scene renders below
~4.2 ms/frame on this machine's GPU; it does **not** mean 240 fps on a 60 Hz display or on
a phone. The mobile case is a desktop GPU with a mobile viewport, not a phone GPU.

## Quality / downgrade behaviour

- `Settings::auto_quality()` selects **LOW** on mobile user agents or ≤ 4 logical cores,
  **MEDIUM** on ≤ 8 cores, otherwise **HIGH**. The acceptance test opens the page with an
  Android user agent and confirms `LOW`, a drawing buffer of 366×791 (DPR capped at 1.25
  and render scale 0.75), and the collapsed mobile deck.
- LOW reduces render scale (0.75), DPR cap (1.25), terrain grid (192×128), shadow map
  (1024²), sky octaves (3), and ring steps (2); HIGH uses 1.0 / 2.0 / 384×200 / 2048² /
  5 / 4. HIGH and LOW are both retained in the deck.
- The orbit view uses a 1.5× ring material lift for legibility at its more face-on angle;
  the RINGS slider remains authoritative.

## Startup and walking

- The HIGH terrain mesh build + GPU upload is logged by the app; the latest four-build
  sample after the detail pass measured **42.5–46.4 ms** (LOW 15.3 ms). Earlier builds
  measured ≈57.6 ms, and one instrumented load reached 107.8 ms, so this remains the
  largest known frame-time spike under load. The mesh is rebuilt when the player moves
  more than 96 m (was 32 m) or changes quality, so a sprinting player sees a brief hitch
  roughly every 15 s. Normal walking (2.1 m/s) rebuilds about every 45 s; it does not
  occur while standing or orbiting. The layered regolith shader evaluates up to six
  noise bands at close range, skips fully filtered bands, and widens PCF only on steep
  faces; hardware GPU cost was not measured.
- Shader compilation happens once at boot; the sky/terrain programs are cached. The
  first frames include the terrain mesh build; the loading veil covers the transition.
- CPU-side scene + HUD cost is ≈0.08 ms (surface) / ≈0.005 ms (orbit) in a native
  optimized fixture; this is a scene-only figure, not browser/GPU timing.

## Page weight

`dist/` total **≈366 KB uncompressed** at the measured build:

| Asset | Raw | gzip |
|---|---|---|
| WASM | 282 KB | 126 KB |
| JS glue | 60 KB | 10 KB |
| CSS | 9.3 KB | 2.8 KB |
| HTML + favicon data URI | 6.2 KB | ~2 KB |

No textures, fonts, analytics, or third-party network requests. The app remains
functional after the initial load without network access.

## Reproduce

```bash
trunk build --release --public-url ./
python -m http.server 8788 --bind 127.0.0.1 --directory dist

# software lower bound (default of the harness)
NO_PROXY=127.0.0.1,localhost node tools/qa/perf.mjs --url http://127.0.0.1:8788/

# hardware headless (omit SwiftShader flags)
SWIFTSHADER=0 NO_PROXY=127.0.0.1,localhost \
  node tools/qa/perf.mjs --url http://127.0.0.1:8788/

# full control / console / mobile acceptance
NO_PROXY=127.0.0.1,localhost node tools/qa/acceptance.mjs http://127.0.0.1:8788/
```

## Caveats

- Software numbers are a lower bound; hardware numbers are headless-cap-limited and not a
  certified device benchmark. A real phone GPU (Adreno/Mali/Apple) was not available in
  this environment.
- The headless mobile case uses a mobile viewport/UA but desktop silicon; it validates the
  downgrade path, not phone thermals.
- The 240 Hz ceiling masks differences between HIGH/LOW on the hardware path. On a real
  60 Hz display the page has ample headroom; the QUALITY control still changes drawing
  buffer, shadow map, terrain density, and sky cost.
- No automated performance gate is in CI; the numbers above are a snapshot.
