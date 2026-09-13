/** Isolated external Chrome regression. All IPC uses fake keys and controlled responses. */
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const testUrl = process.env.SHILU_TEST_URL ?? 'http://127.0.0.1:1421';
const profile = await mkdtemp(path.join(tmpdir(), 'shilu-model-check-'));
const output = path.join(project, 'work', 'model-settings-check');
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const pending = new Map(), results = [], exceptions = [];
let chrome, socket, sessionId, sequence = 0, exited = false;
function command(method, params = {}, browser = false) {
  const id = ++sequence;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout ${method}`)); }, 10000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params, ...(!browser && sessionId ? { sessionId } : {}) }));
  });
}
async function evaluate(expression) {
  const response = await command('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw new Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
}
async function until(expression) {
  for (let index = 0; index < 120; index++) {
    if (await evaluate(expression)) return;
    await pause(35);
  }
  throw new Error(`Timed out: ${expression}`);
}
const scope = kind => `.model-config-panel[aria-labelledby="${kind}-model-heading"]`;
async function click(selector) {
  await evaluate(`document.querySelector(${JSON.stringify(selector)}).scrollIntoView({block:'center'})`);
  const point = await evaluate(`(() => {const el=document.querySelector(${JSON.stringify(selector)}),r=el.getBoundingClientRect(); if(el.disabled) throw new Error('disabled'); return {x:r.x+r.width/2,y:r.y+r.height/2};})()`);
  await command('Input.dispatchMouseEvent', { type: 'mousePressed', ...point, button: 'left', buttons: 1, clickCount: 1 });
  await command('Input.dispatchMouseEvent', { type: 'mouseReleased', ...point, button: 'left', buttons: 0, clickCount: 1 });
}
async function input(kind, field, value) {
  const selector = `${scope(kind)} ${field === 'model' ? '.model-manual-input' : field === 'baseUrl' ? 'input[type="url"]' : 'input[placeholder="sk-..."]'}`;
  await evaluate(`(() => { const input=document.querySelector(${JSON.stringify(selector)}); input.value=${JSON.stringify(value)}; input.dispatchEvent(new Event('input',{bubbles:true})); })()`);
}
const action = (kind, index) => `${scope(kind)} .model-actions button:nth-child(${index})`;
async function check(name, run) {
  try { await run(); results.push({name, status:'PASS'}); console.log(`PASS ${name}`); }
  catch(error) { results.push({name, status:'FAIL', error:error.message}); console.error(`FAIL ${name}: ${error.message}`); }
}
function installMocks() {
  window.calls = [];
  window.deferred = {};
  window.hold = {};
  window.rejectNext = {};
  window.settings = {theme:'light',ocrModel:{enabled:true,baseUrl:'https://ocr.example.test/v1',apiKey:'fake-ocr-key',model:''},polishModel:{enabled:true,baseUrl:'https://polish.example.test/v1',apiKey:'fake-polish-key',model:'polish-original'}};
  window.isTauri = true;
  window.__TAURI_INTERNALS__ = {
    metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback:()=>1,unregisterCallback:()=>{},convertFileSrc:value=>value,
    invoke:async(cmd,args={})=>{
      window.calls.push({cmd,args:structuredClone(args)});
      const kind=args.kind??(args.model?.apiKey.includes('ocr')?'ocr':'polish');
      const key=`${kind}:${cmd}`;
      if(window.hold[key]) await new Promise(resolve=>window.deferred[key]=resolve);
      if(window.rejectNext[key]) {delete window.rejectNext[key];throw {code:'MOCK_ERROR',message:'HTTP 401：API Key 无效，请检查后重试。'};}
      switch(cmd) {
        case 'get_app_settings':return structuredClone(window.settings);
        case 'set_theme_preference':return {theme:args.theme};
        case 'get_library_status':return {configured:true,libraryPath:'\\\\?\\D:\\ModelTestLibrary',ready:true,missingItems:[],articleCount:0};
        case 'list_articles':return [];
        case 'plugin:event|listen':return 1;
        case 'plugin:event|unlisten':return null;
        case 'fetch_model_list':return {models:['vision-alpha','vision-beta','vision-alpha']};
        case 'test_model':return kind==='ocr'?'SHILU 8246':'已整理网页资料，方便日后查找。';
        case 'save_single_model_settings':window.settings[kind==='ocr'?'ocrModel':'polishModel']=structuredClone(args.model);return structuredClone(window.settings);
        default:throw new Error(`Unmocked IPC: ${cmd}`);
      }
    },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
}
async function stopChrome() {
  if(!chrome||exited)return;
  if(socket?.readyState===WebSocket.OPEN) {command('Browser.close',{},true).catch(()=>{});await Promise.race([new Promise(resolve=>chrome.once('exit',resolve)),pause(2000)]);}
  if(!exited) {const process=spawn('taskkill',['/PID',String(chrome.pid),'/T','/F'],{windowsHide:true,stdio:'ignore'});await Promise.race([new Promise(resolve=>process.once('exit',resolve)),pause(2000)]);}
}
const deadline=setTimeout(()=>void stopChrome().finally(()=>process.exit(1)),90000);
try {
  await mkdir(output,{recursive:true});
  await fetch(testUrl,{signal:AbortSignal.timeout(5000)});
  chrome=spawn(process.env.CHROME_PATH??'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',['--headless=new','--remote-debugging-port=0',`--user-data-dir=${profile}`,'--no-first-run','--no-default-browser-check','--disable-extensions','--disable-background-networking','--disable-sync','about:blank'],{windowsHide:true,stdio:['ignore','ignore','pipe']});
  chrome.once('exit',()=>{exited=true;});
  const endpoint=await new Promise((resolve,reject)=>{
    let stderr='';const timer=setTimeout(()=>reject(new Error('Chrome startup timeout')),10000);
    chrome.once('error',error=>{clearTimeout(timer);reject(error);});
    chrome.stderr.on('data',data=>{stderr+=data;const match=stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);if(match){clearTimeout(timer);resolve(match[1]);}});
  });
  socket=new WebSocket(endpoint);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  socket.addEventListener('message',({data})=>{const message=JSON.parse(data);if(message.method==='Runtime.exceptionThrown')exceptions.push(message.params.exceptionDetails);const request=pending.get(message.id);if(!request)return;clearTimeout(request.timer);pending.delete(message.id);if(message.error)request.reject(new Error(message.error.message));else request.resolve(message.result);});
  const target=await command('Target.createTarget',{url:'about:blank'},true);
  ({sessionId}=await command('Target.attachToTarget',{targetId:target.targetId,flatten:true},true));
  await command('Page.enable');await command('Runtime.enable');
  await command('Page.addScriptToEvaluateOnNewDocument',{source:`(${installMocks.toString()})()`});
  await command('Emulation.setDeviceMetricsOverride',{width:1440,height:900,deviceScaleFactor:1,mobile:false});
  await command('Page.navigate',{url:`${testUrl}/#/settings`});
  await until(`document.querySelectorAll('.model-config-panel').length===2`);
  await pause(400);
  await until(`document.querySelectorAll('.model-config-panel').length===2`);
  await check('model list works with blank model; dropdown and manual input stay available',async()=>{
    assert.equal(await evaluate(`document.querySelector(${JSON.stringify(action('ocr',1))}).disabled`),false);
    await click(action('ocr',1));
    await until(`!!document.querySelector(${JSON.stringify(`${scope('ocr')} select`)})`);
    assert.equal(await evaluate(`document.querySelectorAll(${JSON.stringify(`${scope('ocr')} select option`)}).length`),3);
    assert.equal(await evaluate(`calls.find(c=>c.cmd==='fetch_model_list').args.model.model`),'');
    assert.equal(await evaluate(`!!document.querySelector(${JSON.stringify(`${scope('ocr')} .model-manual-input`)})`),true);
    await evaluate(`(()=>{const select=document.querySelector(${JSON.stringify(`${scope('ocr')} select`)});select.value='vision-beta';select.dispatchEvent(new Event('change',{bubbles:true}));})()`);
    assert.equal(await evaluate(`document.querySelector(${JSON.stringify(`${scope('ocr')} .model-manual-input`)}).value`),'vision-beta');
  });
  await check('OCR and polish test concurrently; second completion cannot clear first loading',async()=>{
    await evaluate(`hold['ocr:test_model']=true;hold['polish:test_model']=true`);
    await click(action('ocr',2));await click(action('polish',2));
    await until(`!!deferred['ocr:test_model']&&!!deferred['polish:test_model']`);
    assert.equal(await evaluate(`document.querySelectorAll('.model-actions button:disabled').length`),2);
    await evaluate(`deferred['polish:test_model']();hold['polish:test_model']=false`);
    await until(`document.querySelector(${JSON.stringify(scope('polish'))}).textContent.includes('测试成功')`);
    assert.equal(await evaluate(`document.querySelector(${JSON.stringify(action('ocr',2))}).textContent.trim()`),'测试中...');
    await evaluate(`deferred['ocr:test_model']();hold['ocr:test_model']=false`);
    await until(`document.querySelector(${JSON.stringify(scope('ocr'))}).textContent.includes('SHILU 8246')`);
    const tests=await evaluate(`calls.filter(c=>c.cmd==='test_model')`);
    assert.equal(tests[0].args.model.model,'vision-beta');
    assert.equal(tests[1].args.model.model,'polish-original');
  });
  await check('save targets one form and cannot discard newer edits',async()=>{
    await input('polish','model','unsaved-polish');
    await evaluate(`hold['ocr:save_single_model_settings']=true`);
    await click(action('ocr',3));await until(`!!deferred['ocr:save_single_model_settings']`);
    await input('ocr','model','newer-ocr');
    await evaluate(`deferred['ocr:save_single_model_settings']();hold['ocr:save_single_model_settings']=false`);
    await until(`document.querySelector(${JSON.stringify(scope('ocr'))}).textContent.includes('之后的修改尚未保存')`);
    assert.equal(await evaluate(`settings.polishModel.model`),'polish-original');
    assert.equal(await evaluate(`document.querySelector(${JSON.stringify(`${scope('ocr')} .model-manual-input`)}).value`),'newer-ocr');
    assert.equal(await evaluate(`document.querySelector(${JSON.stringify(`${scope('polish')} .model-manual-input`)}).value`),'unsaved-polish');
  });
  await check('stale model list is discarded after provider edit',async()=>{
    await evaluate(`hold['ocr:fetch_model_list']=true`);await click(action('ocr',1));await until(`!!deferred['ocr:fetch_model_list']`);
    await input('ocr','baseUrl','https://changed.example.test/v1');
    await evaluate(`deferred['ocr:fetch_model_list']();hold['ocr:fetch_model_list']=false`);
    await until(`document.querySelector(${JSON.stringify(scope('ocr'))}).textContent.includes('旧连接的列表未应用')`);
    assert.equal(await evaluate(`!!document.querySelector(${JSON.stringify(`${scope('ocr')} select`)})`),false);
  });
  await check('test errors preserve selection and retry shows returned text',async()=>{
    await evaluate(`rejectNext['ocr:test_model']=true`);await click(action('ocr',2));
    await until(`document.querySelector(${JSON.stringify(scope('ocr'))}).textContent.includes('HTTP 401')`);
    assert.equal(await evaluate(`document.querySelector(${JSON.stringify(`${scope('ocr')} .model-manual-input`)}).value`),'newer-ocr');
    await click(action('ocr',2));await until(`document.querySelector(${JSON.stringify(scope('ocr'))}).textContent.includes('newer-ocr 图片文字识别测试成功')`);
  });
  await check('same-form list and test are independent; late test identifies old configuration',async()=>{
    await evaluate(`delete deferred['ocr:test_model'];delete deferred['ocr:fetch_model_list'];hold['ocr:test_model']=true;hold['ocr:fetch_model_list']=true`);
    await click(action('ocr',2));await click(action('ocr',1));
    await until(`!!deferred['ocr:test_model']&&!!deferred['ocr:fetch_model_list']`);
    assert.equal(await evaluate(`document.querySelector(${JSON.stringify(action('ocr',1))}).disabled`),true);
    assert.equal(await evaluate(`document.querySelector(${JSON.stringify(action('ocr',2))}).disabled`),true);
    await input('ocr','model','ocr-after-test-start');
    await evaluate(`deferred['ocr:fetch_model_list']();hold['ocr:fetch_model_list']=false`);
    await until(`document.querySelector(${JSON.stringify(scope('ocr'))}).textContent.includes('已获取 2 个模型')`);
    assert.equal(await evaluate(`document.querySelector(${JSON.stringify(action('ocr',2))}).textContent.trim()`),'测试中...');
    await evaluate(`deferred['ocr:test_model']();hold['ocr:test_model']=false`);
    await until(`document.querySelector(${JSON.stringify(scope('ocr'))}).textContent.includes('此结果仅对应测试时的配置')`);
    assert.equal(await evaluate(`document.querySelector(${JSON.stringify(`${scope('ocr')} .model-manual-input`)}).value`),'ocr-after-test-start');
  });
  await check('both configuration saves remain pending independently',async()=>{
    await evaluate(`delete deferred['ocr:save_single_model_settings'];delete deferred['polish:save_single_model_settings'];hold['ocr:save_single_model_settings']=true;hold['polish:save_single_model_settings']=true`);
    await click(action('ocr',3));await click(action('polish',3));
    await until(`!!deferred['ocr:save_single_model_settings']&&!!deferred['polish:save_single_model_settings']`);
    await evaluate(`deferred['ocr:save_single_model_settings']();hold['ocr:save_single_model_settings']=false`);
    await until(`document.querySelector(${JSON.stringify(scope('ocr'))}).textContent.includes('OCR 识别模型配置已保存')`);
    assert.equal(await evaluate(`document.querySelector(${JSON.stringify(action('polish',3))}).textContent.trim()`),'保存中...');
    await evaluate(`deferred['polish:save_single_model_settings']();hold['polish:save_single_model_settings']=false`);
    await until(`document.querySelector(${JSON.stringify(scope('polish'))}).textContent.includes('文案润色模型配置已保存')`);
  });
  await check('layout fits desktop and narrow windows in light and dark',async()=>{
    for(const [width,height,theme] of [[1440,900,'light'],[1100,680,'light'],[760,680,'dark']]) {
      await command('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});
      await evaluate(`document.documentElement.dataset.theme=${JSON.stringify(theme)};document.querySelector('.model-settings-card').scrollIntoView({block:'start'})`);await pause(100);
      const metrics=await evaluate(`(()=>{const page=document.querySelector('.page-content');return {document:document.documentElement.scrollWidth,width:innerWidth,scroll:page.scrollWidth,client:page.clientWidth,fields:[...document.querySelectorAll('.model-fields input,.model-fields select,.model-actions button')].map(el=>({right:el.getBoundingClientRect().right,left:el.getBoundingClientRect().left}))};})()`);
      assert.ok(metrics.document<=width&&metrics.scroll<=metrics.client+1,JSON.stringify(metrics));
      assert.ok(metrics.fields.every(field=>field.left>=0&&field.right<=width+1),JSON.stringify(metrics));
      if(theme==='dark') assert.equal(await evaluate(`getComputedStyle(document.querySelector('.model-config-panel .settings-feedback-success')).color`),'oklch(0.76 0.13 150)');
      const image=await command('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
      await writeFile(path.join(output,`settings-${width}-${theme}.png`),Buffer.from(image.data,'base64'));
    }
  });
  await check('no unhandled frontend exceptions',async()=>assert.deepEqual(exceptions,[]));
  await writeFile(path.join(output,'report.json'),JSON.stringify({mode:'external Chrome; fake IPC only; no real API requests',results},null,2));
  console.log(`${results.filter(result=>result.status==='PASS').length}/${results.length} passed; ${output}`);
  if(results.some(result=>result.status==='FAIL'))process.exitCode=1;
} finally {
  clearTimeout(deadline);await stopChrome();socket?.close();for(const request of pending.values())clearTimeout(request.timer);
  if(path.dirname(profile)===path.resolve(tmpdir())&&path.basename(profile).startsWith('shilu-model-check-'))await rm(profile,{recursive:true,force:true,maxRetries:3,retryDelay:200});
}
