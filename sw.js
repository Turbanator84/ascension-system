// Service worker: keeps the app usable offline and shows push reminders.
// build.mjs replaces 20261128115054 on every deploy so phones pick up new versions.
const VERSION = '20261128115054';
const SHELL = `ascension-shell-${VERSION}`;
const RUNTIME = 'ascension-runtime';
const CORE = ['./', 'index.html', 'manifest.webmanifest', 'icon-192.png', 'icon-512.png', 'apple-touch-icon.png'];
// Third-party files that are safe to keep for offline use (fonts and pinned libraries).
const CACHEABLE = /^https:\/\/(fonts\.googleapis\.com|fonts\.gstatic\.com|cdn\.jsdelivr\.net)\//;

self.addEventListener('install', e => {
  e.waitUntil(caches.open(SHELL).then(c => c.addAll(CORE)).then(() => self.skipWaiting()));
});

self.addEventListener('activate', e => {
  e.waitUntil(
    caches.keys()
      .then(keys => Promise.all(keys.filter(k => k.startsWith('ascension-shell-') && k !== SHELL).map(k => caches.delete(k))))
      .then(() => self.clients.claim())
  );
});

self.addEventListener('fetch', e => {
  const req = e.request;
  if (req.method !== 'GET') return;
  const url = new URL(req.url);

  // The app page: try the network so updates arrive, fall back to the cached copy offline.
  if (req.mode === 'navigate') {
    e.respondWith(
      fetch(req)
        .then(res => { const copy = res.clone(); caches.open(SHELL).then(c => c.put('index.html', copy)); return res; })
        .catch(() => caches.match('index.html').then(r => r || caches.match('./')))
    );
    return;
  }

  if (url.origin === location.origin) {
    // Brightspace feed: always fresh when online, last copy when offline.
    if (url.pathname.includes('/data/')) {
      e.respondWith(fetch(req).then(res => { const copy = res.clone(); caches.open(RUNTIME).then(c => c.put(req, copy)); return res; }).catch(() => caches.match(req)));
      return;
    }
    e.respondWith(caches.match(req).then(hit => hit || fetch(req)));
    return;
  }

  if (CACHEABLE.test(req.url)) {
    e.respondWith(
      caches.open(RUNTIME).then(c => c.match(req).then(hit => {
        const net = fetch(req).then(res => { if (res.ok || res.type === 'opaque') c.put(req, res.clone()); return res; }).catch(() => hit);
        return hit || net;
      }))
    );
  }
  // Everything else (Supabase, Open Food Facts) goes straight to the network.
});

self.addEventListener('push', e => {
  let d = {};
  try { d = e.data ? e.data.json() : {}; } catch (_) { d = { body: e.data && e.data.text() }; }
  e.waitUntil(self.registration.showNotification(d.title || 'Ascension', {
    body: d.body || '',
    icon: 'icon-192.png',
    badge: 'icon-192.png',
    tag: d.tag || 'ascension',
    data: { tab: d.tab || 'home' },
  }));
});

self.addEventListener('notificationclick', e => {
  e.notification.close();
  const tab = (e.notification.data && e.notification.data.tab) || 'home';
  e.waitUntil(
    self.clients.matchAll({ type: 'window', includeUncontrolled: true }).then(list => {
      for (const c of list) { c.postMessage({ tab }); return c.focus(); }
      return self.clients.openWindow(`./#${tab}`);
    })
  );
});
