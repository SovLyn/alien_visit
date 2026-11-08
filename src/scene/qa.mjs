// Scene integration proof. Serve dist first, then:
// NO_PROXY=127.0.0.1,localhost node src/scene/qa.mjs http://127.0.0.1:8777/
import assert from 'node:assert/strict';
import path from 'node:path';
import {launch,awaitReady,root} from '../../tools/qa/common.mjs';
const base=process.argv[2]||'http://127.0.0.1:8777/';
const browser=await launch();
const context=await browser.newContext({viewport:{width:1440,height:900},deviceScaleFactor:1});
const page=await context.newPage();
const errors=[];
page.on('pageerror',e=>errors.push(e.message));
page.on('response',r=>{if(r.status()>=400&&!r.url().includes('favicon.ico'))errors.push(`${r.status()} ${r.url()}`);});
page.on('console',m=>{
  if(m.text().includes('[ossa scene]'))console.log(m.text());
  if(m.type()==='error'&&!m.text().includes('404'))errors.push(m.text());
});
const frames=()=>page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
const hud=()=>page.evaluate(()=>({
  phase:document.querySelector('#phase')?.textContent,elevation:document.querySelector('#elevation')?.textContent,
  tilt:document.querySelector('#tilt')?.textContent,clock:document.querySelector('#clock')?.textContent,
  markers:['marker-vhal','marker-kesh','arrow-vhal','arrow-kesh'].map(id=>{let e=document.getElementById(id);return {id,hidden:e.hidden,warning:e.classList.contains('warning'),text:e.textContent,transform:e.style.transform};}),
  snapshot:JSON.parse(window.__ossa.snapshot()),
}));
async function capture(name,query){
  await page.goto(`${base}?paused=1&${query}`,{waitUntil:'networkidle'});await awaitReady(page);await frames();
  await page.screenshot({path:path.join(root,`docs/shots/dev-scene-${name}.png`)});
  const result=await hud();console.log(name,JSON.stringify(result));return result;
}
try {
  await capture('surface','view=surface&ui=0');
  await capture('orbit','view=orbit&ui=0&focus=vhal');
  const opening=await capture('hud','view=surface');
  assert.equal(opening.phase,'47.1%');assert.equal(opening.elevation,'+4.6°');assert.equal(opening.tilt,'12.0°');
  assert.equal(opening.clock,'DAY 0001 · 00:00:00');
  assert(opening.markers.filter(m=>!m.hidden).every(m=>!m.warning));
  const arrows=await capture('arrows','view=surface&yaw=180&pitch=0');
  assert(arrows.markers.filter(m=>m.id.startsWith('arrow')).every(m=>!m.hidden));
  const ridge=await capture('ridge','view=surface&t=1867.5695775801682&yaw=87&pitch=0');
  assert(ridge.markers.some(m=>!m.hidden&&m.id==='marker-kesh'&&m.warning&&m.text.includes('OCCLUDED')));
  const eclipse=await capture('eclipse','view=surface&t=64287.491228240404&yaw=0&pitch=30');
  assert(eclipse.markers.some(m=>!m.hidden&&m.id==='marker-kesh'&&m.warning&&m.text.includes('ECLIPSED')));
  // Observe actual input effects through marker projections (not synthetic
  // calls to the Rust camera). Canvas-only drag and wheel, global WASD only.
  await page.goto(`${base}?paused=1`,{waitUntil:'networkidle'});await awaitReady(page);
  const before=await hud();
  await page.mouse.move(720,300);await page.mouse.down();await page.mouse.move(920,340,{steps:12});await page.mouse.up();
  await page.waitForTimeout(180);await frames();
  const after=await hud();assert.notEqual(after.markers[0].transform,before.markers[0].transform);
  await page.keyboard.down('KeyW');await page.waitForTimeout(450);await page.keyboard.up('KeyW');await frames();
  await page.keyboard.press('Digit2');await page.waitForTimeout(850);await frames();
  assert.equal((await hud()).snapshot.view,'ORBIT');
  const orbitBefore=await hud();await page.mouse.move(720,300);await page.mouse.wheel(0,450);
  await page.waitForTimeout(550);await frames();
  const orbitAfter=await hud();assert.notEqual(orbitAfter.markers.find(m=>m.id==='marker-vhal').text,orbitBefore.markers.find(m=>m.id==='marker-vhal').text);
  await page.locator('#focus-ossa').click();await page.waitForTimeout(2600);await frames();
  const focused=await hud();assert.equal(focused.snapshot.focus,'OSSA');
  await page.evaluate(()=>window.__ossa.set({markers:false,orbits:false}));await frames();
  assert((await hud()).markers.every(m=>m.hidden));
  console.log('SCENE INPUT/HUD PASS; non-favicon errors:',JSON.stringify(errors));
  assert.deepEqual(errors,[]);
} finally {await context.close();await browser.close();}
