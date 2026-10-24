// perf/cdp-probe.mjs — mide paint, FPS y métricas de tarea del webview vía DevTools Protocol.
// Uso: node perf/cdp-probe.mjs [--port 9222] [--ms 5000]
// Requiere la app arrancada con WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<port>
// Autolimitado: si el target no responde, sale con codigo 2 en vez de colgarse.

const args = process.argv.slice(2);
const arg = (name, def) => {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1] : def;
};
const PORT = Number(arg('--port', '9222'));
const MS = Number(arg('--ms', '5000'));
const HARD_DEADLINE_MS = Number(arg('--deadline', '30000'));

const hardTimer = setTimeout(() => {
  process.stdout.write(JSON.stringify({ error: 'probe hard deadline ' + HARD_DEADLINE_MS + 'ms' }) + '\n');
  process.exit(2);
}, HARD_DEADLINE_MS);
hardTimer.unref?.();

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function pickTarget() {
  const ctrl = new AbortController();
  const t = setTimeout(() => ctrl.abort(), 1500);
  try {
    const res = await fetch('http://127.0.0.1:' + PORT + '/json/list', { signal: ctrl.signal });
    const list = await res.json();
    const pages = list.filter((x) => x.type === 'page' && x.webSocketDebuggerUrl);
    return pages.find((x) => /whatsapp/i.test(x.url)) || pages[0] || null;
  } finally {
    clearTimeout(t);
  }
}

function rpc(ws) {
  let id = 0;
  const pending = new Map();
  ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject, timer } = pending.get(msg.id);
      clearTimeout(timer);
      pending.delete(msg.id);
      if (msg.error) reject(new Error(JSON.stringify(msg.error)));
      else resolve(msg.result);
    }
  });
  return (method, params = {}, timeoutMs = 6000) =>
    new Promise((resolve, reject) => {
      const myId = ++id;
      const timer = setTimeout(() => {
        pending.delete(myId);
        reject(new Error('rpc timeout: ' + method));
      }, timeoutMs);
      pending.set(myId, { resolve, reject, timer });
      ws.send(JSON.stringify({ id: myId, method, params }));
    });
}

const evaluate = (send) => async (expression, timeoutMs = 12000) => {
  const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, timeoutMs);
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.text);
  return r.result.value;
};

async function main() {
  const started = Date.now();
  let target = null;
  while (Date.now() - started < 15000) {
    try {
      target = await pickTarget();
      if (target) break;
    } catch {}
    await sleep(150);
  }
  if (!target) throw new Error('no CDP page target on port ' + PORT);

  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('ws open timeout')), 6000);
    ws.addEventListener('open', () => { clearTimeout(timer); resolve(); }, { once: true });
    ws.addEventListener('error', (e) => { clearTimeout(timer); reject(new Error('ws error')); }, { once: true });
  });

  const send = rpc(ws);
  await send('Runtime.enable', {}, 6000);
  await send('Performance.enable', {}, 6000);
  const ev = evaluate(send);

  const nav = await ev("(() => {" +
    "const n = performance.getEntriesByType('navigation')[0] || {};" +
    "const paint = performance.getEntriesByType('paint') || [];" +
    "return {" +
    " url: location.href," +
    " readyState: document.readyState," +
    " domContentLoaded: n.domContentLoadedEventEnd ?? null," +
    " loadEvent: n.loadEventEnd ?? null," +
    " responseEnd: n.responseEnd ?? null," +
    " firstPaint: (paint.find((p) => p.name === 'first-paint') || {}).startTime ?? null," +
    " firstContentfulPaint: (paint.find((p) => p.name === 'first-contentful-paint') || {}).startTime ?? null," +
    " visibility: document.visibilityState," +
    " nodes: document.getElementsByTagName('*').length" +
    "};})()");

  const rafExpr = "new Promise((resolve) => {" +
    "const frames = [];" +
    "let last = performance.now();" +
    "const t0 = last;" +
    "function tick(now) {" +
    "  frames.push(now - last); last = now;" +
    "  if (now - t0 < " + MS + ") requestAnimationFrame(tick);" +
    "  else {" +
    "    const sorted = frames.slice(1).sort((a, b) => a - b);" +
    "    const sum = sorted.reduce((a, b) => a + b, 0);" +
    "    const p = (q) => sorted.length ? sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * q))] : null;" +
    "    resolve({ frames: sorted.length, durationMs: Math.round(now - t0), avgFps: +(sorted.length / ((now - t0) / 1000)).toFixed(2)," +
    "      p50FrameMs: p(0.5), p95FrameMs: p(0.95), p99FrameMs: p(0.99)," +
    "      maxFrameMs: sorted.length ? sorted[sorted.length - 1] : null," +
    "      over50ms: sorted.filter((d) => d > 50).length, meanFrameMs: sorted.length ? +(sum / sorted.length).toFixed(2) : null });" +
    "  }" +
    "}" +
    "requestAnimationFrame(tick);" +
    "})";
  const raf = await ev(rafExpr, MS + 10000);

  const metricsRaw = await send('Performance.getMetrics', {}, 6000);
  const metrics = Object.fromEntries(metricsRaw.metrics.map((m) => [m.name, m.value]));
  const pick = ['TaskDuration','ScriptDuration','LayoutDuration','RecalcStyleDuration','JSHeapUsedSize','JSHeapTotalSize','Documents','Frames','Nodes'];
  const out = {
    target: { url: target.url, title: target.title },
    nav,
    raf,
    metrics: Object.fromEntries(pick.filter((k) => k in metrics).map((k) => [k, metrics[k]])),
  };
  try { ws.close(); } catch {}
  return out;
}

main()
  .then((out) => { process.stdout.write(JSON.stringify(out) + '\n'); process.exit(0); })
  .catch((err) => {
    process.stdout.write(JSON.stringify({ error: String((err && err.message) || err) }) + '\n');
    process.exit(1);
  });
