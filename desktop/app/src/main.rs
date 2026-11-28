// No console window behind the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod reminders;
mod storage;

use serde::Serialize;
use std::time::Duration;
use storage::Store;
use tauri::{Manager, State};
use tauri_plugin_notification::NotificationExt;

fn today_local() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// The window reports in once it has started; logged so problems are easy to spot.
#[tauri::command]
fn ui_ready(engine: Option<String>) {
    match engine {
        Some(v) => eprintln!("Ascension System: UI started with the Rust engine {v}"),
        None => eprintln!("Ascension System: UI started WITHOUT the Rust engine (using the JavaScript fallback)"),
    }
}

/// The saved state, or nothing on first run.
#[tauri::command]
fn load_state(store: State<'_, Store>) -> Option<String> {
    store.state()
}

/// Save the state the UI just wrote.
#[tauri::command]
fn save_state(store: State<'_, Store>, json: String) -> Result<(), String> {
    store.save_state(&json, &today_local())
}

#[derive(Serialize)]
struct SettingsView {
    has_feed: bool,
    reminders: bool,
    data_dir: String,
}

#[tauri::command]
fn get_settings(store: State<'_, Store>) -> SettingsView {
    let s = store.settings();
    SettingsView {
        has_feed: s.feed_url.is_some(),
        reminders: !s.reminders_off,
        data_dir: store.dir().display().to_string(),
    }
}

/// Remember the Brightspace calendar feed link on this computer.
#[tauri::command]
fn set_feed(store: State<'_, Store>, url: String) -> Result<(), String> {
    let url = url.trim().to_string();
    if !(url.starts_with("https://") || url.starts_with("http://") || url.starts_with("webcal://")) {
        return Err("That doesn't look like a feed link. Copy it from Brightspace → Calendar → Subscribe.".into());
    }
    store.update_settings(|s| s.feed_url = Some(url))
}

#[tauri::command]
fn set_reminders(store: State<'_, Store>, on: bool) -> Result<(), String> {
    store.update_settings(|s| s.reminders_off = !on)
}

/// Download the Brightspace calendar. Fetching from the app itself means no
/// GitHub step, passphrase or browser restrictions are involved.
#[tauri::command]
async fn fetch_feed(store: State<'_, Store>) -> Result<String, String> {
    let Some(url) = store.settings().feed_url else {
        return Err("Paste your Brightspace feed link first.".into());
    };
    let url = match url.strip_prefix("webcal://") {
        Some(rest) => format!("https://{rest}"),
        None => url,
    };
    let client = reqwest::Client::builder()
        .user_agent(concat!("AscensionSystem/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|_| "Couldn't reach Brightspace. Check your internet connection and try again.".to_string())?;
    if !res.status().is_success() {
        return Err(format!(
            "Brightspace answered {}. The feed link may have changed; copy it again from Calendar → Subscribe.",
            res.status().as_u16()
        ));
    }
    let body = res.text().await.map_err(|e| e.to_string())?;
    if !body.contains("BEGIN:VCALENDAR") {
        return Err("That link didn't return a calendar. Copy it again from Brightspace → Calendar → Subscribe.".into());
    }
    Ok(body)
}

/// Check for reminders once a minute while the app runs.
fn start_reminders(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(60));
        let store = app.state::<Store>();
        let settings = store.settings();
        if settings.reminders_off {
            continue;
        }
        let Some(state) = store.state().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()) else {
            continue;
        };
        let now = chrono::Local::now();
        let today = now.format("%Y-%m-%d").to_string();
        let moment = reminders::Now {
            today: &today,
            hour: chrono::Timelike::hour(&now),
            weekday: chrono::Datelike::weekday(&now).num_days_from_sunday(),
        };
        for r in reminders::due(&state, moment, |k| settings.sent.get(k).cloned()) {
            let _ = app.notification().builder().title(&r.title).body(&r.body).show();
            let _ = store.update_settings(|s| {
                s.sent.insert(r.kind.to_string(), today.clone());
            });
        }
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            app.manage(Store::open(dir)?);
            start_reminders(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![ui_ready, load_state, save_state, get_settings, set_feed, set_reminders, fetch_feed])
        .run(tauri::generate_context!())
        .expect("Ascension System failed to start");
}
