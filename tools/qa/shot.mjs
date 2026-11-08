import path from 'node:path';
import {launch,openPage,root} from './common.mjs';
const args=process.argv.slice(2); const val=(name, fallback)=>{const i=args.indexOf(name);return i>=0?args[i+1]:fallback;};
const positional=args.filter((x,i)=>!x.startsWith('--') && !(args[i-1]||'').startsWith('--'));
let url=val('--url',positional[0]||process.env.URL||'http://127.0.0.1:8766/'); let output=val('--output',positional[1]||'docs/shots/dev-qa-shot.png');
const width=Number(val('--width',positional[2]||1440)),height=Number(val('--height',positional[3]||900)), waitMs=Number(val('--wait-ms',process.env.WAIT_MS||0));
const hideUi=args.includes('--hide-ui')||process.env.HIDE_UI==='1', fullPage=args.includes('--full-page');
const browser=await launch(); try {const {page,errors,context}=await openPage(browser,url,{width,height}); if(waitMs) await page.waitForTimeout(waitMs); if(hideUi) await page.evaluate(()=>window.__ossa.hideUI(true)); const snap=await page.evaluate(()=>({canvas:(()=>{const c=document.querySelector('#scene');return c?{cssWidth:c.clientWidth,cssHeight:c.clientHeight,width:c.width,height:c.height}:null})(),snapshot:JSON.parse(window.__ossa.snapshot())})); output=path.resolve(root,output); await page.screenshot({path:output,fullPage}); console.log(JSON.stringify({output,...snap},null,2)); await context.close(); if(errors.length) process.exitCode=1;} finally {await browser.close();}
