// Parity test: the Rust engine (WebAssembly) must give exactly the same results
// as the UI's JavaScript engine, on hundreds of random saves and calendar feeds.
//
//   cargo run -p xtask -- frontend
//   node tests/parity.mjs [runs]
//
// Needs Playwright with Chromium (`npm i -D playwright && npx playwright install chromium`).
import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const dist = join(dirname(fileURLToPath(import.meta.url)), '..', 'app', 'dist');
const RUNS = Number(process.argv[2] || 300);

const server = createServer((req, res) => {
  const name = (req.url.split('?')[0].replace(/^\/+/, '') || 'index.html');
  try {
    const body = readFileSync(join(dist, name));
    res.writeHead(200, { 'content-type': name.endsWith('.js') ? 'text/javascript' : 'text/html' });
    res.end(body);
  } catch {
    res.writeHead(404);
    res.end();
  }
}).listen(0, '127.0.0.1');
await new Promise((r) => server.once('listening', r));
const url = `http://127.0.0.1:${server.address().port}/`;

const browser = await chromium.launch();
const page = await browser.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(e.message));
await page.goto(url);
await page.waitForFunction(() => window.AscensionApp && window.AscensionEngine, null, { timeout: 20000 });

const result = await page.evaluate((RUNS) => {
  // ---------- deterministic random data ----------
  let seed = 12345;
  const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  const ri = (a, b) => a + Math.floor(rnd() * (b - a + 1));
  const pick = (a) => a[Math.floor(rnd() * a.length)];
  const maybe = (p) => rnd() < p;
  const pad = (n) => String(n).padStart(2, '0');
  const dkey = (d) => d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());
  const now = new Date();
  const today = dkey(now);
  const ago = (n) => { const d = new Date(now); d.setDate(d.getDate() - n); return dkey(d); };
  const num = (x) => (maybe(0.1) ? String(x) : x);
  const STATS = ['STR', 'AGI', 'VIT', 'INT', 'WIL'];

  function randomState() {
    const span = ri(5, 160);
    // Some saves are very active so streaks, shields and weekly bosses get exercised too.
    const dense = maybe(0.35);
    const P = (p) => (dense ? Math.min(1, p + 0.45) : p);
    const quests = Array.from({ length: ri(0, 6) }, (_, i) => ({
      id: 'q' + i, name: 'Q' + i, diff: pick(['E', 'D', 'C', 'B', 'A', 'S']), stat: pick(STATS),
      days: [0, 1, 2, 3, 4, 5, 6].filter(() => maybe(0.7)), created: maybe(0.3) ? ago(ri(0, span)) : undefined,
    }));
    const skills = [{ id: 'sk_read', name: pick(['Reading', 'Guitar', 'read more']), stat: 'INT' }, { id: 'sk_x', name: 'Spanish', stat: 'INT' }];
    const classes = [{ id: 'c1', name: 'Calc' }, { id: 'c2', name: 'Chem' }];
    const hist = {};
    for (let i = 0; i < span; i++) {
      if (!maybe(P(0.75))) continue;
      const h = { p: {}, a: {} };
      quests.forEach((q) => { if (maybe(0.6)) h.a[q.id] = { xp: ri(10, 120), pts: Math.round(rnd() * 140) / 10, stat: maybe(0.05) ? 'LUCK' : q.stat, skill: maybe(0.3) ? pick(['sk_read', 'sk_x', '']) : '' }; });
      if (maybe(P(0.5))) { h.clr = true; if (maybe(0.9)) h.ca = { xp: 25, pts: Math.round(rnd() * 100) / 10 }; }
      if (maybe(P(0.4))) h.w = Array.from({ length: ri(1, 2) }, () => {
        const w = { type: pick(['strength', 'cardio', 'sport', 'mobility']), min: num(ri(0, 200)) };
        if (maybe(0.5)) w.ex = Array.from({ length: ri(0, 4) }, () => ({ sets: num(ri(1, 6)) })); else if (maybe(0.5)) w.ns = ri(0, 30);
        if (w.type === 'cardio' && maybe(0.7)) w.km = Math.round(rnd() * 250) / 10;
        return w;
      });
      if (maybe(0.15)) h.prx = 30 * ri(1, 3);
      if (maybe(P(0.5))) {
        if (maybe(0.85)) h.f = Array.from({ length: ri(1, 5) }, () => ({ k: num(ri(50, 1200)), p: num(Math.round(rnd() * 600) / 10), c: ri(0, 100), f: ri(0, 50) }));
        else h.fs = { k: ri(1200, 3500), p: ri(40, 200), c: 200, f: 60, n: ri(1, 6) };
        if (maybe(0.7)) h.tg = { k: ri(1500, 3200), p: ri(80, 200) };
      }
      if (maybe(0.3)) h.wt = Math.round((60 + rnd() * 40) * 10) / 10;
      if (maybe(0.4)) h.wa = ri(0, 12);
      if (maybe(P(0.4))) h.s = Array.from({ length: ri(1, 3) }, () => ({ min: num(ri(dense ? 40 : 5, 200)), cls: maybe(0.7) ? pick(['c1', 'c2']) : '', pomo: maybe(0.3) ? ri(1, 4) : undefined }));
      if (maybe(0.2)) h.fc = ri(1, 120);
      if (maybe(0.2)) h.rd = ri(1, 400);
      hist[ago(i)] = h;
    }
    const tasks = Array.from({ length: ri(0, 12) }, (_, i) => {
      const done = maybe(0.6), due = maybe(0.85) ? ago(ri(-20, span)) : '';
      const t = { id: 't' + i, title: 'T' + i, cls: maybe(0.7) ? pick(['c1', 'c2']) : '', due, done, diff: 'C' };
      if (done) { t.doneOn = ago(ri(0, span)); if (maybe(0.9)) t.award = { xp: ri(10, 90), pts: Math.round(rnd() * 140) / 10, stat: maybe(0.8) ? 'INT' : '', day: maybe(0.8) ? t.doneOn : '' }; }
      return t;
    });
    const coop = {};
    if (maybe(0.3)) coop[ago(ri(0, span))] = maybe(0.5) ? true : ago(ri(0, span));
    return {
      created: ago(span), bw: 50 + ri(0, 60), hcm: 150 + ri(0, 50), age: ri(14, 60), sex: pick(['m', 'f']),
      act: pick(['sedentary', 'light', 'moderate', 'very', 'athlete', 'unknown']), goal: pick(['cut', 'maintain', 'bulk']),
      rate: pick([0.25, 0.5, 0.75, 1]), kcalOverride: maybe(0.1) ? ri(1500, 3000) : 0, wkTarget: ri(1, 6), waterTarget: ri(4, 10),
      quests, skills, classes, tasks, hist, coop,
      books: maybe(0.3) ? [{ status: 'done', fin: ago(ri(0, span)) }, { status: 'reading' }] : [],
      buys: maybe(0.3) ? [{ cost: ri(10, 200) }, { cost: '5' }] : [],
    };
  }

  // ---------- comparison ----------
  const diffs = [];
  function same(a, b, path) {
    if (diffs.length > 20) return;
    if (typeof a === 'number' && typeof b === 'number') {
      if (Math.abs(a - b) > 1e-9 * Math.max(1, Math.abs(a))) diffs.push(`${path}: js=${a} rust=${b}`);
      return;
    }
    if (a === undefined && b === undefined) return;
    if (a === null || b === null || typeof a !== 'object' || typeof b !== 'object') {
      if (a !== b) diffs.push(`${path}: js=${JSON.stringify(a)} rust=${JSON.stringify(b)}`);
      return;
    }
    const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
    for (const k of keys) {
      if (a[k] === undefined && b[k] === undefined) continue;
      same(a[k], b[k], path + '.' + k);
    }
  }

  const app = window.AscensionApp;
  const E = window.AscensionEngine;

  // ---------- compute parity ----------
  let computed = 0, bossWins = 0, shieldsUsed = 0;
  for (let n = 0; n < RUNS && diffs.length === 0; n++) {
    app.setState(randomState());
    const S = app.state();
    const js = JSON.parse(JSON.stringify(app.computeJS()));
    const rs = E.compute(S, today);
    same(js, rs, `run${n}`);
    bossWins += js.cnt.bosses; shieldsUsed += js.streak.daily.used.length;
    computed++;
  }

  // ---------- Brightspace import parity ----------
  const SUMMARIES = ['Assignment 3 - Due', 'Midterm Exam 2 - Availability Ends', 'Midterm Exam 2 - Availability Starts', 'Quiz 5 - Due',
    'Lab 4 report', 'Lab 4 opens', 'Final exam', 'Unit test', 'Attestation form', 'Read chapter 4', 'Discussion post: week 3', 'Essay draft due',
    'Project milestone: due date', 'Problem set 7 – End', 'Start date for module', 'Reading response', 'Group project'];
  const LOCS = ['CHEM 1010 - General Chemistry', 'MATH 1210 - Calculus I', '', 'calc', 'Math 1210 - calculus i'];
  const ics = (d) => d.getUTCFullYear() + pad(d.getUTCMonth() + 1) + pad(d.getUTCDate());
  const icsT = (d) => ics(d) + 'T' + pad(d.getUTCHours()) + pad(d.getUTCMinutes()) + pad(d.getUTCSeconds());
  function randomFeed(n) {
    const lines = ['BEGIN:VCALENDAR', 'VERSION:2.0'];
    for (let i = 0; i < n; i++) {
      const d = new Date(now.getTime() + ri(-12, 30) * 864e5 + ri(0, 86399) * 1000);
      const when = pick([icsT(d) + 'Z', icsT(d), ics(d)]);
      lines.push('BEGIN:VEVENT');
      if (maybe(0.9)) lines.push('UID:evt-' + ri(1, 12) + '@d2l');
      lines.push('SUMMARY:' + pick(SUMMARIES));
      lines.push((maybe(0.8) ? 'DTSTART:' : 'DTEND;TZID=x:') + when);
      const loc = pick(LOCS);
      if (loc) lines.push(maybe(0.3) ? 'LOCATION:' + loc.slice(0, 6) + '\r\n ' + loc.slice(6) : 'LOCATION:' + loc);
      lines.push('END:VEVENT');
    }
    lines.push('END:VCALENDAR');
    return lines.join('\r\n');
  }
  const shape = (st) => {
    const cname = (id) => (st.classes.find((c) => c.id === id) || {}).name || id;
    return {
      classes: st.classes.map((c) => c.name).sort(),
      tasks: st.tasks.map((t) => [t.uid || t.id, t.title, cname(t.cls), t.type, t.due, t.diff, !!t.done, !!t.auto, t.src || ''].join('|')).sort(),
      bsCount: st.bsCount,
    };
  };
  let imported = 0;
  for (let n = 0; n < Math.ceil(RUNS / 3) && diffs.length === 0; n++) {
    const base = { ...randomState(), tasks: [], classes: [{ id: 'c1', name: 'Calc' }] };
    const feeds = [randomFeed(ri(0, 14)), randomFeed(ri(1, 14))];
    // JavaScript path
    app.setState(JSON.parse(JSON.stringify(base)));
    const keep = window.AscensionEngine;
    window.AscensionEngine = undefined;
    const jsMsgs = feeds.map((f) => app.importICS(f).msg);
    const jsShape = shape(app.state());
    window.AscensionEngine = keep;
    // Rust path
    app.setState(JSON.parse(JSON.stringify(base)));
    const rsMsgs = feeds.map((f) => app.importICS(f).msg);
    const rsShape = shape(app.state());
    same({ jsMsgs, ...jsShape }, { jsMsgs: rsMsgs, ...rsShape }, `import${n}`);
    imported++;
  }
  return { computed, imported, diffs, engine: E.version, bossWins, shieldsUsed };
}, RUNS);

await browser.close();
server.close();
if (errors.length) console.log('page errors:', errors.slice(0, 5));
console.log(`engine ${result.engine}: compared ${result.computed} saves (${result.bossWins} boss wins, ${result.shieldsUsed} shields used) and ${result.imported} calendar imports`);
if (result.diffs.length) {
  console.log('MISMATCHES:\n' + result.diffs.join('\n'));
  process.exit(1);
}
console.log('Rust and JavaScript engines agree.');
