/** External Chrome regression, isolated fake IPC and generated clipboard fixtures only. */
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const testUrl = process.env.SHILU_TEST_URL ?? 'http://127.0.0.1:1421';
const profile = await mkdtemp(path.join(tmpdir(), 'shilu-article-check-'));
const output = path.join(project, 'work', 'article-workflow-check');
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
  for (let index = 0; index < 160; index++) { if (await evaluate(expression)) return; await pause(35); }
  throw new Error(`Timed out: ${expression}`);
}
async function click(selector) {
  await evaluate(`document.querySelector(${JSON.stringify(selector)}).scrollIntoView({block:'center',behavior:'instant'})`);
  const point = await evaluate(`(() => {const el=document.querySelector(${JSON.stringify(selector)}),r=el.getBoundingClientRect(); if(el.disabled) throw new Error('disabled: '+el.textContent);const x=r.x+r.width/2,y=r.y+r.height/2,hit=document.elementFromPoint(x,y);if(hit!==el&&!el.contains(hit))throw new Error('not clickable: '+el.textContent);return {x,y};})()`);
  await command('Input.dispatchMouseEvent', { type: 'mousePressed', ...point, button: 'left', buttons: 1, clickCount: 1 });
  await command('Input.dispatchMouseEvent', { type: 'mouseReleased', ...point, button: 'left', buttons: 0, clickCount: 1 });
}
async function clickText(text, root = '.page-view') {
  const selector = await evaluate(`(()=>{const candidates=[...document.querySelectorAll(${JSON.stringify(root+' button')})];const el=candidates.find(el=>el.textContent.trim()===${JSON.stringify(text)}&&el.getClientRects().length);if(!el)throw new Error('missing button '+${JSON.stringify(text)});el.dataset.testAction='target';return '[data-test-action="target"]';})()`);
  await click(selector); await evaluate(`document.querySelector('[data-test-action="target"]')?.removeAttribute('data-test-action')`);
}
async function fill(selector, value) {
  await evaluate(`(()=>{const el=document.querySelector(${JSON.stringify(selector)});el.value=${JSON.stringify(value)};el.dispatchEvent(new Event('input',{bubbles:true}));})()`);
}
async function route(route, selector) {
  await evaluate(`location.hash=${JSON.stringify('#'+route)}`);
  await until(`location.hash===${JSON.stringify('#'+route)}&&!!document.querySelector(${JSON.stringify(selector)})`);
  await pause(240);
}
async function paste(selector = 'body', position) {
  await evaluate(`(()=>{const el=document.querySelector(${JSON.stringify(selector)});el.focus();${position === undefined ? '' : `el.setSelectionRange(${position},${position});el.dispatchEvent(new Event('select'));`}const bytes=Uint8Array.from(atob(fixture.split(',')[1]),c=>c.charCodeAt(0)),data=new DataTransfer();data.items.add(new File([bytes],'clipboard.png',{type:'image/png'}));el.dispatchEvent(new ClipboardEvent('paste',{bubbles:true,cancelable:true,clipboardData:data}));})()`);
}
async function screenshot(name, width = 1440, height = 900, anchor) {
  await command('Emulation.setDeviceMetricsOverride', {width,height,deviceScaleFactor:1,mobile:false});
  await evaluate(anchor ? `document.querySelector(${JSON.stringify(anchor)}).scrollIntoView({block:'start',behavior:'instant'})` : `document.querySelector('.page-content').scrollTop=0`); await pause(100);
  const image = await command('Page.captureScreenshot', {format:'png',captureBeyondViewport:false});
  await writeFile(path.join(output,name+'.png'),Buffer.from(image.data,'base64'));
}
async function check(name, run) {
  try { await run(); results.push({name,status:'PASS'}); console.log(`PASS ${name}`); }
  catch(error) { const page=await evaluate(`document.querySelector('.page-view')?.innerText`).catch(()=>null);results.push({name,status:'FAIL',error:error.message,page}); console.error(`FAIL ${name}: ${error.message}`); }
}
function installMocks() {
  const canvas=document.createElement('canvas');canvas.width=560;canvas.height=210;const ctx=canvas.getContext('2d');
  ctx.fillStyle='#eaf3ef';ctx.fillRect(0,0,560,210);ctx.fillStyle='#254c46';ctx.font='bold 28px sans-serif';ctx.fillText('ShiLu / Screenshot collection',24,55);ctx.font='18px sans-serif';ctx.fillText('Collect a useful thought.',24,104);ctx.fillText('Save the source. Correct the text. Add context.',24,143);
  window.fixture=canvas.toDataURL('image/png');window.calls=[];window.hold={};window.deferred={};window.rejectNext={};window.rejectAll={};window.importCount={};
  const makeDoc=(folderName,title,status,captureStep,content='')=>({id:folderName,folderName,title,status,captureStep,content,summary:'',sourceUrl:'https://example.test/article',tags:['测试资料'],notes:'',createdAt:'2026-09-12T08:00:00Z',updatedAt:'2026-09-12T08:00:00Z',markdownPath:`D:/IsolatedWorkflow/articles/${folderName}/index.md`,sourceImages:['images/001.png']});
  window.docs={existing:makeDoc('existing','正式资料示例','active',3,'# 正式资料\n\n可供查询的内容。'),archived:makeDoc('archived','暂不适用的资料','archived',3,'# 已归档资料'),draft:makeDoc('draft','待补充信息的草稿','draft',2)};
  window.settings={theme:'light',ocrModel:{enabled:true,baseUrl:'https://ocr.example.test/v1',apiKey:'fake-ocr-key',model:'vision-test'},polishModel:{enabled:true,baseUrl:'https://polish.example.test/v1',apiKey:'fake-polish-key',model:'polish-test'}};
  const imports=(reference,sources)=>({articleId:reference,articleFolderName:reference,images:sources.map(source=>{const number=window.importCount[reference]=(window.importCount[reference]??1)+1;const fileName=String(number).padStart(3,'0')+'.png';return {fileName,originalFileName:source.name??'source.png',path:`D:/IsolatedWorkflow/articles/${reference}/images/${fileName}`,relativePath:`images/${fileName}`,extension:'png',sizeBytes:1000};})});
  window.isTauri=true;
  window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback:()=>1,unregisterCallback:()=>{},convertFileSrc:()=>window.fixture,
    invoke:async(cmd,args={})=>{
      // Tauri serializes argument objects through JSON, including Vue proxy arrays.
      args=JSON.parse(JSON.stringify(args));calls.push({cmd,args:structuredClone(args)});
      if(hold[cmd])await new Promise(resolve=>deferred[cmd]=resolve);
      if(rejectNext[cmd]||rejectAll[cmd]){delete rejectNext[cmd];throw {code:'MOCK_ERROR',message:cmd==='save_article'?'测试磁盘写入失败，请重试。':'测试 OCR 请求失败，请重试。'};}
      switch(cmd){
        case 'get_app_settings':return structuredClone(settings);
        case 'set_theme_preference':return {theme:args.theme};
        case 'get_library_status':return {configured:true,libraryPath:'D:/IsolatedWorkflow',ready:true,missingItems:[],articleCount:Object.values(docs).filter(doc=>doc.status==='active').length};
        case 'plugin:event|listen':return 1;
        case 'plugin:event|unlisten':return null;
        case 'plugin:fs|stat':return {size:1000};
        case 'read_clipboard_image':return {base64:fixture.split(',')[1]};
        case 'read_article':if(!docs[args.articleReference])throw new Error('missing article');return structuredClone(docs[args.articleReference]);
        case 'list_articles':return structuredClone(Object.values(docs).filter(doc=>doc.status===(args.status??'active')&&(!args.query||[doc.title,doc.content,doc.sourceUrl,...doc.tags].join(' ').includes(args.query))));
        case 'create_capture_draft':{const reference='new-draft';docs[reference]=makeDoc(reference,args.title||'未命名草稿','draft',1);docs[reference].sourceUrl=args.sourceUrl??'';docs[reference].sourceImages=[];importCount[reference]=0;const images=imports(reference,args.sources);return {article:{id:reference,folderName:reference,folderPath:`D:/IsolatedWorkflow/articles/${reference}`,markdownPath:docs[reference].markdownPath},images};}
        case 'import_article_sources':return imports(args.articleReference,args.sources);
        case 'save_article':Object.assign(docs[args.articleReference],structuredClone(args.draft));return structuredClone(docs[args.articleReference]);
        case 'set_article_status':docs[args.articleReference].status=args.status;return structuredClone(docs[args.articleReference]);
        case 'ocr_article_with_model':return {articleId:args.articleReference,text:'# OCR 提取结果\n\n这段文字需要校正。\n\n- 保留来源\n- 补充链接',imageCount:docs[args.articleReference].sourceImages.length,rawPath:'raw/ocr.txt'};
        case 'polish_with_model':return args.text.replace('手动校正后的正文','润色后的正文')+'\n\n整理完成。';
        default:throw new Error('Unmocked IPC: '+cmd);
      }
    }};
  window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
}
async function stopChrome() {
  if(!chrome||exited)return;
  if(socket?.readyState===WebSocket.OPEN){command('Browser.close',{},true).catch(()=>{});await Promise.race([new Promise(resolve=>chrome.once('exit',resolve)),pause(2000)]);}
  if(!exited){const child=spawn('taskkill',['/PID',String(chrome.pid),'/T','/F'],{windowsHide:true,stdio:'ignore'});await Promise.race([new Promise(resolve=>child.once('exit',resolve)),pause(2000)]);}
}
const deadline=setTimeout(()=>void stopChrome().finally(()=>process.exit(1)),120000);
try {
  await mkdir(output,{recursive:true});await fetch(testUrl,{signal:AbortSignal.timeout(5000)});
  chrome=spawn(process.env.CHROME_PATH??'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',['--headless=new','--remote-debugging-port=0',`--user-data-dir=${profile}`,'--no-first-run','--no-default-browser-check','--disable-extensions','--disable-background-networking','--disable-sync','about:blank'],{windowsHide:true,stdio:['ignore','ignore','pipe']});chrome.once('exit',()=>{exited=true;});
  const endpoint=await new Promise((resolve,reject)=>{let stderr='';const timer=setTimeout(()=>reject(new Error('Chrome startup timeout')),10000);chrome.once('error',error=>{clearTimeout(timer);reject(error);});chrome.stderr.on('data',data=>{stderr+=data;const match=stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);if(match){clearTimeout(timer);resolve(match[1]);}});});
  socket=new WebSocket(endpoint);await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  socket.addEventListener('message',({data})=>{const message=JSON.parse(data);if(message.method==='Runtime.exceptionThrown')exceptions.push(message.params.exceptionDetails);const request=pending.get(message.id);if(!request)return;clearTimeout(request.timer);pending.delete(message.id);if(message.error)request.reject(new Error(message.error.message));else request.resolve(message.result);});
  const target=await command('Target.createTarget',{url:'about:blank'},true);({sessionId}=await command('Target.attachToTarget',{targetId:target.targetId,flatten:true},true));await command('Page.enable');await command('Runtime.enable');
  await command('Page.addScriptToEvaluateOnNewDocument',{source:`(${installMocks.toString()})()`});await command('Emulation.setDeviceMetricsOverride',{width:1440,height:900,deviceScaleFactor:1,mobile:false});await command('Page.navigate',{url:`${testUrl}/#/capture`});await until(`!!document.querySelector('.capture-dropzone')`);
  await check('Step 1 clipboard source and Step 2 metadata survive back navigation',async()=>{
    await paste();await until(`document.querySelectorAll('.capture-queue-item').length===1`);await clickText('下一步：补充信息');
    await fill('.capture-field input[type="text"]','截图收集与文章整理');await fill('.capture-field input[type="url"]','https://example.test/resource');
    await clickText('上一步');assert.equal(await evaluate(`document.querySelectorAll('.capture-queue-item').length`),1);await screenshot('capture-step1');await clickText('下一步：补充信息');
    assert.equal(await evaluate(`document.querySelector('.capture-field input[type="text"]').value`),'截图收集与文章整理');await screenshot('capture-step2',1100,680);
  });
  await check('Draft save and resume restores title, link, screenshot and Step 2',async()=>{
    await clickText('保存草稿');await until(`document.querySelector('.capture-workspace').textContent.includes('已保存到草稿箱')`);
    await route('/drafts','.article-list');assert.equal(await evaluate(`document.querySelectorAll('.article-row').length`),2);await screenshot('draft-list');
    await click('.article-open[href*="new-draft"]');await until(`!!document.querySelector('.capture-details-panel')&&!document.querySelector('.capture-details-panel').style.display`);
    assert.equal(await evaluate(`document.querySelector('.capture-field input[type="text"]').value`),'截图收集与文章整理');assert.equal(await evaluate(`document.querySelector('.capture-field input[type="url"]').value`),'https://example.test/resource');assert.equal(await evaluate(`document.querySelectorAll('.capture-source-preview img').length`),1);
  });
  await check('OCR failure keeps inputs and retry writes editable Markdown directly',async()=>{
    await evaluate(`rejectNext.ocr_article_with_model=true`);await clickText('下一步：识别并编辑');await until(`document.querySelector('.capture-feedback-error')?.textContent.includes('OCR 请求失败')`);assert.ok((await evaluate(`location.hash`)).startsWith('#/capture'));assert.equal(await evaluate(`docs['new-draft'].sourceImages.length`),1);
    await screenshot('ocr-failure',1100,680);await clickText('下一步：识别并编辑');await until(`!!document.querySelector('.editor-content')`);assert.equal(await evaluate(`document.querySelector('.editor-content').value`),'# OCR 提取结果\n\n这段文字需要校正。\n\n- 保留来源\n- 补充链接');assert.equal(await evaluate(`docs['new-draft'].captureStep`),3);assert.equal(await evaluate(`!!document.querySelector('.editor-preview h1')`),true);await screenshot('ocr-editor');
  });
  await check('Polish uses latest edits and delayed result cannot overwrite newer edits',async()=>{
    await fill('.editor-content','# 手动校正后的正文\n\n修改了 OCR 错别字。');await evaluate(`hold.polish_with_model=true`);await clickText('AI 润色');await until(`!!deferred.polish_with_model`);assert.equal(await evaluate(`calls.filter(call=>call.cmd==='polish_with_model').at(-1).args.text`),'# 手动校正后的正文\n\n修改了 OCR 错别字。');
    await fill('.editor-content','# 手动校正后的正文\n\n润色过程中又补充了新内容。');await evaluate(`deferred.polish_with_model();hold.polish_with_model=false`);await until(`!!document.querySelector('.polish-columns')`);assert.equal(await evaluate(`[...document.querySelectorAll('.polish-actions button')].find(el=>el.textContent.includes('采用')).disabled`),true);assert.ok(await evaluate(`document.querySelector('.editor-content').value.includes('又补充了新内容')`));await screenshot('polish-stale');await screenshot('polish-stale-review',1440,900,'.polish-review');await clickText('放弃','.polish-review');
    await clickText('AI 润色');await until(`!!document.querySelector('.polish-columns')`);await clickText('采用润色结果','.polish-review');assert.ok(await evaluate(`document.querySelector('.editor-content').value.includes('润色后的正文')`));
  });
  await check('Clipboard illustration inserts at cursor, renders image and never calls OCR',async()=>{
    const before=await evaluate(`calls.filter(call=>call.cmd==='ocr_article_with_model').length`);await fill('.editor-content','前面的说明\n\n后面的补充');await paste('.editor-content',6);await until(`!!document.querySelector('.editor-preview img')&&document.querySelector('.editor-preview img').naturalWidth>0`);
    assert.equal(await evaluate(`document.querySelector('.editor-content').value`),'前面的说明\n\n\n![配图](images/002.png)\n\n\n后面的补充');assert.equal(await evaluate(`calls.filter(call=>call.cmd==='ocr_article_with_model').length`),before);assert.deepEqual(await evaluate(`docs['new-draft'].sourceImages`),['images/001.png']);await until(`docs['new-draft'].content.includes('images/002.png')`);await screenshot('editor-illustration');await screenshot('editor-illustration-narrow',1100,680);await screenshot('editor-preview-narrow',1100,680,'.editor-preview');
  });
  await check('Draft publication and archive/restore filter the correct article collections',async()=>{
    await clickText('保存到资料库');await until(`document.querySelector('.article-status').textContent==='已收录'`);assert.equal(await evaluate(`docs['new-draft'].status`),'active');await route('/drafts','.article-list');assert.equal(await evaluate(`document.querySelectorAll('.article-row').length`),1);await route('/library','.article-list');assert.equal(await evaluate(`document.querySelectorAll('.article-row').length`),2);
    await click('button[aria-label="归档：截图收集与文章整理"]');await until(`document.querySelectorAll('.article-row').length===1`);await route('/archive','.article-list');assert.equal(await evaluate(`document.querySelectorAll('.article-row').length`),2);await screenshot('archive-list');await click('button[aria-label="恢复：截图收集与文章整理"]');await until(`document.querySelectorAll('.article-row').length===1`);await route('/library','.article-list');assert.equal(await evaluate(`document.querySelectorAll('.article-row').length`),2);
  });
  await check('Save failure blocks leaving edited article and preserves body for retry',async()=>{
    await click('.article-open[href*="new-draft"]');await until(`!!document.querySelector('.editor-content')`);await fill('.editor-content','这一段尚未写入磁盘，不能丢失。');await evaluate(`rejectAll.save_article=true`);await click('.editor-back');await until(`document.querySelector('.editor-feedback[role="alert"]')?.textContent.includes('磁盘写入失败')`);assert.ok(await evaluate(`location.hash.includes('/articles/new-draft/edit')`));assert.equal(await evaluate(`document.querySelector('.editor-content').value`),'这一段尚未写入磁盘，不能丢失。');await screenshot('editor-save-failure');await evaluate(`rejectAll.save_article=false`);await clickText('重试保存');await until(`docs['new-draft'].content==='这一段尚未写入磁盘，不能丢失。'`);await click('.editor-back');await until(`location.hash==='#/library'`);
  });
  await check('Capture save failure blocks navigation until draft retry succeeds',async()=>{
    await route('/capture','.capture-dropzone');await paste();await until(`document.querySelectorAll('.capture-queue-item').length===1`);await evaluate(`rejectAll.save_article=true`);await click('.arc-nav-item[aria-label="草稿箱"]');await until(`document.querySelector('.capture-feedback-error')?.textContent.includes('磁盘写入失败')`);assert.equal(await evaluate(`location.hash`),'#/capture');assert.equal(await evaluate(`document.querySelectorAll('.capture-queue-item').length`),1);await evaluate(`rejectAll.save_article=false`);await clickText('保存草稿');await until(`document.querySelector('.capture-workspace').textContent.includes('已保存到草稿箱')`);
  });
  await check('Six arc entries switch by wheel in expanded/collapsed windows without overflow',async()=>{
    await route('/overview','.dashboard-view');assert.equal(await evaluate(`document.querySelectorAll('.arc-nav-item').length`),6);
    for(const [width,height] of [[1440,900],[1100,680]]){
      await command('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});
      if(await evaluate(`!!document.querySelector('.expand-button')`))await click('.expand-button');
      for(const collapsed of [false,true]){
        if(collapsed)await click('.sidebar-toggle');await route('/overview','.dashboard-view');
        for(const [index,expected] of ['capture','drafts','library','archive','settings'].entries()){
          const point=await evaluate(`(()=>{const r=document.querySelector('.arc-nav').getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2};})()`);await command('Input.dispatchMouseEvent',{type:'mouseWheel',...point,deltaX:0,deltaY:120});await until(`location.hash==='#/${expected}'`);await pause(240);assert.equal(await evaluate(`document.querySelector('.arc-nav-item[aria-current="page"]').dataset.navIndex`),String(index+1));
          const metrics=await evaluate(`(()=>{const el=document.querySelector('.page-content');return {doc:document.documentElement.scrollWidth,width:innerWidth,scroll:el.scrollWidth,client:el.clientWidth};})()`);assert.ok(metrics.doc<=width&&metrics.scroll<=metrics.client+1,JSON.stringify(metrics));
        }
        await route('/archive','.article-list');await screenshot(`navigation-${width}-${collapsed?'collapsed':'expanded'}`,width,height);
      }
    }
  });
  await check('No unhandled frontend exceptions',async()=>assert.deepEqual(exceptions,[]));
  await writeFile(path.join(output,'report.json'),JSON.stringify({mode:'external Chrome; in-memory IPC; generated clipboard fixture; no real API keys or user files',results},null,2));console.log(`${results.filter(result=>result.status==='PASS').length}/${results.length} passed; ${output}`);if(results.some(result=>result.status==='FAIL'))process.exitCode=1;
} finally {
  clearTimeout(deadline);await stopChrome();socket?.close();for(const request of pending.values())clearTimeout(request.timer);
  if(path.dirname(profile)===path.resolve(tmpdir())&&path.basename(profile).startsWith('shilu-article-check-'))await rm(profile,{recursive:true,force:true,maxRetries:3,retryDelay:200});
}
