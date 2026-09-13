/** External Chrome with isolated fake IPC; never reads the user's clipboard or library. */
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
const url = process.env.SHILU_TEST_URL ?? 'http://127.0.0.1:1421';
const output = path.resolve('work/capture-flow-check');
const profile = await mkdtemp(path.join(tmpdir(), 'shilu-capture-flow-'));
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
let chrome, socket, sessionId, sequence = 0;
const pending = new Map(), results = [], exceptions = [];
function command(method, params = {}, browser = false) {
  const id = ++sequence;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout ${method}`)); }, 10000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params, ...(!browser && sessionId ? { sessionId } : {}) }));
  });
}
async function evaluate(expression) {
  const r = await command('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails));
  return r.result.value;
}
async function until(expression) {
  for (let i = 0; i < 160; i++) { if (await evaluate(expression)) return; await pause(35); }
  throw new Error(`Timed out: ${expression}`);
}
async function click(selector) {
  await evaluate(`document.querySelector(${JSON.stringify(selector)}).scrollIntoView({block:'center'})`);
  const point = await evaluate(`(()=>{const el=document.querySelector(${JSON.stringify(selector)}),r=el.getBoundingClientRect(),x=r.x+r.width/2,y=r.y+r.height/2,hit=document.elementFromPoint(x,y);if(el.disabled||!(el===hit||el.contains(hit)))throw new Error('not clickable '+${JSON.stringify(selector)});return {x,y};})()`);
  await command('Input.dispatchMouseEvent', { type: 'mousePressed', ...point, button: 'left', buttons: 1, clickCount: 1 });
  await command('Input.dispatchMouseEvent', { type: 'mouseReleased', ...point, button: 'left', buttons: 0, clickCount: 1 });
}
async function button(text) {
  await evaluate(`(()=>{document.querySelectorAll('[data-test-target]').forEach(e=>e.removeAttribute('data-test-target'));const e=[...document.querySelectorAll('button')].find(e=>e.offsetWidth&&e.textContent.trim()===${JSON.stringify(text)});if(!e)throw new Error('missing '+${JSON.stringify(text)});e.dataset.testTarget='1';})()`);
  await click('[data-test-target]');
}
async function input(selector, value) {
  await click(selector);
  await command('Input.dispatchKeyEvent', { type: 'keyDown', key: 'a', code: 'KeyA', windowsVirtualKeyCode: 65, modifiers: 2 });
  await command('Input.dispatchKeyEvent', { type: 'keyUp', key: 'a', code: 'KeyA', windowsVirtualKeyCode: 65, modifiers: 2 });
  await command('Input.insertText', { text: value });
}
async function navigate(route, selector) {
  await evaluate(`location.hash=${JSON.stringify('#/' + route)}`);
  await until(`!!document.querySelector(${JSON.stringify(selector)})`);
  await pause(100);
}
async function reset() {
  await command('Page.navigate', { url: `${url}/?capturecheck=${++sequence}#/capture` });
  await until(`!!document.querySelector('.capture-dropzone')`);
}
async function paste(count = 1) {
  await evaluate(`(async()=>{const data=new DataTransfer();for(let i=0;i<${count};i++)data.items.add(new File([await (await fetch(fixture)).blob()],'image.png',{type:'image/png'}));document.body.dispatchEvent(new ClipboardEvent('paste',{bubbles:true,cancelable:true,clipboardData:data}));})()`);
  await until(`document.querySelectorAll('.capture-queue-item').length>=${count}`);
}
async function check(name, action) {
  try { await action(); results.push({ name, status: 'PASS' }); console.log(`PASS ${name}`); }
  catch (error) { results.push({ name, status: 'FAIL', error: error.message }); console.error(`FAIL ${name}: ${error.message}`); }
}
async function screenshot(name, width, height) {
  await command('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
  await evaluate(`document.querySelector('.page-content').scrollTop=0`); await pause(200);
  const metrics = await evaluate(`(()=>{const p=document.querySelector('.page-content');return {doc:document.documentElement.scrollWidth,scroll:p.scrollWidth,client:p.clientWidth}})()`);
  assert.ok(metrics.doc <= width && metrics.scroll <= metrics.client + 1, JSON.stringify(metrics));
  const shot = await command('Page.captureScreenshot', { format: 'png' });
  await writeFile(path.join(output, `${name}.png`), Buffer.from(shot.data, 'base64'));
}
function mocks() {
  window.calls = []; window.docs = {}; window.images = {}; window.hold = {}; window.release = {}; window.fail = ''; window.failSaveAfterOcr = false; window.ocrReady = true; window.count = 0;
  const canvas = document.createElement('canvas'); canvas.width=480;canvas.height=240;const ctx=canvas.getContext('2d');ctx.fillStyle='#f1f8fc';ctx.fillRect(0,0,480,240);ctx.fillStyle='#125b77';ctx.font='24px Arial';ctx.fillText('ShiLu source screenshot',24,80);window.fixture=canvas.toDataURL('image/png');
  const summary = {summary:'',tags:[],content:'',notes:'',createdAt:'2026-09-12',updatedAt:'2026-09-12',status:'draft',captureStep:2,sourceImages:[]};
  function imported(folder, sources) { return sources.map((_source,i)=>{const n=String((images[folder]??0)+1).padStart(3,'0');images[folder]=(images[folder]??0)+1;return {fileName:n+'.png',originalFileName:'image.png',path:`D:/Test/${folder}/images/${n}.png`,relativePath:`images/${n}.png`,extension:'png',sizeBytes:100};}); }
  window.isTauri=true;
  window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
  window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},convertFileSrc:()=>fixture,transformCallback:()=>1,unregisterCallback:()=>{},invoke:async(cmd,args={})=>{
    args=JSON.parse(JSON.stringify(args));
    calls.push({cmd,args});
    if(hold[cmd])await new Promise(resolve=>release[cmd]=resolve);
    if(fail===cmd)throw {message:'模拟失败，请重试。'};
    if(cmd==='plugin:event|listen')return 1;
    if(cmd==='plugin:event|unlisten')return null;
    if(cmd==='plugin:fs|stat')return {size:100};
    if(cmd==='get_app_settings')return {theme:'light',ocrModel:{enabled:ocrReady,baseUrl:'https://example.test',apiKey:'fake-key',model:'vision'},polishModel:{enabled:true,baseUrl:'https://example.test',apiKey:'fake-key',model:'writer'}};
    if(cmd==='get_library_status')return {configured:true,libraryPath:'D:/Test',ready:true,missingItems:[],articleCount:Object.keys(docs).length};
    if(cmd==='read_clipboard_image')return {base64:fixture.split(',')[1],width:480,height:240};
    if(cmd==='create_capture_draft') {const folder='20260912-test-'+String(++count).padStart(6,'0'),rows=imported(folder,args.sources);docs[folder]={...structuredClone(summary),id:folder.slice(-6),folderName:folder,title:args.title||'未命名资料',sourceUrl:args.sourceUrl||'',sourceImages:rows.map(r=>r.relativePath),markdownPath:`D:/Test/${folder}/index.md`};return {article:{...docs[folder],folderPath:`D:/Test/${folder}`},images:{articleId:folder.slice(-6),articleFolderName:folder,images:rows}};}
    if(cmd==='read_article')return structuredClone(docs[args.articleReference]);
    if(cmd==='save_article'){Object.assign(docs[args.articleReference],args.draft,{title:args.draft.title.trim()||'未命名资料'});return structuredClone(docs[args.articleReference]);}
    if(cmd==='import_article_sources')return {articleId:args.articleReference.slice(-6),articleFolderName:args.articleReference,images:imported(args.articleReference,args.sources)};
    if(cmd==='ocr_article_with_model'){if(failSaveAfterOcr)fail='save_article';return {text:'## 识别标题\n\n这是可编辑的正文。',imageCount:docs[args.articleReference].sourceImages.length,articleId:args.articleReference.slice(-6),rawPath:''};}
    if(cmd==='list_articles')return Object.values(docs).filter(d=>d.status===args.status);
    throw new Error('unmocked '+cmd);
  }};
}
try {
  await mkdir(output,{recursive:true});
  chrome=spawn('C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',['--headless=new','--remote-debugging-port=0',`--user-data-dir=${profile}`,'--no-first-run','--disable-extensions','--disable-background-networking','about:blank'],{windowsHide:true,stdio:['ignore','ignore','pipe']});
  const endpoint=await new Promise((resolve,reject)=>{let data='';const timer=setTimeout(()=>reject(new Error('Chrome startup timeout')),10000);chrome.once('error',reject);chrome.stderr.on('data',chunk=>{data+=chunk;const m=data.match(/DevTools listening on (ws:\/\/\S+)/);if(m){clearTimeout(timer);resolve(m[1]);}});});
  socket=new WebSocket(endpoint);await new Promise(resolve=>socket.addEventListener('open',resolve,{once:true}));
  socket.addEventListener('message',({data})=>{const m=JSON.parse(data);if(m.method==='Runtime.exceptionThrown')exceptions.push(m.params.exceptionDetails);if(m.method==='Page.javascriptDialogOpening')void command('Page.handleJavaScriptDialog',{accept:true});const p=pending.get(m.id);if(p){clearTimeout(p.timer);pending.delete(m.id);m.error?p.reject(new Error(m.error.message)):p.resolve(m.result);}});
  const target=await command('Target.createTarget',{url:'about:blank'},true);({sessionId}=await command('Target.attachToTarget',{targetId:target.targetId,flatten:true},true));
  await command('Page.enable');await command('Runtime.enable');await command('Emulation.setDeviceMetricsOverride',{width:1440,height:900,deviceScaleFactor:1,mobile:false});
  await command('Page.addScriptToEvaluateOnNewDocument',{source:`(${mocks.toString()})()`});
  await check('empty capture navigation creates no draft',async()=>{await reset();await navigate('drafts','.library-view');assert.equal(await evaluate(`calls.filter(c=>c.cmd==='create_capture_draft').length`),0);});
  await check('paste and step navigation preserve images, title and URL',async()=>{await reset();await paste(2);await screenshot('capture-step1-1440',1440,900);await button('下一步：补充信息');await input('.capture-field input[type=text]','收集测试');await input('.capture-field input[type=url]','https://example.test/note');await button('上一步');assert.equal(await evaluate(`document.querySelectorAll('.capture-queue-item').length`),2);await button('下一步：补充信息');assert.equal(await evaluate(`document.querySelector('.capture-field input[type=text]').value`),'收集测试');await screenshot('capture-step2-1100',1100,680);});
  await check('save and resave use one draft; append reorder remove preserve source identity',async()=>{await button('保存草稿');await until(`document.body.textContent.includes('已保存到草稿箱')`);await button('保存草稿');await until(`!document.querySelector('.capture-save-actions button').disabled`);assert.equal(await evaluate(`calls.filter(c=>c.cmd==='create_capture_draft').length`),1);await button('上一步');await paste(1);await click('button[aria-label="将第 3 张截图上移"]');await click('button[aria-label="移除第 1 张截图"]');await button('保存草稿');await until(`!document.querySelector('.capture-save-actions button').disabled`);assert.deepEqual(await evaluate(`Object.values(docs)[0].sourceImages`),['images/003.png','images/002.png']);assert.equal(await evaluate(`calls.filter(c=>c.cmd==='import_article_sources').length`),1);});
  await check('draft restores collection step and source images',async()=>{await navigate('drafts','.library-view');await click('.article-open');await until(`!!document.querySelector('.capture-dropzone')`);assert.equal(await evaluate(`document.querySelectorAll('.capture-queue-item').length`),2);await button('下一步：补充信息');assert.equal(await evaluate(`document.querySelector('.capture-field input[type=text]').value`),'收集测试');});
  await check('OCR failure retains Step2; retry fills main Markdown editor',async()=>{await evaluate(`fail='ocr_article_with_model'`);await button('下一步：识别并编辑');await until(`!!document.querySelector('.capture-feedback-error')`);assert.equal(await evaluate(`document.querySelector('.capture-field input[type=text]').value`),'收集测试');await evaluate(`fail=''`);await button('下一步：识别并编辑');await until(`!!document.querySelector('.editor-content')`);assert.equal(await evaluate(`document.querySelector('.editor-content').value`),'## 识别标题\n\n这是可编辑的正文。');assert.equal(await evaluate(`Object.values(docs)[0].captureStep`),3);});
  await check('OCR success with disk error retries saving without another OCR call',async()=>{await reset();await paste();await button('下一步：补充信息');await input('.capture-field input[type=text]','保存失败恢复');await evaluate(`failSaveAfterOcr=true`);await button('下一步：识别并编辑');await until(`!!document.querySelector('.capture-feedback-error')`);await evaluate(`fail='';failSaveAfterOcr=false`);await button('下一步：编辑正文');await until(`!!document.querySelector('.editor-content')`);assert.equal(await evaluate(`calls.filter(c=>c.cmd==='ocr_article_with_model').length`),1);});
  await check('missing model stores draft; saving failure blocks navigation',async()=>{await reset();await paste();await button('下一步：补充信息');await input('.capture-field input[type=text]','待配置模型');await evaluate(`ocrReady=false`);await button('下一步：识别并编辑');await until(`!!document.querySelector('.capture-settings-button')`);assert.equal(await evaluate(`Object.values(docs)[0].status`),'draft');await input('.capture-field input[type=text]','修改未保存');await evaluate(`fail='save_article'`);await button('前往模型设置');await until(`!!document.querySelector('.capture-feedback-error')`);assert.ok(await evaluate(`!!document.querySelector('.capture-view')`));await evaluate(`fail=''`);await button('前往模型设置');await until(`!!document.querySelector('.settings-view')`);assert.equal(await evaluate(`Object.values(docs)[0].title`),'修改未保存');});
  await check('untitled draft restores screenshots and still requires a title before OCR',async()=>{await reset();await paste();await button('保存草稿');await until(`document.body.textContent.includes('已保存到草稿箱')`);assert.equal(await evaluate(`Object.values(docs)[0].title`),'未命名资料');await navigate('drafts','.library-view');await click('.article-open');await until(`!!document.querySelector('.capture-dropzone')`);assert.equal(await evaluate(`document.querySelectorAll('.capture-queue-item').length`),1);await screenshot('capture-restored-1100',1100,680);await button('下一步：补充信息');assert.equal(await evaluate(`document.querySelector('.capture-field input[type=text]').value`),'');await button('下一步：识别并编辑');await until(`document.body.textContent.includes('请填写资料标题后再识别')`);assert.equal(await evaluate(`calls.filter(c=>c.cmd==='ocr_article_with_model').length`),0);});
  await check('no unhandled errors',async()=>assert.deepEqual(exceptions,[]));
  await writeFile(path.join(output,'report.json'),JSON.stringify({mode:'external Chrome; fake IPC',results},null,2));
  console.log(`${results.filter(r=>r.status==='PASS').length}/${results.length} passed`);
  if(results.some(r=>r.status==='FAIL'))process.exitCode=1;
} finally {
  if(socket?.readyState===WebSocket.OPEN){await command('Browser.close',{},true).catch(()=>{});socket.close();}
  if(chrome&&chrome.exitCode===null)await Promise.race([new Promise(resolve=>chrome.once('exit',resolve)),pause(2000)]);
  for(const p of pending.values())clearTimeout(p.timer);
  if(path.dirname(profile)===path.resolve(tmpdir())&&path.basename(profile).startsWith('shilu-capture-flow-'))await rm(profile,{recursive:true,force:true,maxRetries:3,retryDelay:200});
}
