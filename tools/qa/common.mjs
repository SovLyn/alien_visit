import { chromium } from 'playwright-core';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
export const chromePath = process.env.CHROME_PATH || 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
export async function launch() {
  const base = ['--headless=new', '--no-sandbox', '--disable-gpu-sandbox'];
  const gpu = process.env.SWIFTSHADER === '0' ? [] : ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'];
  return chromium.launch({ executablePath: chromePath, headless: true, args: [...base, ...gpu] });
}
export function monitor(page) { const errors=[]; page.on('console', m=>{ if(m.type()==='error'){const text=m.text(); errors.push({kind:'console',text}); console.error(`[console error] ${text}`);} else if(m.type()==='warning') console.warn(`[console warning] ${m.text()}`); }); page.on('pageerror', e=>{errors.push({kind:'pageerror',text:e.message}); console.error(`[page error] ${e.stack||e.message}`);}); return errors; }
export async function awaitReady(page) { await page.waitForFunction(()=>window.__ossa && window.__ossa.ready(),null,{timeout:30000}); await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))); await page.waitForTimeout(800); }
export async function openPage(browser, url, {width=1440,height=900,dpr=1}={}) { const context=await browser.newContext({viewport:{width,height},deviceScaleFactor:dpr}); const page=await context.newPage(); const errors=monitor(page); await page.goto(url,{waitUntil:'networkidle'}); await awaitReady(page); return {context,page,errors}; }
