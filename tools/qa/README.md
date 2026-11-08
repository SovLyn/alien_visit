# Local QA harness

Requires Node.js, Python, Trunk/Rust WASM target, and installed Chrome. No browser download is used. `CHROME_PATH` overrides the default `C:\Program Files\Google\Chrome\Application\chrome.exe`.

From repository root (Git Bash or PowerShell):

```sh
cd tools/qa && npm install --no-audit --no-fund && cd ../..
trunk build --release --public-url ./
python -m http.server 8766 --directory dist
```

In another terminal:

```sh
node tools/qa/shot.mjs "http://127.0.0.1:8766/?view=surface&ui=0&paused=1&t=0" docs/shots/dev-qa-surface.png 1440 900
node tools/qa/shot.mjs "http://127.0.0.1:8766/?view=orbit&ui=0&paused=1&t=0" docs/shots/dev-qa-orbit.png 1440 900
node tools/qa/smoke.mjs http://127.0.0.1:8766/
node tools/qa/perf.mjs --url http://127.0.0.1:8766/
```

Shot options: `--url URL --output PATH --width N --height N --hide-ui --wait-ms N --full-page` (positional URL output width height also supported). Screenshots use DPR 1. Output path is rooted at repository. Environment `URL`, `WAIT_MS`, `HIDE_UI`, `CHROME_PATH` supported.

Perf runs 10 seconds per desktop surface HIGH, desktop orbit HIGH, and mobile surface LOW scenario; samples `__ossa.fps()` once per second and prints JSON including drawing buffer, GPU string and errors. Save output yourself with shell redirection if desired, or tee it to `tools/qa/perf-results.json` (`node ... > tools/qa/perf-results.json`). These Chrome headless SwiftShader figures are a **software lower bound**, not representative hardware GPU performance. GPU selection/driver and machine load affect results. Mobile applies 4× CDP CPU throttling where available.

`smoke.mjs` checks scene/debug functionality, loading indicator and screenshot byte-size as a nonblank heuristic; missing UI controls are reported/tolerated during UI development. Console errors and page exceptions fail shot/smoke. Warnings are logged, not treated as errors. No known errors are filtered. It does not validate pixel-level art or actual visible canvas pixels: WebGL canvas screenshot data is intentionally only checked by file size.
