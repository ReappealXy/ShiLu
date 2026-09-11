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
    const image = await captureScreenshot(`navigation-failure-${results.length + 1}`).catch(() => null);
    results.push({ name, status: "FAIL", error: error.message, image });
    console.error(`FAIL ${name}: ${error.message}${image ? ` Screenshot: ${image}` : ""}`);
  }
}

async function captureScreenshot(name) {
  const image = await command("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
  const imagePath = path.join(outputDirectory, `${name}.png`);
  await writeFile(imagePath, Buffer.from(image.data, "base64"));
  return imagePath;
}

async function setCollapsed(collapsed) {
  if ((await state()).collapsed !== collapsed) {
    await click(`button[aria-label="${collapsed ? "折叠侧边栏" : "展开侧边栏"}"]`);
    await until(async () => (await state()).collapsed === collapsed, `sidebar collapsed=${collapsed}`);
    await pause(280);
  }
}

async function setTheme(theme) {
  if (await evaluate(`document.documentElement.dataset.theme !== ${JSON.stringify(theme)}`)) {
    await click(`button[aria-label="切换${theme === "dark" ? "深色" : "浅色"}主题"]`);
    await until(() => evaluate(`document.documentElement.dataset.theme === ${JSON.stringify(theme)}`), `${theme} theme`);
    await pause(160);
  }
}

async function collapsedGeometry() {
  const layout = await evaluate(`(() => {
    const rect = element => {
      const r = element.getBoundingClientRect();
      return { left: r.left, right: r.right, top: r.top, bottom: r.bottom,
        width: r.width, height: r.height, centerX: r.left + r.width / 2, centerY: r.top + r.height / 2 };
    };
    const sidebar = document.querySelector('.sidebar');
    const brandCopy = document.querySelector('.brand-copy');
    return {
      viewport: { width: innerWidth, height: innerHeight, deviceScaleFactor: devicePixelRatio },
      theme: document.documentElement.dataset.theme,
      collapsed: document.querySelector('.app-shell').classList.contains('sidebar-is-collapsed'),
      documentWidth: document.documentElement.scrollWidth,
      sidebar: rect(sidebar), brand: rect(document.querySelector('.brand-row')),
      brandMark: rect(document.querySelector('.brand-mark')),
      brandCopy: { ...rect(brandCopy), display: getComputedStyle(brandCopy).display },
      nav: rect(document.querySelector('.arc-nav')), bottom: rect(document.querySelector('.sidebar-bottom')),
      expand: rect(document.querySelector('.expand-button')),
      items: [...document.querySelectorAll('.arc-nav-item')].map(element => {
        const label = element.querySelector('.arc-nav-label');
        return { ...rect(element), icon: rect(element.querySelector('.arc-nav-icon')),
          transform: getComputedStyle(element).transform,
          label: { ...rect(label), display: getComputedStyle(label).display, text: label.textContent.trim() },
          accessibleName: element.getAttribute('aria-label') };
      })
    };
  })()`);
  const near = (actual, expected, message) => assert.ok(Math.abs(actual - expected) <= 1,
    `${message}: expected ${expected}, received ${actual}`);
  assert.equal(layout.collapsed, true, "geometry check requires a collapsed sidebar");
  assert.equal(layout.items.length, 4, "all four navigation entries must remain visible");
  near(layout.brand.height, 44, "brand row must not have hidden grid rows");
  assert.equal(layout.brandCopy.display, "none", "hidden brand text must not reserve layout space");
  near(layout.brandCopy.width, 0, "hidden brand text width");
  near(layout.brandCopy.height, 0, "hidden brand text height");
  near(layout.brandMark.centerX, layout.sidebar.centerX, "brand is centered in sidebar");
  near(layout.nav.top - layout.brand.bottom, 24, "fixed space below brand");
  near(layout.nav.height, 4 * 44 + 3 * 8, "navigation must not stretch to fill sidebar");
  for (const [index, item] of layout.items.entries()) {
    near(item.width, 44, `entry ${index} width`);
    near(item.height, 44, `entry ${index} height`);
    near(item.centerX, layout.sidebar.centerX, `entry ${index} centered in sidebar`);
    near(item.icon.centerX, item.centerX, `entry ${index} icon horizontally centered`);
    near(item.icon.centerY, item.centerY, `entry ${index} icon vertically centered`);
    assert.equal(item.transform, "none", `entry ${index} must not retain arc transforms`);
    assert.equal(item.label.display, "none", `entry ${index} text must not reserve layout space`);
    near(item.label.width, 0, `entry ${index} hidden label width`);
    near(item.label.height, 0, `entry ${index} hidden label height`);
    assert.equal(item.accessibleName, item.label.text, `entry ${index} keeps its accessible name`);
    if (index > 0) near(item.top - layout.items[index - 1].bottom, 8, `gap before entry ${index}`);
    assert.ok(item.top >= layout.nav.top - 1 && item.bottom <= layout.nav.bottom + 1,
      `entry ${index} must not be vertically clipped`);
  }
  near(layout.expand.centerX, layout.sidebar.centerX, "expand control centered in sidebar");
  near(layout.sidebar.bottom - layout.expand.bottom, 12, "expand control remains anchored at bottom");
  assert.ok(layout.nav.bottom <= layout.bottom.top, "navigation does not overlap expand control");
  assert.ok(layout.documentWidth <= layout.viewport.width, "collapsed layout must not overflow horizontally");
  return layout;
}

async function collapsedScreenshot({ width, height, deviceScaleFactor = 1 }, theme) {
  await command("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor, mobile: false });
  await navigate("overview");
  await setTheme(theme);
  await setCollapsed(true);
  const image = await captureScreenshot(`navigation-collapsed-${width}x${height}-${deviceScaleFactor}x-${theme}`);
  const layout = await collapsedGeometry();
  return { image, viewport: layout.viewport, theme, button: "44 x 44", gap: 8, brandHeight: layout.brand.height };
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
  const imagePath = await captureScreenshot(`navigation-regression-${width}x${height}`);
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

  await check("collapsed navigation: every icon is centered and mouse-clickable", async () => {
    await navigate();
    await setCollapsed(true);
    await collapsedGeometry();
    const visited = [];
    for (const [index, route] of ["overview", "library", "capture", "settings"].entries()) {
      await clickNav(index);
      visited.push(await expectRoute(route, index));
      await collapsedGeometry();
    }
    return { visited };
  });

  await check("collapsed navigation: wheel changes pages immediately", async () => {
    await navigate();
    await setCollapsed(true);
    const visited = [];
    for (const [index, route] of [[1, "library"], [2, "capture"], [3, "settings"]]) {
      await wheel(120);
      visited.push(await expectRoute(route, index));
      await pause(280);
    }
    await wheel(-120);
    visited.push(await expectRoute("capture", 2));
    await collapsedGeometry();
    return { visited };
  });

  await check("repeated collapse/expand preserves compact and arc layouts", async () => {
    await navigate("library");
    const expandedGeometry = () => evaluate(`[...document.querySelectorAll('.arc-nav-item')].map(element => {
      const r = element.getBoundingClientRect();
      return { left: r.left, top: r.top, width: r.width, height: r.height };
    })`);
    const initial = await expandedGeometry();
    for (let cycle = 0; cycle < 3; cycle += 1) {
      await setCollapsed(true);
      await collapsedGeometry();
      await expectRoute("library", 1);
      await setCollapsed(false);
      const restored = await expandedGeometry();
      for (const [index, item] of restored.entries()) {
        for (const property of ["left", "top", "width", "height"]) {
          assert.ok(Math.abs(item[property] - initial[index][property]) <= 1,
            `cycle ${cycle + 1}, arc entry ${index} ${property} was not restored`);
        }
      }
      assert.ok(await evaluate(`!!document.querySelector('.brand-copy').getBoundingClientRect().width`),
        "brand text must return after expanding");
      await expectRoute("library", 1);
    }
    return { cycles: 3, image: await captureScreenshot("navigation-expanded-after-collapse-cycles") };
  });

  await check("reduced-motion navigation remains mouse-clickable", async () => {
    await command("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
    try {
      await navigate();
      assert.equal((await state()).collapsed, false);
      assert.ok(await evaluate(`document.querySelector('.arc-nav').classList.contains('arc-nav-reduced')`));
      await clickNav(3);
      const expanded = await expectRoute("settings", 3);
      await setCollapsed(true);
      await collapsedGeometry();
      await clickNav(1);
      const collapsed = await expectRoute("library", 1);
      return { expanded, collapsed, image: await captureScreenshot("navigation-collapsed-reduced-motion") };
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
  for (const viewport of [
    { width: 1440, height: 900 },
    { width: 1100, height: 680 },
    { width: 1920, height: 1080 },
    // 1650 x 1020 physical pixels at Windows 150% maps to a 1100 x 680 CSS viewport.
    { width: 1100, height: 680, deviceScaleFactor: 1.5 },
  ]) {
    for (const theme of ["light", "dark"]) {
      await check(`collapsed ${viewport.width} x ${viewport.height} @ ${viewport.deviceScaleFactor ?? 1}x ${theme}`,
        () => collapsedScreenshot(viewport, theme));
    }
  }
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
