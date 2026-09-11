/**
 * Navigation regression in an isolated, local Chrome instance (Node >= 22).
 * Start Vite first, then run: node scripts/check-navigation.mjs
 * Optional: SHILU_TEST_URL and CHROME_PATH environment variables.
 * Does not open Codex's browser or read the user's Chrome profile.
 */
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const testUrl = process.env.SHILU_TEST_URL ?? "http://127.0.0.1:1420";
const chromePath = process.env.CHROME_PATH ?? "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe";
const temporaryRoot = path.resolve(tmpdir());
const profile = await mkdtemp(path.join(temporaryRoot, "shilu-nav-regression-"));
const outputDirectory = path.join(project, "work");
const results = [];
const exceptions = [];
let chrome;
let socket;
let sessionId;
let sequence = 0;
let navigationSequence = 0;
let chromeExited = false;
const pending = new Map();
const pause = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

function command(method, params = {}, browser = false) {
  const id = ++sequence;
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`CDP timeout: ${method}`));
    }, 10000);
    pending.set(id, { resolve, reject, timeout });
    socket.send(JSON.stringify({ id, method, params, ...(!browser && sessionId ? { sessionId } : {}) }));
  });
}

async function evaluate(expression) {
  const result = await command("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
  if (result.exceptionDetails) throw new Error(result.exceptionDetails.text);
  return result.result.value;
}

async function until(predicate, description, timeout = 5000) {
  const deadline = Date.now() + timeout;
  do {
    if (await predicate()) return;
    await pause(25);
  } while (Date.now() < deadline);
  throw new Error(`Timed out waiting for ${description}`);
}

async function navigate(route = "overview") {
  // A unique document URL avoids hash-only navigation preserving sidebar state
  // from the preceding case, notably the collapsed and reduced-motion checks.
  await command("Page.navigate", { url: `${testUrl}/?navigationRegression=${++navigationSequence}#/${route}` });
  await until(() => evaluate(`!!document.querySelector('.arc-nav-item[data-nav-index="3"]')`), "navigation mount");
  await until(() => evaluate(`location.hash === '#/${route}'`), route);
  await pause(280);
}

async function state() {
  return evaluate(`({
    route: location.hash,
    selected: [...document.querySelectorAll('.arc-nav-item-selected')].map(el => Number(el.dataset.navIndex)),
    current: [...document.querySelectorAll('.arc-nav-item[aria-current="page"]')].map(el => Number(el.dataset.navIndex)),
    collapsed: document.querySelector('.app-shell').classList.contains('sidebar-is-collapsed')
  })`);
}

async function expectRoute(route, selectedIndex) {
  await until(() => evaluate(`location.hash === '#/${route}'`), `route ${route}`);
  const observed = await state();
  if (selectedIndex !== undefined) {
    assert.deepEqual(observed.selected, [selectedIndex], `highlight on ${route}`);
    assert.deepEqual(observed.current, [selectedIndex], `current page on ${route}`);
  }
  return observed;
}

async function center(selector) {
  const point = await evaluate(`(() => {
    const element = document.querySelector(${JSON.stringify(selector)});
    if (!element) return null;
    const bounds = element.getBoundingClientRect();
    const x = bounds.x + bounds.width / 2;
    const y = bounds.y + bounds.height / 2;
    const hit = document.elementFromPoint(x, y);
    return { x, y, hit: hit === element || element.contains(hit) || !!hit?.closest(${JSON.stringify(selector)}) };
  })()`);
  assert.ok(point, `missing element ${selector}`);
  assert.ok(point.hit, `element is not mouse-accessible: ${selector} ${JSON.stringify(point)}`);
  return { x: point.x, y: point.y };
}

async function click(selector) {
  const point = await center(selector);
  await command("Input.dispatchMouseEvent", { type: "mouseMoved", ...point });
  await command("Input.dispatchMouseEvent", { type: "mousePressed", ...point, button: "left", buttons: 1, clickCount: 1 });
  await command("Input.dispatchMouseEvent", { type: "mouseReleased", ...point, button: "left", buttons: 0, clickCount: 1 });
}

const clickNav = (index) => click(`.arc-nav-item[data-nav-index="${index}"] .arc-nav-icon`);

async function wheel(deltaY, modifiers = 0) {
  const point = await evaluate(`(() => {
    const bounds = document.querySelector('.arc-nav').getBoundingClientRect();
    return { x: bounds.x + bounds.width - 8, y: bounds.y + bounds.height / 2 };
  })()`);
  await command("Input.dispatchMouseEvent", { type: "mouseMoved", ...point });
  await command("Input.dispatchMouseEvent", { type: "mouseWheel", ...point, deltaX: 0, deltaY, modifiers });
}

async function key(key, code, virtualKeyCode) {
  await command("Input.dispatchKeyEvent", { type: "keyDown", key, code, windowsVirtualKeyCode: virtualKeyCode });
  await command("Input.dispatchKeyEvent", { type: "keyUp", key, code, windowsVirtualKeyCode: virtualKeyCode });
}

async function check(name, run) {
  try {
    const detail = await run();
    results.push({ name, status: "PASS", detail });
    console.log(`PASS ${name}${detail ? ` ${JSON.stringify(detail)}` : ""}`);
  } catch (error) {
    results.push({ name, status: "FAIL", error: error.message });
    console.error(`FAIL ${name}: ${error.message}`);
  }
}

async function screenshot(width, height) {
  await command("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  await navigate("library");
  assert.equal((await state()).collapsed, false, "screenshot must cover the expanded arc navigation");
  const layout = await evaluate(`(() => {
    const rect = selector => {
      const r = document.querySelector(selector).getBoundingClientRect();
      return { left: r.left, right: r.right, top: r.top, bottom: r.bottom };
    };
    return {
      viewport: { width: innerWidth, height: innerHeight },
      documentWidth: document.documentElement.scrollWidth,
      contentWidth: document.querySelector('.page-content').scrollWidth,
      contentClientWidth: document.querySelector('.page-content').clientWidth,
      sidebar: rect('.sidebar'), workspace: rect('.workspace'),
      nav: rect('.arc-nav'), brand: rect('.brand-row'), bottom: rect('.sidebar-bottom'),
      topbar: rect('.topbar'), page: rect('.page-content'),
      title: rect('.topbar-title'), actions: rect('.topbar-actions'),
      items: [...document.querySelectorAll('.arc-nav-item')].map(el => {
        const r = el.getBoundingClientRect();
        return { top: r.top, bottom: r.bottom };
      })
    };
  })()`);
  const image = await command("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
  const imagePath = path.join(outputDirectory, `navigation-regression-${width}x${height}.png`);
  await writeFile(imagePath, Buffer.from(image.data, "base64"));
  assert.ok(layout.documentWidth <= width, `document horizontal overflow: ${JSON.stringify(layout)}`);
  assert.ok(layout.contentWidth <= layout.contentClientWidth + 1, `content horizontal overflow: ${JSON.stringify(layout)}`);
  assert.ok(layout.sidebar.right <= layout.workspace.left + 1, "sidebar overlaps workspace");
  assert.ok(layout.brand.bottom <= layout.nav.top + 1, "brand overlaps navigation");
  assert.ok(layout.nav.bottom <= layout.bottom.top + 1, "navigation overlaps storage footer");
  assert.ok(layout.topbar.bottom <= layout.page.top + 1, "topbar overlaps page content");
  assert.ok(layout.title.right <= layout.actions.left + 1, "topbar title overlaps actions");
  for (const item of layout.items) {
    assert.ok(item.top >= layout.nav.top - 1 && item.bottom <= layout.nav.bottom + 1, "navigation item is vertically clipped");
  }
  return { image: imagePath, viewport: layout.viewport, horizontalOverflow: false };
}

async function terminateChrome() {
  if (!chrome || chromeExited) return;
  if (socket?.readyState === WebSocket.OPEN) {
    command("Browser.close", {}, true).catch(() => {});
    await Promise.race([new Promise(resolve => chrome.once("exit", resolve)), pause(2000)]);
  }
  if (!chromeExited && chrome.pid) {
    const killer = spawn("taskkill", ["/PID", String(chrome.pid), "/T", "/F"], { windowsHide: true, stdio: "ignore" });
    await Promise.race([new Promise(resolve => killer.once("exit", resolve)), pause(3000)]);
  }
}

const deadline = setTimeout(() => {
  console.error("Navigation regression exceeded its 90-second limit.");
  terminateChrome().finally(() => process.exit(1));
}, 90000);

try {
  await mkdir(outputDirectory, { recursive: true });
  await fetch(testUrl, { signal: AbortSignal.timeout(5000) });
  chrome = spawn(chromePath, [
    "--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`,
    "--no-first-run", "--no-default-browser-check", "--disable-extensions", "--disable-background-networking",
    "--disable-component-update", "--disable-sync", "--window-size=1440,900", "about:blank",
  ], { windowsHide: true, stdio: ["ignore", "ignore", "pipe"] });
  chrome.once("exit", () => { chromeExited = true; });
  const endpoint = await new Promise((resolve, reject) => {
    let stderr = "";
    const timeout = setTimeout(() => reject(new Error("Chrome did not expose CDP within 10 seconds")), 10000);
    chrome.once("error", (error) => { clearTimeout(timeout); reject(error); });
    chrome.stderr.on("data", (data) => {
      stderr += data;
      const match = stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (match) { clearTimeout(timeout); resolve(match[1]); }
    });
  });
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  socket.addEventListener("message", ({ data }) => {
    const message = JSON.parse(data);
    if (message.method === "Runtime.exceptionThrown") exceptions.push(message.params.exceptionDetails);
    const request = pending.get(message.id);
    if (!request) return;
    clearTimeout(request.timeout);
    pending.delete(message.id);
    if (message.error) request.reject(new Error(message.error.message));
    else request.resolve(message.result);
  });
  const target = await command("Target.createTarget", { url: "about:blank" }, true);
  ({ sessionId } = await command("Target.attachToTarget", { targetId: target.targetId, flatten: true }, true));
  await command("Page.enable");
  await command("Runtime.enable");
  await command("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });

  await check("wheel routes without Enter; upper boundary is clamped", async () => {
    await navigate();
    await expectRoute("overview", 0);
    await wheel(120);
    await expectRoute("library", 1);
    await wheel(-120);
    await expectRoute("overview", 0);
    await wheel(-120);
    await pause(300);
    return expectRoute("overview", 0);
  });

  await check("mouse click on non-centered capture item routes immediately", async () => {
    await navigate();
    await clickNav(2);
    return expectRoute("capture", 2);
  });

  await check("click overrides a pending wheel gesture; lower boundary is clamped", async () => {
    await navigate("library");
    await wheel(120);
    await clickNav(3);
    await expectRoute("settings", 3);
    await pause(450);
    await expectRoute("settings", 3);
    await wheel(120);
    await pause(300);
    return expectRoute("settings", 3);
  });

  await check("drag selects without a synthetic click; Enter still confirms", async () => {
    await navigate();
    // Focus a real button first; pointer capture for the drag starts only after moving.
    await clickNav(0);
    const point = await center('.arc-nav-item[data-nav-index="0"] .arc-nav-icon');
    await command("Input.dispatchMouseEvent", { type: "mousePressed", ...point, button: "left", buttons: 1, clickCount: 1 });
    for (let distance = 10; distance <= 80; distance += 10) {
      await command("Input.dispatchMouseEvent", { type: "mouseMoved", x: point.x, y: point.y - distance, buttons: 1 });
    }
    await command("Input.dispatchMouseEvent", { type: "mouseReleased", x: point.x, y: point.y - 80, button: "left", buttons: 0, clickCount: 1 });
    await pause(320);
    const dragged = await state();
    assert.equal(dragged.route, "#/overview", "drag navigated unexpectedly");
    assert.deepEqual(dragged.selected, [1], "drag failed to select library");
    await key("Enter", "Enter", 13);
    return { dragged, confirmed: await expectRoute("library", 1) };
  });

  await check("collapsed icon navigation remains mouse-clickable", async () => {
    await navigate();
    await click('button[aria-label="折叠侧边栏"]');
    await pause(260);
    assert.equal((await state()).collapsed, true);
    const transforms = await evaluate(`[...document.querySelectorAll('.arc-nav-item')].map(el => getComputedStyle(el).transform)`);
    assert.deepEqual(transforms, ["none", "none", "none", "none"]);
    await clickNav(2);
    return expectRoute("capture", 2);
  });

  await check("reduced-motion navigation remains mouse-clickable", async () => {
    await command("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
    try {
      await navigate();
      assert.equal((await state()).collapsed, false);
      assert.ok(await evaluate(`document.querySelector('.arc-nav').classList.contains('arc-nav-reduced')`));
      await clickNav(3);
      return await expectRoute("settings", 3);
    } finally {
      await command("Emulation.setEmulatedMedia", { features: [] });
    }
  });

  await check("Ctrl + wheel does not navigate", async () => {
    await navigate();
    await wheel(120, 2);
    await pause(350);
    return expectRoute("overview", 0);
  });

  await check("1440 x 900 screenshot and shell geometry", () => screenshot(1440, 900));
  await check("1100 x 680 screenshot and shell geometry", () => screenshot(1100, 680));
  await check("no uncaught JavaScript exceptions", async () => assert.deepEqual(exceptions, []));
} catch (error) {
  results.push({ name: "test infrastructure", status: "FAIL", error: error.stack });
  console.error(error);
} finally {
  await terminateChrome();
  socket?.close();
  for (const request of pending.values()) {
    clearTimeout(request.timeout);
    request.reject(new Error("CDP session closed"));
  }
  pending.clear();
  clearTimeout(deadline);
  // Only remove the exact, freshly created temporary profile, never a user profile.
  const relativeProfile = path.relative(temporaryRoot, path.resolve(profile));
  if (!relativeProfile.startsWith("..") && !path.isAbsolute(relativeProfile)
      && relativeProfile.startsWith("shilu-nav-regression-") && !relativeProfile.includes(path.sep)) {
    await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }).catch(error => {
      console.warn(`Could not remove temporary Chrome profile ${profile}: ${error.message}`);
    });
  }
}

const failed = results.filter(result => result.status === "FAIL").length;
console.log(`\n${results.length - failed}/${results.length} checks passed.`);
process.exitCode = failed ? 1 : 0;
