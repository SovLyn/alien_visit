// Director acceptance smoke test: controls, debug API, console health.
// Usage: NO_PROXY=127.0.0.1,localhost node tools/qa/acceptance.mjs http://127.0.0.1:8788/
import { chromium } from 'playwright-core';
const base = process.argv[2] || process.env.URL || 'http://127.0.0.1:8788/';
const chromePath = process.env.CHROME_PATH || 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const results = [];
let failed = 0;
const check = (name, ok, detail) => {
  results.push({ name, ok: !!ok, detail });
  if (!ok) failed++;
};
const url = (q = '') => `${base}?ui=1&quality=high${q}`;
const browser = await chromium.launch({
  executablePath: chromePath,
  headless: true,
  args: ['--headless=new', '--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--disable-gpu-sandbox'],
});
const errors = [];
try {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  const page = await context.newPage();
  page.on('pageerror', e => errors.push('pageerror: ' + e.message));
  page.on('console', m => { if (m.type() === 'error') errors.push('console: ' + m.text()); });
  await page.goto(url(), { waitUntil: 'networkidle', timeout: 60000 });
  await page.waitForFunction(() => window.__ossa && window.__ossa.ready(), null, { timeout: 30000 });
  await page.waitForTimeout(700);
  const snap = async () => JSON.parse(await page.evaluate(() => window.__ossa.snapshot()));
  const setRange = async (sel, value) => {
    await page.locator(sel).evaluate((el, v) => {
      el.value = String(v);
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.dispatchEvent(new Event('change', { bubbles: true }));
    }, value);
    await page.waitForTimeout(120);
  };
  const s0 = await snap();
  check('boot surface / high', s0.view === 'SURFACE' && s0.quality === 'HIGH', s0);
  check('opening framing values', Math.abs(s0.fov - 76) < 0.01 && Math.abs(s0.exposure - 1) < 0.01, s0);
  check('markers + orbits on', s0.markers === true && s0.orbits === true, s0);
  check('loader hidden', await page.locator('#loader').evaluate(e => e.classList.contains('is-hidden')), '');

  // Basic controls.
  await setRange('#exposure', 1.4);
  await setRange('#haze', 0.2);
  await setRange('#rings', 0.5);
  await setRange('#light', 0.4);
  await setRange('#fov', 90);
  const s1 = await snap();
  check('sliders reach settings', Math.abs(s1.exposure - 1.4) < 0.01 && Math.abs(s1.haze - 0.2) < 0.01 && Math.abs(s1.rings - 0.5) < 0.01 && Math.abs(s1.vhalLight - 0.4) < 0.01 && Math.abs(s1.fov - 90) < 0.01, s1);

  await page.locator('#quality-low').click(); await page.waitForTimeout(1400);
  const s2 = await snap();
  check('quality low', s2.quality === 'LOW' && s2.width < s0.width, s2);

  await page.locator('#speed-600').click(); await page.waitForTimeout(700);
  check('time scale 600', (await snap()).timeScale === 600, await snap());
  await page.locator('#pause').click(); await page.waitForTimeout(700);
  const clockA = (await snap()).time;
  await page.waitForTimeout(800);
  const clockB = (await snap()).time;
  check('pause freezes sim time', Math.abs(clockA - clockB) < 1e-6, { clockA, clockB });
  await page.locator('#pause').click();
  await page.waitForTimeout(400);

  await setRange('#scrub', 0.5);
  await page.waitForTimeout(800);
  const scrub = (await snap()).time;
  check('scrub changes time', scrub > 1000, scrub);

  await page.locator('#markers').click(); await page.waitForTimeout(800);
  check('markers toggle', (await snap()).markers === false, null);
  await page.locator('#markers').click();
  await page.locator('#orbits').click(); await page.waitForTimeout(800);
  check('orbits toggle', (await snap()).orbits === false, null);
  await page.locator('#orbits').click();

  // View / focus / orbit UI.
  await page.locator('#view-orbit').click(); await page.waitForTimeout(700);
  const so = await snap();
  check('view orbit', so.view === 'ORBIT', so);
  check('focus group visible', await page.locator('#focus-group').isVisible(), '');
  check('orbit key visible', await page.locator('#orbit-key').isVisible(), '');
  await page.locator('#focus-system').click(); await page.waitForTimeout(2600);
  const sf = await snap();
  check('focus system', sf.focus === 'SYSTEM', sf);
  await page.locator('#focus-vhal').click(); await page.waitForTimeout(2600);
  check('focus vhal', (await snap()).focus === 'VHAL', null);
  await page.locator('#quality-high').click(); await page.waitForTimeout(250);

  // View switch keyboard and hide UI.
  await page.keyboard.press('Digit1'); await page.waitForTimeout(1000);
  check('key 1 surface', (await snap()).view === 'SURFACE', null);
  await page.keyboard.press('KeyH'); await page.waitForTimeout(600);
  check('H hides UI', await page.locator('#ui-root').evaluate(e => e.classList.contains('is-hidden')), '');
  await page.keyboard.press('KeyH'); await page.waitForTimeout(600);
  check('H shows UI', !(await page.locator('#ui-root').evaluate(e => e.classList.contains('is-hidden'))), '');

  // Deck collapse.
  await page.locator('#deck-toggle').click(); await page.waitForTimeout(350);
  check('deck collapses', await page.locator('#deck').evaluate(e => e.classList.contains('collapsed')), '');
  await page.locator('#deck-toggle').click(); await page.waitForTimeout(350);

  // Pointer look changes a projected marker.
  const before = await page.evaluate(() => document.getElementById('marker-vhal').style.transform);
  await page.mouse.move(700, 300); await page.mouse.down(); await page.mouse.move(940, 340, { steps: 12 }); await page.mouse.up();
  await page.waitForTimeout(250);
  const after = await page.evaluate(() => document.getElementById('marker-vhal').style.transform);
  check('drag look moves marker', before !== after, { before, after });

  // Debug API.
  await page.evaluate(() => window.__ossa.set({ exposure: 0.8, haze: 1.2, rings: 0.2, vhalLight: 0.7, fov: 60, quality: 'med' }));
  await page.waitForTimeout(900);
  const sd = await snap();
  check('debug set applies', Math.abs(sd.exposure - 0.8) < 0.01 && Math.abs(sd.haze - 1.2) < 0.01 && Math.abs(sd.rings - 0.2) < 0.01 && Math.abs(sd.fov - 60) < 0.01 && sd.quality === 'MED', sd);
  await page.evaluate(() => window.__ossa.hideUI(true)); await page.waitForTimeout(300);
  check('debug hideUI', !(await snap()).ui, null);
  await page.evaluate(() => window.__ossa.hideUI(false));

  check('console clean', errors.length === 0, errors);
  await page.screenshot({ path: 'docs/shots/acceptance-desktop.png' });
  await context.close();

  // Mobile / touch context.
  const mcontext = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, userAgent: 'Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Mobile Safari/537.36' });
  const mpage = await mcontext.newPage();
  mpage.on('pageerror', e => errors.push('mobile pageerror: ' + e.message));
  mpage.on('console', m => { if (m.type() === 'error') errors.push('mobile console: ' + m.text()); });
  await mpage.goto(`${base}?paused=1&ui=1`, { waitUntil: 'networkidle', timeout: 60000 });
  await mpage.waitForFunction(() => window.__ossa && window.__ossa.ready(), null, { timeout: 30000 });
  await mpage.waitForTimeout(700);
  const ms = JSON.parse(await mpage.evaluate(() => window.__ossa.snapshot()));
  check('mobile low + portrait', ms.quality === 'LOW' && ms.width <= 500, ms);
  check('mobile deck collapsed', await mpage.locator('#deck').evaluate(e => e.classList.contains('collapsed')), '');
  const joyVisible = await mpage.locator('#joystick').evaluate(e => getComputedStyle(e).display !== 'none');
  check('touch joystick present', joyVisible, '');
  if (joyVisible) {
    const box = await mpage.locator('#joystick').boundingBox();
    await mpage.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await mpage.mouse.down();
    await mpage.mouse.move(box.x + box.width - 4, box.y + box.height / 2, { steps: 8 });
    const dot = await mpage.locator('#joystick-dot').evaluate(e => e.style.transform);
    await mpage.mouse.up();
    check('joystick moves', !!dot && dot !== 'translate(0px,0px)', dot);
  }
  await mpage.screenshot({ path: 'docs/shots/acceptance-mobile.png' });
  
  await mcontext.close();
} finally {
  await browser.close();
}
console.log(JSON.stringify({ passed: results.filter(r => r.ok).length, failed, results, errors }, null, 2));
if (failed) process.exitCode = 1;
