// Fetches your private Brightspace calendar feed, encrypts it, and writes
// _site/data/brightspace.enc.json for the app to decrypt in your browser.
//
// Needs two repository secrets:
//   BRIGHTSPACE_ICS_URL  the calendar subscription link from Brightspace
//   SYNC_PASSPHRASE      any long phrase; enter the same one in the app
//
// The feed is encrypted with AES-256-GCM (key from PBKDF2-SHA256, 200k rounds)
// so the published file is unreadable without your passphrase.
import { pbkdf2Sync, randomBytes, createCipheriv } from 'node:crypto';
import { writeFileSync, mkdirSync } from 'node:fs';

const url = process.env.BRIGHTSPACE_ICS_URL;
const pass = process.env.SYNC_PASSPHRASE;

if (!url || !pass) {
  console.log('Brightspace secrets not set; skipping sync. The app still works with manual .ics import.');
  process.exit(0);
}
if (pass.length < 8) {
  console.error('SYNC_PASSPHRASE must be at least 8 characters.');
  process.exit(1);
}

const res = await fetch(url.replace(/^webcal:/i, 'https:'), { headers: { 'User-Agent': 'ascension-system-sync' } });
if (!res.ok) {
  console.error(`Brightspace returned HTTP ${res.status}. Check that BRIGHTSPACE_ICS_URL is your current calendar feed link.`);
  process.exit(1);
}
const ics = await res.text();
if (!ics.includes('BEGIN:VCALENDAR')) {
  console.error('The feed did not return a calendar. Copy the link from Brightspace Calendar > Subscribe again.');
  process.exit(1);
}

const iter = 200000;
const salt = randomBytes(16);
const iv = randomBytes(12);
const key = pbkdf2Sync(pass, salt, iter, 32, 'sha256');
const cipher = createCipheriv('aes-256-gcm', key, iv);
const ct = Buffer.concat([cipher.update(ics, 'utf8'), cipher.final(), cipher.getAuthTag()]);
const events = (ics.match(/BEGIN:VEVENT/g) || []).length;

mkdirSync('_site/data', { recursive: true });
writeFileSync('_site/data/brightspace.enc.json', JSON.stringify({
  v: 1, iter,
  salt: salt.toString('base64'), iv: iv.toString('base64'), ct: ct.toString('base64'),
  at: new Date().toISOString()
}));
console.log(`Encrypted ${events} calendar events.`);
