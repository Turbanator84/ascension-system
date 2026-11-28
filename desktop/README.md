# Ascension System — desktop app (Rust)

A native Windows, macOS and Linux app built with Rust and [Tauri](https://tauri.app).

| Part | What it does | Where |
| --- | --- | --- |
| **Game engine** (`ascension-core`) | XP, levels, ranks, streaks and shields, weekly bosses, coins, nutrition targets, Brightspace import with review sessions | `core/` |
| **Desktop shell** (`ascension-desktop`) | The app window, your save file with daily backups, fetching your Brightspace feed, reminder notifications | `app/` |
| **Build helper** (`xtask`) | Compiles the engine to WebAssembly and bundles it with the UI | `xtask/` |

The screens are the same `src/app.html` the website uses (Tauri shows them in a native window), so a UI change made for the website shows up in the desktop app automatically. The engine runs in Rust: the window loads it as WebAssembly and calls it for every total, and the desktop shell links it natively for reminders.

`tests/parity.mjs` runs the Rust engine and the website's JavaScript engine on hundreds of random saves and calendar feeds and fails if a single number differs, so the two can't drift apart.

## Download

Installers are built by GitHub Actions:

- **Releases:** push a tag like `desktop-v0.1.0`; a draft release appears under **Releases** with the installers attached.
- **Any pull request or change on `main`:** open the **Desktop app (Rust)** run under **Actions** and download the installers from **Artifacts**.

| System | File |
| --- | --- |
| Windows | `.msi` or `-setup.exe` |
| macOS | `.dmg` (Apple Silicon `aarch64`, Intel `x64`) |
| Linux | `.AppImage`, `.deb` or `.rpm` |

The installers aren't code-signed yet, so the first launch needs one extra step:

- **Windows:** SmartScreen → **More info** → **Run anyway**.
- **macOS:** right-click the app → **Open** → **Open**. If macOS says the app is damaged, run `xattr -cr "/Applications/Ascension System.app"` in Terminal once.

## What's different from the website

- **Your save is a file on your computer**, in the app's data folder, with one backup per day for the last 14 days. Website data doesn't carry over automatically: copy it from the website's **Profile → Backup** and paste it into the desktop app's **Profile → Backup → Restore**. Cloud sync (Supabase) also works in the desktop app if you set it up.
- **Brightspace:** paste your feed link in **School → Brightspace sync**. The app downloads it itself every 3 hours while it's open; no GitHub secrets or passphrase are needed. The link stays on your computer.
- **Reminders** are native notifications while the app is open: quests left at 8 pm, tasks due tomorrow at 6 pm, and the weekly report on Sunday at 7 pm.
- Barcode scanning needs camera access, which depends on your system's webview (it works on Windows and macOS; on Linux, type the barcode or search instead).

| System | Data folder |
| --- | --- |
| Windows | `%APPDATA%\io.github.turbanator84.ascension` |
| macOS | `~/Library/Application Support/io.github.turbanator84.ascension` |
| Linux | `~/.local/share/io.github.turbanator84.ascension` |

## Build it yourself

Install [Rust](https://rustup.rs), then:

```bash
rustup target add wasm32-unknown-unknown
```

On Linux, also install the window libraries (Ubuntu/Debian):

```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
```

Run the app from the `desktop` folder:

```bash
cargo run -p xtask -- frontend
```

```bash
cargo run -p ascension-desktop
```

Build installers (needs the Tauri CLI: `cargo install tauri-cli --version "^2"` or `npm i -g @tauri-apps/cli@2`):

```bash
cargo tauri build
```

Tests:

```bash
cargo test --workspace
```

Engine parity (needs Node and Playwright with Chromium):

```bash
node tests/parity.mjs 400
```

## Changing the engine

The website's JavaScript (`computeJS`, `targets`, `importICS` in `src/app.html`) and the Rust engine must stay in step. If you change a rule in one, change it in the other; the parity test in CI will tell you if they disagree.
