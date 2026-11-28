//! Your save file, settings and daily backups, kept in the app's data folder
//! (e.g. `%APPDATA%\io.github.turbanator84.ascension` on Windows,
//! `~/Library/Application Support/io.github.turbanator84.ascension` on macOS,
//! `~/.local/share/io.github.turbanator84.ascension` on Linux).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const KEEP_BACKUPS: usize = 14;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Brightspace calendar feed link. It works like a password, so it stays on this computer.
    pub feed_url: Option<String>,
    /// Reminder kind -> the date it was last sent, so each goes out at most once a day.
    pub sent: BTreeMap<String, String>,
    /// Desktop reminders are on unless turned off here.
    pub reminders_off: bool,
}

pub struct Store {
    dir: PathBuf,
    state: Mutex<Option<String>>,
    settings: Mutex<Settings>,
}

/// Write via a temporary file and rename, so a crash never leaves half a save.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

impl Store {
    pub fn open(dir: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(dir.join("backups"))?;
        let state = fs::read_to_string(dir.join("state.json")).ok().filter(|s| !s.trim().is_empty());
        let settings = fs::read_to_string(dir.join("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Ok(Store { dir, state: Mutex::new(state), settings: Mutex::new(settings) })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn state(&self) -> Option<String> {
        self.state.lock().expect("state lock").clone()
    }

    /// Save the whole state (JSON from the UI). Also keeps one backup per day for two weeks.
    pub fn save_state(&self, json: &str, today: &str) -> Result<(), String> {
        serde_json::from_str::<serde_json::Value>(json).map_err(|e| format!("Not saved: the data isn't valid ({e})."))?;
        write_atomic(&self.dir.join("state.json"), json.as_bytes()).map_err(|e| format!("Couldn't save: {e}"))?;
        let backup = self.dir.join("backups").join(format!("state-{today}.json"));
        if !backup.exists() {
            let _ = write_atomic(&backup, json.as_bytes());
            self.prune_backups();
        }
        *self.state.lock().expect("state lock") = Some(json.to_string());
        Ok(())
    }

    fn prune_backups(&self) {
        let Ok(rd) = fs::read_dir(self.dir.join("backups")) else { return };
        let mut files: Vec<PathBuf> = rd
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("state-") && n.ends_with(".json")))
            .collect();
        files.sort();
        while files.len() > KEEP_BACKUPS {
            let _ = fs::remove_file(files.remove(0));
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().expect("settings lock").clone()
    }

    pub fn update_settings(&self, f: impl FnOnce(&mut Settings)) -> Result<(), String> {
        let mut s = self.settings.lock().expect("settings lock");
        f(&mut s);
        let bytes = serde_json::to_vec_pretty(&*s).map_err(|e| e.to_string())?;
        write_atomic(&self.dir.join("settings.json"), &bytes).map_err(|e| format!("Couldn't save settings: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ascension-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn saves_reloads_and_rejects_bad_json() {
        let dir = temp_dir("save");
        let st = Store::open(dir.clone()).unwrap();
        assert!(st.state().is_none());
        st.save_state(r#"{"v":2,"name":"A"}"#, "2026-11-28").unwrap();
        assert!(st.save_state("{oops", "2026-11-28").is_err());
        let again = Store::open(dir.clone()).unwrap();
        assert_eq!(again.state().as_deref(), Some(r#"{"v":2,"name":"A"}"#));
        assert!(dir.join("backups/state-2026-11-28.json").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn keeps_two_weeks_of_backups() {
        let dir = temp_dir("backups");
        let st = Store::open(dir.clone()).unwrap();
        for d in 1..=20 {
            st.save_state("{}", &format!("2026-11-{d:02}")).unwrap();
        }
        let n = fs::read_dir(dir.join("backups")).unwrap().count();
        assert_eq!(n, KEEP_BACKUPS);
        assert!(!dir.join("backups/state-2026-11-01.json").exists());
        assert!(dir.join("backups/state-2026-11-20.json").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn settings_persist() {
        let dir = temp_dir("settings");
        let st = Store::open(dir.clone()).unwrap();
        st.update_settings(|s| s.feed_url = Some("https://example.com/feed.ics".into())).unwrap();
        assert_eq!(Store::open(dir.clone()).unwrap().settings().feed_url.as_deref(), Some("https://example.com/feed.ics"));
        let _ = fs::remove_dir_all(dir);
    }
}
