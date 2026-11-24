# Ascension System

A leveling-system habit tracker. Daily quests, training, nutrition and school each level up on their own, and five attributes (STR, AGI, VIT, INT, WIL) are graded like sorcerers: **Grade 5 → Grade 4 → Grade 3 → Semi-Grade 2 → Grade 2 → Semi-Grade 1 → Grade 1 → Special Grade**.

## Tabs

| Tab | What it does |
| --- | --- |
| **Home** | Hunter card (level, rank, streak), reset countdown, the four path tiles, daily quests, upcoming deadlines |
| **Train** | Log strength, cardio, sport or mobility workouts; weekly target and week streak; benchmarks and rank ladders |
| **Fuel** | Calorie and macro log with quick-add foods, water, weigh-ins and trend, cut / maintain / bulk targets |
| **School** | Assignments (synced from Brightspace), study timer and sessions, class ranks, skills |
| **Calendar** | Month view of everything you did, plus due dates; tap a day for details |
| **Stats** | Attribute radar, path levels, streaks, charts and achievements |

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

## Data

Progress is saved in your browser (and to your claude.ai account in the hosted artifact version). Use **Profile → Backup** to copy your data between devices.

## Development

`src/app.html` is the whole app (no dependencies). `node scripts/build.mjs` wraps it into `_site/index.html`.
