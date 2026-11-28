//! Desktop notifications, checked once a minute while the app is open:
//! - 8 pm: daily quests still open ("2 quests left, resets in 4h")
//! - 6 pm: tasks due tomorrow
//! - Sunday 7 pm: the weekly report is ready

use ascension_core::date::add_days;
use ascension_core::engine::scheduled_on;
use ascension_core::ics::due_on;
use ascension_core::js::{get, key, truthy};
use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub struct Reminder {
    pub kind: &'static str,
    pub title: String,
    pub body: String,
}

/// The local moment we're checking at.
#[derive(Debug, Clone, Copy)]
pub struct Now<'a> {
    pub today: &'a str,
    pub hour: u32,
    /// Sunday = 0.
    pub weekday: u32,
}

/// Reminders that are due now and haven't gone out today. `sent(kind)` is the date each kind last went out.
pub fn due(state: &Value, now: Now<'_>, sent: impl Fn(&str) -> Option<String>) -> Vec<Reminder> {
    let fresh = |kind: &str| sent(kind).as_deref() != Some(now.today);
    let mut out = Vec::new();

    if now.hour >= 20 && fresh("quests") {
        let done = get(state, "hist").and_then(|h| get(h, now.today)).and_then(|d| get(d, "a"));
        let left = scheduled_on(state, now.today)
            .into_iter()
            .filter(|q| !truthy(done.and_then(|a| get(a, &key(get(q, "id"))))))
            .count();
        if left > 0 {
            let hours = 24 - now.hour;
            out.push(Reminder {
                kind: "quests",
                title: "Daily quests".into(),
                body: format!(
                    "{left} quest{} left. Resets in {hours}h.",
                    if left == 1 { "" } else { "s" }
                ),
            });
        }
    }

    if now.hour >= 18 && fresh("due") {
        let tomorrow = add_days(now.today, 1);
        let tasks = due_on(state, &tomorrow);
        if let Some(first) = tasks.first() {
            let mut body = key(get(first, "title"));
            if let Some(second) = tasks.get(1) {
                body = format!("{body} and {}", key(get(second, "title")));
            }
            if tasks.len() > 2 {
                body = format!("{body}, plus {} more", tasks.len() - 2);
            }
            out.push(Reminder { kind: "due", title: "Due tomorrow".into(), body });
        }
    }

    if now.weekday == 0 && now.hour >= 19 && fresh("report") {
        out.push(Reminder {
            kind: "report",
            title: "Weekly report".into(),
            body: "Your week is in. See what improved and what slipped.".into(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state() -> Value {
        json!({
            "created": "2026-11-01",
            "quests": [
                {"id": "q1", "days": [0,1,2,3,4,5,6]},
                {"id": "q2", "days": [0,1,2,3,4,5,6]},
                {"id": "q3", "days": [1]}
            ],
            "tasks": [
                {"title": "Quiz 5", "due": "2026-11-29", "done": false},
                {"title": "Lab report", "due": "2026-11-29", "done": true},
                {"title": "Essay", "due": "2026-11-30", "done": false}
            ],
            "hist": {"2026-11-28": {"a": {"q1": {"xp": 25}}}}
        })
    }

    #[test]
    fn evening_reminders() {
        // Saturday 2026-11-28, 8 pm: q2 still open (q3 is Mondays only); Quiz 5 due tomorrow.
        let r = due(&state(), Now { today: "2026-11-28", hour: 20, weekday: 6 }, |_| None);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].body, "1 quest left. Resets in 4h.");
        assert_eq!(r[1].body, "Quiz 5");
    }

    #[test]
    fn nothing_before_six_or_when_already_sent() {
        assert!(due(&state(), Now { today: "2026-11-28", hour: 17, weekday: 6 }, |_| None).is_empty());
        let r = due(&state(), Now { today: "2026-11-28", hour: 21, weekday: 6 }, |_| Some("2026-11-28".into()));
        assert!(r.is_empty());
    }

    #[test]
    fn sunday_report() {
        let r = due(&state(), Now { today: "2026-11-29", hour: 19, weekday: 0 }, |k| (k != "report").then(|| "2026-11-29".into()));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].kind, "report");
    }
}
