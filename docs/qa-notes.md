# QA notes

Local visual/performance harness lives in `tools/qa/README.md`. Development screenshots are not committed; the final evidence is under `docs/shots/*.png` (surface, surface-hud, orbit, orbit-hud, system, occluded, eclipse, arrows, mobile). Headless Chrome is explicitly forced to SwiftShader; report its FPS as a software lower bound, not hardware performance. The smoke nonblank test uses screenshot byte size, not pixel variance, so it can detect a missing/empty screenshot but is not a visual correctness test. Check the actual image against `VISUAL_SPEC.md` acceptance items.
