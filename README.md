# Ascension System

A leveling-system habit tracker with two looks you can switch between in Profile (or with the ⇄ button at the top): **Sorcerer** (crimson ink, Grade 5 → Special Grade) and **System** (blue status window, E → SS+). Daily quests, training, nutrition and school each level up on their own, and five attributes (STR, AGI, VIT, INT, WIL) are graded like sorcerers: **Grade 5 → Grade 4 → Grade 3 → Semi-Grade 2 → Grade 2 → Semi-Grade 1 → Grade 1 → Special Grade**.

## Tabs

| Tab | What it does |
| --- | --- |
| **Home** | Hunter card (level, rank, streak, shields, coins), reset countdown, path tiles, weekly boss, daily quests, upcoming deadlines, Sunday report |
| **Train** | Log workouts; saved routines with next-weight suggestions; rest timer and plate calculator; per-lift progress charts; benchmarks and rank ladders |
| **Fuel** | Calorie and macro log; Open Food Facts search and barcode scanning; saved meals; water, weigh-ins, cut / maintain / bulk targets |
| **School** | Assignments (synced from Brightspace), study timer with pomodoro mode, grade tracker, exam prep with flashcards, book tracker, class ranks, skills |
| **Calendar** | Month view of everything you did, plus due dates; tap a day for details |
| **Stats** | Attribute radar, path levels, streaks, charts, weekly reports and achievements |
| **Shop** (tap your coins) | Spend coins on real-life rewards you set yourself |
| **Party** (from Profile) | Shared weekly leaderboard, each other's streaks, and a co-op boss |

## Features in more detail

