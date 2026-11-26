// Sends push reminders. Runs hourly from .github/workflows/reminders.yml.
// For each device that turned reminders on, it checks the user's local time and their digest
// (written by the app on every save) and sends at most one of each kind per day:
//   evening  20:00–20:59  "2 quests left, resets in 3h"
//   due      18:00–18:59  "Quiz 5 due tomorrow"
//   report   Sunday 19:00–19:59  "Your weekly report is ready"
//
// Needs secrets: SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY, VAPID_PUBLIC_KEY, VAPID_PRIVATE_KEY.
// Optional: VAPID_SUBJECT (a mailto: or https: contact for push services).
import webpush from 'web-push';

const { SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY, VAPID_PUBLIC_KEY, VAPID_PRIVATE_KEY } = process.env;
if (!SUPABASE_URL || !SUPABASE_SERVICE_ROLE_KEY || !VAPID_PUBLIC_KEY || !VAPID_PRIVATE_KEY) {
  console.log('Reminder secrets not set; skipping.');
  process.exit(0);
}
webpush.setVapidDetails(process.env.VAPID_SUBJECT || 'https://github.com/ascension-system', VAPID_PUBLIC_KEY, VAPID_PRIVATE_KEY);

const base = SUPABASE_URL.replace(/\/+$/, '') + '/rest/v1';
const headers = { apikey: SUPABASE_SERVICE_ROLE_KEY, Authorization: `Bearer ${SUPABASE_SERVICE_ROLE_KEY}`, 'Content-Type': 'application/json' };
const api = async (path, opts = {}) => {
  const r = await fetch(base + path, { ...opts, headers: { ...headers, ...(opts.headers || {}) } });
  if (!r.ok) throw new Error(`${opts.method || 'GET'} ${path}: HTTP ${r.status} ${await r.text()}`);
  return r.status === 204 ? null : r.json().catch(() => null);
};

const subs = await api('/push_subs?select=endpoint,user_id,sub,tz,sent');
if (!subs.length) { console.log('No devices subscribed.'); process.exit(0); }
const ids = [...new Set(subs.map(s => s.user_id))];
const rows = await api(`/ascension_state?select=user_id,digest&user_id=in.(${ids.join(',')})`);
const digests = Object.fromEntries(rows.map(r => [r.user_id, r.digest || {}]));

function localNow(tz) {
  let parts;
  try {
    parts = new Intl.DateTimeFormat('en-CA', { timeZone: tz, year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', weekday: 'short', hourCycle: 'h23' }).formatToParts(new Date());
  } catch { return localNow('UTC'); }
  const g = t => parts.find(p => p.type === t).value;
  return { date: `${g('year')}-${g('month')}-${g('day')}`, hour: +g('hour'), minute: +g('minute'), dow: g('weekday') };
}
const addDays = (k, n) => { const d = new Date(k + 'T12:00:00Z'); d.setUTCDate(d.getUTCDate() + n); return d.toISOString().slice(0, 10); };

let sent = 0, removed = 0;
for (const s of subs) {
  const dg = digests[s.user_id];
  if (!dg) continue;
  const now = localNow(s.tz || dg.tz || 'UTC');
  const done = { ...(s.sent || {}) };
  const msgs = [];

  if (now.hour === 20 && done.evening !== now.date) {
    const left = dg.date === now.date ? dg.left : (dg.sched && dg.sched[now.date]) || 0;
    if (left > 0) {
      const h = 24 - now.hour - (now.minute > 30 ? 1 : 0);
      msgs.push(['evening', { title: `${left} quest${left === 1 ? '' : 's'} left`, body: `Resets in ${h}h.${dg.streak ? ` Keep your ${dg.streak}-day streak alive.` : ''}`, tab: 'home', tag: 'quests' }]);
    }
  }
  if (now.hour === 18 && done.due !== now.date) {
    const tmr = addDays(now.date, 1);
    const due = (dg.due || []).filter(x => x.d === tmr && x.y !== 'Review');
    if (due.length) {
      const first = due[0].t;
      msgs.push(['due', { title: `${first} due tomorrow`, body: due.length > 1 ? `And ${due.length - 1} more due tomorrow.` : 'Get it done tonight for the early bonus.', tab: 'school', tag: 'due' }]);
    }
  }
  if (now.dow === 'Sun' && now.hour === 19 && done.report !== now.date) {
    const boss = (dg.boss || []).length ? ` Boss still needs: ${dg.boss.join(', ')}.` : '';
    msgs.push(['report', { title: 'Your weekly report is ready', body: `See what improved, what slipped and your best day.${boss}`, tab: 'home', tag: 'report' }]);
  }
  if (!msgs.length) continue;

  let gone = false;
  for (const [kind, payload] of msgs) {
    try {
      await webpush.sendNotification(s.sub, JSON.stringify(payload), { TTL: 3600 });
      done[kind] = now.date; sent++;
    } catch (e) {
      if (e.statusCode === 404 || e.statusCode === 410) { gone = true; break; }
      console.error(`Push failed (${e.statusCode || e.message})`);
    }
  }
  const ep = encodeURIComponent(s.endpoint);
  if (gone) { await api(`/push_subs?endpoint=eq.${ep}`, { method: 'DELETE' }); removed++; }
  else await api(`/push_subs?endpoint=eq.${ep}`, { method: 'PATCH', body: JSON.stringify({ sent: done }), headers: { Prefer: 'return=minimal' } });
}
console.log(`Sent ${sent} reminder${sent === 1 ? '' : 's'}; removed ${removed} expired device${removed === 1 ? '' : 's'}.`);
