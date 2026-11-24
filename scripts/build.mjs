// Wraps src/app.html in a full HTML document for GitHub Pages and writes _site/index.html.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';

const app = readFileSync(new URL('../src/app.html', import.meta.url), 'utf8');
const doc = `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover"><style>:root{color-scheme:light;padding-top:env(safe-area-inset-top,0px);padding-bottom:env(safe-area-inset-bottom,0px)}body{margin:0;font:14px system-ui,sans-serif}img{max-width:100%}[hidden]{display:none!important}</style></head><body>
${app}
</body></html>
`;
mkdirSync('_site/data', { recursive: true });
writeFileSync('_site/index.html', doc);
console.log('Built _site/index.html');