- **Undo and editing.** Checking something off, logging, or deleting shows an Undo bar for 7 seconds. Food, workouts, study sessions, tasks, scores, books and routines have an edit button.
- **Streak shields.** Every 7 days in a row earns a shield (hold up to 3). A missed day uses one instead of resetting your daily streak.
- **Weekly boss.** Each week brings a harder challenge, e.g. 4 workouts + 5 h study + 5 days on your calorie target. Beat it for +150 XP and 50 coins.
- **Coins and reward shop.** 1 coin per 10 XP, plus boss bonuses. Set your own rewards ("1 h of gaming", "cheat meal") and redeem them.
- **Weekly report.** On Sundays the Home screen shows what improved, what slipped and your best day. Any week can be opened from Stats.
- **Grade tracker.** Enter scores as you get them back, with optional category weights from your syllabus. It shows your current grade and what you need on the final to hit your target.
- **Exam prep.** Tests get review tasks 7, 4, 2 and 1 days before; quizzes 2 and 1 days before. Flashcards per class use spaced repetition (1, 2, 4, 8, 16, 32 days); missed cards repeat in the same session, and cards are pulled in before an upcoming test.
- **Pomodoro.** 25 minutes on, 5 off (adjustable). Each full round is logged automatically with +10 bonus XP.
- **Books.** Track what you're reading, want to read and have finished. Pages read and finished books earn XP.
- **Routines.** Save "Push / Pull / Legs" once and start it in one tap. If you hit all your reps last time it suggests the next weight ("you did 80 kg × 5 last time, try 82.5").
- **Food search and barcodes.** Uses [Open Food Facts](https://world.openfoodfacts.org), a free food database. Scanning uses the camera (BarcodeDetector where available, ZXing elsewhere, including iPhone).

## Desktop app (Rust)

There's also a native **Windows, macOS and Linux** app in [`desktop/`](desktop/README.md). Its game engine is written in Rust, it saves to a file on your computer with daily backups, fetches your Brightspace feed directly (no GitHub secrets needed), and sends reminder notifications. Installers are built by the **Desktop app (Rust)** workflow; see [desktop/README.md](desktop/README.md).

## How ranking works

- **STR / AGI / VIT** come from real benchmarks (bench, squat, deadlift, overhead press, push-ups, pull-ups, 5K, mile, plank, resting heart rate). They're scaled to your bodyweight and sex. Until you log a benchmark, these stay provisional and can't go above Grade 3. Strength sets of 12 reps or fewer update your records automatically with an estimated one-rep max (Epley formula).
- **INT / WIL**, skills and classes rank from points earned by studying, finishing tasks, quests and streaks. Streaks multiply quest rewards by up to ×1.5.
- **Paths** (Daily, Training, Nutrition, School) each have their own XP, level, rank and streak. Training streaks count weeks you hit your workout target.
- **Nutrition targets** use Mifflin-St Jeor × activity level, ± about 7,700 kcal per kg of weekly change. Protein is set to 1.6–2.2 g per kg.

## Brightspace sync

Brightspace doesn't give students API access. Every account does have a private calendar feed, though, and this repo uses that.

1. In Brightspace, open **Calendar → Subscribe** and copy the feed link.
2. In this repo, go to **Settings → Secrets and variables → Actions** and add:
   - `BRIGHTSPACE_ICS_URL`: the feed link
   - `SYNC_PASSPHRASE`: any long phrase
3. Open the app at https://turbanator84.github.io/ascension-system/, go to **School**, and enter the same passphrase.

The workflow publishes the site to the `gh-pages` branch. If the site doesn't load, check **Settings → Pages** is serving the `gh-pages` branch from `/ (root)`.

Every 3 hours, the workflow fetches your feed and encrypts it with your passphrase (AES-256-GCM). It then publishes the encrypted file next to the app. Your browser decrypts it and turns each assignment, quiz, test and discussion into a task under its course. Tests get review sessions 5 and 2 days before, and quizzes get one the day before. Re-syncing updates changed due dates without duplicating anything.

No secrets set up? Open the feed link in your browser to download the `.ics` file, then use **School → Import .ics file**.

## Install as an app (offline mode)

Open the GitHub Pages site on your phone:

- **iPhone:** Safari → Share → **Add to Home Screen**.
- **Android / desktop Chrome:** Profile → **Install app**, or the browser menu → Install.

The installed app opens full screen and works without signal; changes sync when you're back online.

## Cloud sync, parties and reminders (one-time setup, free)

Without this, progress is saved only in the browser you use. With it, your phone, laptop and the claude.ai copy share one save, and you sign in once per device.

1. Create a free project at [supabase.com](https://supabase.com).
2. In the project, open **SQL Editor → New query**, paste [`supabase/setup.sql`](supabase/setup.sql) and **Run**.
3. Open **Authentication → URL Configuration** and set **Site URL** to your GitHub Pages address. (Sign-in emails contain a link. In the iPhone Home Screen app, press and hold the link, copy it, and paste it into Profile → Cloud sync; tapping it would sign in Safari instead of the app.)
4. Open **Project Settings → API** and copy the **Project URL** and the **anon public** key.
5. In this GitHub repository, open **Settings → Secrets and variables → Actions → Variables** and add `SUPABASE_URL` and `SUPABASE_ANON_KEY`. (Both are designed to be public; row-level security in `setup.sql` keeps each person's data private.) Re-run the **Build, sync Brightspace and deploy** workflow.
6. In the app, open **Profile → Cloud sync**, enter your email and open the link you receive (or paste it in). Do this once on each device. Supabase's built-in email allows only a few sign-in emails per hour.

For the claude.ai copy (or before the GitHub build has the variables), paste the Project URL and anon key into **Profile → Cloud sync** directly.

**Parties:** once signed in, open **Profile → Party** to create one, then send the invite code. Members see each other's level, rank, streak and weekly totals (not your logs), share a weekly leaderboard, and fight a co-op boss whose goals scale with party size.

**Push reminders** ("2 quests left, resets in 3h" at 8 pm, "Quiz 5 due tomorrow" at 6 pm, the weekly report on Sunday at 7 pm, in your time zone):

1. Generate keys once: `npx web-push generate-vapid-keys`.
2. Add repository **variable** `VAPID_PUBLIC_KEY` and repository **secrets** `VAPID_PRIVATE_KEY` and `SUPABASE_SERVICE_ROLE_KEY` (Supabase → Project Settings → API → service_role). Optionally add a `VAPID_SUBJECT` variable such as `mailto:you@example.com`.
3. Re-run the deploy workflow. The **Send push reminders** workflow then runs hourly.
4. In the app: **Profile → App & reminders → Turn on reminders**. On iPhone this only works from the Home Screen app (iOS 16.4 or later).

GitHub can delay scheduled workflows by several minutes, so reminders may arrive a little after the hour.

## Data

Progress is saved in your browser, and to your account when cloud sync is on (the most recent save wins). The claude.ai artifact also saves to your claude.ai account. Use **Profile → Backup** to copy data by hand.

## Development

`src/app.html` is the whole app (no build step, no dependencies). `node scripts/build.mjs` wraps it into `_site/index.html` and adds the manifest, service worker (`src/sw.js`) and icons. `node .claude/serve.mjs` serves `_site` locally on port 8765.
