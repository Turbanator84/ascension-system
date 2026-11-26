// Builds the GitHub Pages site into _site/:
//   index.html            src/app.html wrapped in a full document, plus PWA tags and cloud config
//   manifest.webmanifest  lets phones install it to the home screen
//   sw.js                 offline cache + push reminder handler
//   icon-*.png            app icons (drawn here, no image tools needed)
//
// Optional build-time config (GitHub → Settings → Secrets and variables → Actions → Variables):
//   SUPABASE_URL, SUPABASE_ANON_KEY  cloud sync, sign-in and parties (both are public by design)
//   VAPID_PUBLIC_KEY                 push reminders
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { deflateSync } from 'node:zlib';

const app = readFileSync(new URL('../src/app.html', import.meta.url), 'utf8');
const sw = readFileSync(new URL('../src/sw.js', import.meta.url), 'utf8');
const version = new Date().toISOString().replace(/\D/g, '').slice(0, 14);

const cfg = {
  sbUrl: (process.env.SUPABASE_URL || '').trim().replace(/\/+$/, ''),
  sbKey: (process.env.SUPABASE_ANON_KEY || '').trim(),
  vapid: (process.env.VAPID_PUBLIC_KEY || '').trim(),
};
const cfgTag = cfg.sbUrl && cfg.sbKey
  ? `<script>window.ASC_CONFIG=${JSON.stringify(cfg).replace(/</g, '\\u003c')};</script>`
  : '';

const head = [
  '<meta charset="utf-8">',
  '<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">',
  '<meta name="theme-color" content="#0d0f1e">',
  '<meta name="apple-mobile-web-app-capable" content="yes">',
  '<meta name="mobile-web-app-capable" content="yes">',
  '<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">',
  '<meta name="apple-mobile-web-app-title" content="Ascension">',
  '<link rel="manifest" href="manifest.webmanifest">',
  '<link rel="icon" type="image/png" sizes="192x192" href="icon-192.png">',
  '<link rel="apple-touch-icon" href="apple-touch-icon.png">',
  '<style>:root{color-scheme:dark;padding-top:env(safe-area-inset-top,0px);padding-bottom:env(safe-area-inset-bottom,0px)}body{margin:0;font:14px system-ui,sans-serif;background:#0d0f1e}img{max-width:100%}[hidden]{display:none!important}</style>',
  cfgTag,
].filter(Boolean).join('');

const doc = `<!doctype html><html lang="en"><head>${head}</head><body>
${app}
</body></html>
`;

const manifest = {
  name: 'Ascension System',
  short_name: 'Ascension',
  description: 'Leveling-system habit tracker for training, nutrition and school.',
  start_url: './#home',
  scope: './',
  display: 'standalone',
  orientation: 'portrait',
  background_color: '#0d0f1e',
  theme_color: '#0d0f1e',
  icons: [
    { src: 'icon-192.png', sizes: '192x192', type: 'image/png' },
    { src: 'icon-512.png', sizes: '512x512', type: 'image/png' },
    { src: 'icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
  ],
};

// ---------- tiny PNG writer + icon drawing ----------
const crcTable = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc32 = buf => {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
function chunk(type, data) {
  const len = Buffer.alloc(4); len.writeUInt32BE(data.length);
  const td = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(td));
  return Buffer.concat([len, td, crc]);
}
function png(size, px) {
  const raw = Buffer.alloc((size * 4 + 1) * size);
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0;
    for (let x = 0; x < size; x++) {
      const [r, g, b, a] = px(x + 0.5, y + 0.5);
      const o = y * (size * 4 + 1) + 1 + x * 4;
      raw[o] = r; raw[o + 1] = g; raw[o + 2] = b; raw[o + 3] = a;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0); ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; ihdr[9] = 6; ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}
// Point inside a regular hexagon (pointy top) of radius r centred at cx, cy.
function inHex(x, y, cx, cy, r) {
  const dx = Math.abs(x - cx), dy = Math.abs(y - cy);
  return dy <= r && dx <= r * Math.sqrt(3) / 2 && dx * 0.5 + dy * Math.sqrt(3) / 2 <= r * Math.sqrt(3) / 2;
}
// Supersampled icon: crimson field, bone hexagon, dark hexagon, crimson upward chevron.
function icon(size, pad) {
  const s = size, c = s / 2, R = s * (0.5 - pad);
  const BG = [125, 13, 28], BONE = [241, 239, 233], INK = [13, 15, 30], RED = [214, 40, 57];
  const shade = (x, y) => {
    if (!inHex(x, y, c, c, R * 0.86)) return BG;
    if (!inHex(x, y, c, c, R * 0.74)) return BONE;
    const u = (x - c) / R, v = (y - c) / R;
    const chev = (yy, w) => Math.abs(u) <= w && v >= yy + Math.abs(u) * 0.9 && v <= yy + 0.16 + Math.abs(u) * 0.9;
    if (chev(-0.3, 0.42) || chev(0.02, 0.42)) return RED;
    return INK;
  };
  return png(s, (x, y) => {
    let r = 0, g = 0, b = 0;
    for (const [ox, oy] of [[-0.25, -0.25], [0.25, -0.25], [-0.25, 0.25], [0.25, 0.25]]) {
      const p = shade(x + ox, y + oy); r += p[0]; g += p[1]; b += p[2];
    }
    return [r / 4, g / 4, b / 4, 255];
  });
}

mkdirSync('_site/data', { recursive: true });
writeFileSync('_site/index.html', doc);
writeFileSync('_site/manifest.webmanifest', JSON.stringify(manifest, null, 2));
writeFileSync('_site/sw.js', sw.replaceAll('__VERSION__', version));
writeFileSync('_site/icon-192.png', icon(192, 0.02));
writeFileSync('_site/icon-512.png', icon(512, 0.02));
writeFileSync('_site/icon-maskable-512.png', icon(512, 0.12));
writeFileSync('_site/apple-touch-icon.png', icon(180, 0.04));
console.log(`Built _site (version ${version})${cfgTag ? ' with cloud sync config' : ' without cloud sync config'}.`);
