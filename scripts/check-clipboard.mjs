/** External Chrome UI regression; native IPC/clipboard are mocked. Never changes the OS clipboard. */
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const testUrl = process.env.SHILU_TEST_URL ?? 'http://127.0.0.1:1420';
const profile = await mkdtemp(path.join(tmpdir(), 'shilu-clipboard-check-'));
const output = path.join(project, 'work', 'clipboard-check');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const pending = new Map();
const results = [];
const exceptions = [];
let chrome, socket, sessionId, sequence = 0, chromeExited = false;
function command(method, params = {}, browser = false) {
  const id = ++sequence;
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 10000);
    pending.set(id, { resolve, reject, timeout });
    socket.send(JSON.stringify({ id, method, params, ...(!browser && sessionId ? { sessionId } : {}) }));
  });
}
async function evaluate(expression) {
  const r = await command('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails));
  return r.result.value;
}
async function until(expression, label = expression) {
  for (let attempt = 0; attempt < 160; attempt++) {
    if (await evaluate(expression)) return;
    await pause(30);
  }
  throw new Error(`Timed out: ${label}`);
}
async function click(selector) {
  await evaluate(`document.querySelector(${JSON.stringify(selector)}).scrollIntoView({block:'center'})`);
  const point = await evaluate(`(() => {
    const el = document.querySelector(${JSON.stringify(selector)}), r = el.getBoundingClientRect();
    const x = r.x + r.width / 2, y = r.y + r.height / 2, hit = document.elementFromPoint(x,y);
    if (el.disabled || !(el === hit || el.contains(hit))) throw new Error('Button unavailable: ' + ${JSON.stringify(selector)});
    return {x,y};
  })()`);
  await command('Input.dispatchMouseEvent', { type: 'mousePressed', ...point, button: 'left', buttons: 1, clickCount: 1 });
  await command('Input.dispatchMouseEvent', { type: 'mouseReleased', ...point, button: 'left', buttons: 0, clickCount: 1 });
}
async function clickText(text, scope = '') {
  await evaluate(`(() => {
    document.querySelectorAll('[data-check-target]').forEach(el=>el.removeAttribute('data-check-target'));
    const el=[...document.querySelectorAll(${JSON.stringify(`${scope} button`)})].find(el=>el.textContent.trim()===${JSON.stringify(text)});
    if (!el) throw new Error('Missing button: '+${JSON.stringify(text)}); el.dataset.checkTarget='true';
  })()`);
  await click('[data-check-target]');
}
async function input(selector, value) {
  await click(selector);
  await command('Input.dispatchKeyEvent', { type: 'keyDown', key: 'a', code: 'KeyA', windowsVirtualKeyCode: 65, modifiers: 2 });
  await command('Input.dispatchKeyEvent', { type: 'keyUp', key: 'a', code: 'KeyA', windowsVirtualKeyCode: 65, modifiers: 2 });
  await command('Input.insertText', { text: value });
}
async function pasteImages(count = 1, selector = 'body') {
  return evaluate(`(async () => {
    const dt = new DataTransfer();
    for(let i=0;i<${count};i++) {
      const blob = await (await fetch(window.fixturePng(i))).blob();
      dt.items.add(new File([blob], 'image.png', {type:'image/png'}));
    }
    const event = new ClipboardEvent('paste',{clipboardData:dt,bubbles:true,cancelable:true});
    document.querySelector(${JSON.stringify(selector)}).dispatchEvent(event);
    return event.defaultPrevented;
  })()`);
}
async function navigate(route) {
  await command('Page.navigate', { url: `${testUrl}/?clipboardCheck=${++sequence}#/${route}` });
  await until(`!!document.querySelector(${JSON.stringify(route === 'capture' ? '.capture-dropzone' : '.editor-content')})`);
}
async function screenshot(name, width = 1100, height = 680, theme = 'light') {
  await command('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
  await evaluate(`document.documentElement.dataset.theme=${JSON.stringify(theme)}; document.querySelector('.page-content').scrollTop=0`);
  await pause(180);
  const metrics = await evaluate(`(() => {
    const page=document.querySelector('.page-content');
    const buttons=[...document.querySelectorAll('.capture-dropzone button,.editor-images button')];
    return {document:document.documentElement.scrollWidth,page:page.scrollWidth,client:page.clientWidth,
      buttons:buttons.filter(el=>el.offsetWidth).map(el=>{const r=el.getBoundingClientRect();return {text:el.textContent.trim(),left:r.left,right:r.right,width:r.width}})};
  })()`);
  assert.ok(metrics.document <= width && metrics.page <= metrics.client + 1, JSON.stringify(metrics));
  for(const button of metrics.buttons) assert.ok(button.left >= 0 && button.right <= width + 1, JSON.stringify(button));
  const image = await command('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
  await writeFile(path.join(output, `${name}.png`), Buffer.from(image.data, 'base64'));
  if(await evaluate(`!!document.querySelector('.editor-images')`)) {
    await evaluate(`document.querySelector('.editor-images').scrollIntoView({block:'center'})`);
    const detail = await command('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
    await writeFile(path.join(output, `${name}-images.png`), Buffer.from(detail.data, 'base64'));
  }
}
async function check(name, run) {
  try { await run(); results.push({ name, status: 'PASS' }); console.log(`PASS ${name}`); }
  catch(error) { results.push({ name, status: 'FAIL', error: error.message }); console.error(`FAIL ${name}: ${error.message}`); }
}
async function stopChrome() {
  if (!chrome || chromeExited) return;
  if(socket?.readyState === WebSocket.OPEN) {
    command('Browser.close', {}, true).catch(()=>{});
    await Promise.race([new Promise(resolve=>chrome.once('exit',resolve)),pause(2000)]);
  }
  if(!chromeExited) {
    const killer=spawn('taskkill',['/PID',String(chrome.pid),'/T','/F'],{windowsHide:true,stdio:'ignore'});
    await Promise.race([new Promise(resolve=>killer.once('exit',resolve)),pause(3000)]);
  }
}

// All IPC is intercepted in this isolated Chrome process. Images are generated fixtures.
function installMocks() {
  window.calls = [];
  window.fixturePng = (variant = 0) => {
    const canvas = document.createElement('canvas'); canvas.width = 640; canvas.height = 280;
    const ctx = canvas.getContext('2d'); ctx.fillStyle = variant ? '#eef3fb' : '#fff7eb'; ctx.fillRect(0,0,640,280);
    ctx.fillStyle = '#263b47'; ctx.font = 'bold 34px Arial'; ctx.fillText(`ShiLu screenshot ${variant+1}`,32,72);
    ctx.font = '24px Arial'; ctx.fillText('Clipboard to Markdown',32,126); ctx.fillText('Save first, then OCR.',32,174);
    return canvas.toDataURL('image/png');
  };
  window.documentFixture = { id:'abc123',folderName:'20260911-test-abc123',title:'剪贴板回归资料',summary:'',sourceUrl:'',tags:[],content:'原来的正文',notes:'',createdAt:'2026-09-11',updatedAt:'2026-09-11',markdownPath:'D:/TestLibrary/articles/test/index.md' };
  window.isTauri = true;
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow:{label:'main'},currentWebview:{label:'main'} },
    transformCallback: () => 1,
    unregisterCallback: () => {},
    convertFileSrc: () => window.fixturePng(),
    invoke: async (cmd,args={}) => {
      window.calls.push({cmd,args:JSON.parse(JSON.stringify(args))});
      if(window.delayedCommand===cmd) await new Promise(resolve=>window.releaseCommand=resolve);
      if(window.failedCommand===cmd) throw {code:'TEST_ERROR',message:cmd==='read_clipboard_image'?'剪贴板里没有图片，请先截图后重试。':'测试保存失败，原数据未改动。'};
      switch(cmd) {
        case 'get_app_settings': case 'set_theme_preference': return {theme:args.theme??'light'};
        case 'get_library_status': return {configured:true,libraryPath:'D:/TestLibrary',ready:true,missingItems:[],articleCount:1};
        case 'list_articles': return [];
        case 'plugin:event|listen': return 1;
        case 'plugin:event|unlisten': return null;
        case 'plugin:dialog|open': return ['D:/fixtures/disk.png'];
        case 'plugin:fs|stat': return {size:2048};
        case 'read_clipboard_image': return {base64:window.fixturePng().split(',')[1]};
        case 'create_article_with_sources':
          return {article:window.documentFixture,images:{articleId:'abc123',articleFolderName:window.documentFixture.folderName,images:args.sources.map((_,i)=>({fileName:`00${i+1}.png`}))}};
        case 'import_article_sources':
          return {articleId:'abc123',articleFolderName:window.documentFixture.folderName,images:args.sources.map((_,i)=>({fileName:`00${i+2}.png`}))};
        case 'read_article': return structuredClone(window.documentFixture);
        case 'save_article': window.documentFixture={...window.documentFixture,...args.draft}; return structuredClone(window.documentFixture);
        case 'ocr_article_images': return {articleId:'abc123',text:'Clipboard OCR result',imageCount:2,rawPath:'D:/TestLibrary/raw/ocr.txt'};
        default: throw new Error('Unmocked IPC: '+cmd);
      }
    },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
}

const deadline=setTimeout(()=>{console.error('Clipboard regression timed out');void stopChrome().finally(()=>process.exit(1));},90000);
try {
  await mkdir(output,{recursive:true});
  await fetch(testUrl,{signal:AbortSignal.timeout(5000)});
  chrome=spawn(process.env.CHROME_PATH??'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',[
    '--headless=new','--remote-debugging-port=0',`--user-data-dir=${profile}`,'--no-first-run','--no-default-browser-check',
    '--disable-extensions','--disable-background-networking','--disable-sync','about:blank',
  ],{windowsHide:true,stdio:['ignore','ignore','pipe']});
  chrome.once('exit',()=>{chromeExited=true;});
  const endpoint=await new Promise((resolve,reject)=>{
    let stderr='';const timer=setTimeout(()=>reject(new Error('Chrome CDP startup timeout')),10000);
    chrome.once('error',error=>{clearTimeout(timer);reject(error);});
    chrome.stderr.on('data',data=>{stderr+=data;const match=stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);if(match){clearTimeout(timer);resolve(match[1]);}});
  });
  socket=new WebSocket(endpoint);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  socket.addEventListener('message',({data})=>{
    const message=JSON.parse(data);if(message.method==='Runtime.exceptionThrown')exceptions.push(message.params.exceptionDetails);
    const request=pending.get(message.id);if(!request)return;
    clearTimeout(request.timeout);pending.delete(message.id);
    if(message.error)request.reject(new Error(message.error.message));else request.resolve(message.result);
  });
  const target=await command('Target.createTarget',{url:'about:blank'},true);
  ({sessionId}=await command('Target.attachToTarget',{targetId:target.targetId,flatten:true},true));
  await command('Page.enable');await command('Runtime.enable');
  await command('Page.addScriptToEvaluateOnNewDocument',{source:`(${installMocks.toString()})()`});
  await command('Emulation.setDeviceMetricsOverride',{width:1440,height:900,deviceScaleFactor:1,mobile:false});

  await check('capture: two same-name screenshots preview; text-only paste is untouched',async()=>{
    await navigate('capture');
    assert.equal(await pasteImages(2),true);
    await until(`document.querySelectorAll('.capture-thumb img').length===2 && [...document.querySelectorAll('.capture-thumb img')].every(img=>img.naturalWidth===640)`);
    const canceled=await evaluate(`(() => {const d=new DataTransfer();d.setData('text/plain','普通文字');const e=new ClipboardEvent('paste',{clipboardData:d,bubbles:true,cancelable:true});document.querySelector('.capture-field input').dispatchEvent(e);return e.defaultPrevented;})()`);
    assert.equal(canceled,false);
    assert.equal(await evaluate(`calls.filter(c=>c.cmd==='read_clipboard_image').length`),0);
    await screenshot('capture-1100-light');await screenshot('capture-1440-dark',1440,900,'dark');
  });
  await check('capture: button and file picker mix; reorder survives failed save and retry',async()=>{
    await clickText('粘贴图片');await until(`document.querySelectorAll('.capture-queue-item').length===3`);
    await clickText('选择图片');await until(`document.querySelectorAll('.capture-queue-item').length===4`);
    await click('.capture-queue-item:last-child button[title="上移"]');
    await input('.capture-field input','剪贴板回归资料');
    await evaluate(`failedCommand='create_article_with_sources'`);
    await click('.capture-save-button');await until(`!!document.querySelector('.capture-feedback-error')`);
    assert.equal(await evaluate(`document.querySelectorAll('.capture-queue-item').length`),4);
    await evaluate(`failedCommand='';delayedCommand='create_article_with_sources'`);
    await click('.capture-save-button');await until(`typeof releaseCommand==='function'`);
    await pasteImages();
    assert.equal(await evaluate(`document.querySelectorAll('.capture-queue-item').length`),4);
    await evaluate(`releaseCommand();delayedCommand=''`);
    await until(`!!document.querySelector('.capture-feedback-success')`);
    const sources=await evaluate(`calls.filter(c=>c.cmd==='create_article_with_sources').at(-1).args.sources`);
    assert.deepEqual(sources.map(s=>s.kind),['clipboard','clipboard','path','clipboard']);
    assert.ok(sources[0].base64.startsWith('iVBOR')&&!sources[0].base64.startsWith('data:'));
    assert.notEqual(sources[0].base64,sources[1].base64);
  });
  await check('capture: empty clipboard gives helpful error; late paste does not enter another page',async()=>{
    await navigate('capture');await evaluate(`failedCommand='read_clipboard_image'`);
    await clickText('粘贴图片');await until(`document.body.textContent.includes('剪贴板里没有图片')`);
    assert.equal(await evaluate(`document.querySelectorAll('.capture-queue-item').length`),0);
    await evaluate(`failedCommand='';delayedCommand='read_clipboard_image'`);await clickText('粘贴图片');
    await until(`typeof releaseCommand==='function'`);await evaluate(`location.hash='#/articles/abc123/edit'`);
    await until(`!!document.querySelector('.editor-content')`);await evaluate(`releaseCommand();delayedCommand=''`);await pause(120);
    assert.equal(await evaluate(`document.querySelectorAll('.editor-image-queue li').length`),0);
  });
  await check('editor: pending screenshots preserve text, save before OCR, and OCR inserts only on request',async()=>{
    await navigate('articles/abc123/edit');
    await input('.editor-content','正在编辑的正文，不应被覆盖');
    assert.equal(await pasteImages(2,'.editor-content'),true);
    await until(`document.querySelectorAll('.editor-image-queue li').length===2`);
    assert.equal(await evaluate(`document.querySelector('.editor-content').value`),'正在编辑的正文，不应被覆盖');
    await screenshot('editor-1100-light');await screenshot('editor-1440-dark',1440,900,'dark');
    assert.equal(await evaluate(`document.querySelector('button[aria-label="识别图片文字"]').disabled`),true);
    await click('.editor-append-button');await until(`document.querySelectorAll('.editor-image-queue li').length===0`);
    assert.equal(await evaluate(`document.querySelector('.editor-content').value`),'正在编辑的正文，不应被覆盖');
    await click('button[aria-label="识别图片文字"]');await until(`!!document.querySelector('.editor-ocr pre')`);
    assert.equal(await evaluate(`document.querySelector('.editor-content').value`),'正在编辑的正文，不应被覆盖');
    await clickText('插入正文');
    assert.match(await evaluate(`document.querySelector('.editor-content').value`),/正在编辑的正文，不应被覆盖\n\nClipboard OCR result/);
    assert.equal(await evaluate(`calls.filter(c=>c.cmd==='read_article').length`),1);
    assert.equal(await evaluate(`calls.filter(c=>c.cmd==='import_article_sources').at(-1).args.sources.length`),2);
  });
  await check('editor: failed image save retains preview and draft for retry',async()=>{
    await navigate('articles/abc123/edit');await pasteImages();
    await until(`document.querySelectorAll('.editor-image-queue li').length===1`);
    await evaluate(`failedCommand='import_article_sources'`);await click('.editor-append-button');
    await until(`document.body.textContent.includes('测试保存失败')`);
    assert.equal(await evaluate(`document.querySelectorAll('.editor-image-queue li').length`),1);
    assert.equal(await evaluate(`document.querySelector('.editor-content').value`),'原来的正文');
    await evaluate(`failedCommand=''`);await click('.editor-append-button');
    await until(`document.querySelectorAll('.editor-image-queue li').length===0`);
  });
  await check('no unhandled frontend exceptions',async()=>assert.deepEqual(exceptions,[]));
  await writeFile(path.join(output,'report.json'),JSON.stringify({mode:'external Chrome; mocked IPC and clipboard',results},null,2));
  console.log(`${results.filter(r=>r.status==='PASS').length}/${results.length} passed. Report: ${output}`);
  if(results.some(r=>r.status==='FAIL'))process.exitCode=1;
} finally {
  clearTimeout(deadline);await stopChrome();socket?.close();
  for(const request of pending.values())clearTimeout(request.timeout);
  if(path.dirname(profile)===path.resolve(tmpdir())&&path.basename(profile).startsWith('shilu-clipboard-check-'))await rm(profile,{recursive:true,force:true,maxRetries:3,retryDelay:200});
}
