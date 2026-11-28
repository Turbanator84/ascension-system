//! Brightspace calendar (iCal) import: every assignment, quiz, test and
//! discussion becomes a task under its course, and tests and quizzes get
//! review sessions scheduled before them. Re-importing updates due dates
//! without duplicating anything.

use crate::date::{add_days, days_from_civil, key_of};
use crate::js::{arr, get, key, truthy};
use serde_json::{json, Map, Value};

pub const CLASS_COLORS: [&str; 8] =
    ["#5a8dff", "#ff6b4a", "#4fd18b", "#f5a623", "#9a8cff", "#ff7fb0", "#3fb8c9", "#c6e46a"];

/// Review sessions, in days before the due date.
pub fn review_offsets(kind: &str) -> &'static [i64] {
    match kind {
        "Test" => &[7, 4, 2, 1],
        "Quiz" => &[2, 1],
        _ => &[],
    }
}

/// Default difficulty for each kind of task.
pub fn type_diff(kind: &str) -> &'static str {
    match kind {
        "Test" | "Project" => "A",
        "Essay" | "Lab" => "B",
        "Discussion" | "Reading" | "Review" => "D",
        _ => "C",
    }
}

/// One calendar event: property name -> value (first occurrence wins).
pub type Event = Map<String, Value>;

/// Parse the VEVENTs out of an iCal file.
pub fn parse_ics(txt: &str) -> Vec<Event> {
    let unfolded = txt.replace("\r\n ", "").replace("\r\n\t", "").replace("\n ", "").replace("\n\t", "");
    let mut out = Vec::new();
    let mut cur: Option<Event> = None;
    for raw in unfolded.split('\n') {
        let ln = raw.strip_suffix('\r').unwrap_or(raw);
        if ln == "BEGIN:VEVENT" {
            cur = Some(Event::new());
            continue;
        }
        if ln == "END:VEVENT" {
            if let Some(ev) = cur.take() {
                out.push(ev);
            }
            continue;
        }
        let Some(ev) = cur.as_mut() else { continue };
        let Some(i) = ln.find(':') else { continue };
        let name = ln[..i].split(';').next().unwrap_or("").to_uppercase();
        ev.entry(name).or_insert_with(|| Value::String(unescape(&ln[i + 1..])));
    }
    out
}

fn unescape(v: &str) -> String {
    v.replace("\\n", "\n").replace("\\N", "\n").replace("\\,", ",").replace("\\;", ";").replace("\\\\", "\\")
}

/// The local date of an iCal date or date-time. `tz_offset_min(epoch_ms)`
/// returns the user's UTC offset in minutes at that instant (e.g. -300 for EST).
pub fn ics_date(v: &str, tz_offset_min: &dyn Fn(f64) -> i64) -> Option<String> {
    let b = v.trim().as_bytes();
    let digits = |r: std::ops::Range<usize>| -> Option<i64> {
        let s = b.get(r)?;
        if s.iter().all(u8::is_ascii_digit) {
            std::str::from_utf8(s).ok()?.parse().ok()
        } else {
            None
        }
    };
    let (y, mo, d) = (digits(0..4)?, digits(4..6)?, digits(6..8)?);
    let time = if b.get(8) == Some(&b'T') {
        match (digits(9..11), digits(11..13), digits(13..15)) {
            (Some(h), Some(mi), Some(s)) => Some((h, mi, s, b.get(15) == Some(&b'Z'))),
            _ => None,
        }
    } else {
        None
    };
    let Some((h, mi, s, utc)) = time else {
        // Date-only values are used exactly as written.
        let raw = std::str::from_utf8(&b[..8]).ok()?;
        return Some(format!("{}-{}-{}", &raw[..4], &raw[4..6], &raw[6..8]));
    };
    let day = days_from_civil(y, mo, d);
    let secs = day * 86_400 + h * 3600 + mi * 60 + s;
    let local_secs = if utc { secs + tz_offset_min(secs as f64 * 1000.0) * 60 } else { secs };
    Some(key_of(local_secs.div_euclid(86_400)))
}

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Does `hay` contain `word` with JavaScript `\b` boundaries on the given sides?
fn has_word(hay: &str, word: &str, left: bool, right: bool) -> bool {
    let h = hay.as_bytes();
    let mut from = 0;
    while let Some(pos) = hay[from..].find(word) {
        let i = from + pos;
        let j = i + word.len();
        let lb = !left || i == 0 || !is_word(h[i - 1]) || !is_word(h[i]);
        let rb = !right || j == h.len() || !is_word(h[j]) || !is_word(h[j - 1]);
        if lb && rb {
            return true;
        }
        from = i + 1;
        while !hay.is_char_boundary(from) {
            from += 1;
        }
    }
    false
}

/// `\bWORD\s*$`
fn ends_with_word(hay: &str, word: &str) -> bool {
    let t = hay.trim_end_matches(char::is_whitespace);
    t.ends_with(word) && {
        let i = t.len() - word.len();
        i == 0 || !is_word(t.as_bytes()[i - 1])
    }
}

/// What kind of task an event is, or `None` for "availability starts" style events.
pub fn classify(summary: &str) -> Option<&'static str> {
    let s = summary.to_lowercase();
    let starts = s.contains("availability starts")
        || ends_with_word(&s, "start")
        || ends_with_word(&s, "starts")
        || ends_with_word(&s, "open")
        || ends_with_word(&s, "opens")
        || s.contains("start date")
        || has_word(&s, "- start", false, true);
    if starts {
        return None;
    }
    Some(if s.contains("midterm") || s.contains("final exam") || has_word(&s, "exam", true, true) || has_word(&s, "test", true, true) {
        "Test"
    } else if s.contains("quiz") {
        "Quiz"
    } else if s.contains("project") {
        "Project"
    } else if s.contains("essay") || s.contains("paper") {
        "Essay"
    } else if has_word(&s, "lab", true, true) {
        "Lab"
    } else if s.contains("discussion") {
        "Discussion"
    } else if s.contains("reading") || has_word(&s, "read", false, true) {
        "Reading"
    } else {
        "Assignment"
    })
}

/// Drop trailing "- Due", ": Availability Ends" and similar from an event name.
pub fn clean_title(s: &str) -> String {
    let mut t = s.to_string();
    // \s*[-–:]\s*(due|due date|availability ends|ends|end date|end)\s*$  (case-insensitive)
    let body = t.trim_end_matches(char::is_whitespace).to_string();
    let lower = body.to_lowercase();
    if lower.len() == body.len() {
        for alt in ["due", "due date", "availability ends", "ends", "end date", "end"] {
            if lower.ends_with(alt) {
                let before = body[..body.len() - alt.len()].trim_end_matches(char::is_whitespace);
                if let Some(last) = before.chars().last().filter(|c| matches!(c, '-' | '–' | ':')) {
                    t = before[..before.len() - last.len_utf8()].trim_end_matches(char::is_whitespace).to_string();
                    break;
                }
            }
        }
    }
    // \s+due\s*$
    let body = t.trim_end_matches(char::is_whitespace);
    if body.len() >= 3 && body.is_char_boundary(body.len() - 3) && body[body.len() - 3..].eq_ignore_ascii_case("due") {
        let before = &body[..body.len() - 3];
        if before.ends_with(char::is_whitespace) {
            t = before.trim_end_matches(char::is_whitespace).to_string();
        }
    }
    let trimmed = t.trim();
    if trimmed.is_empty() {
        s.to_string()
    } else {
        trimmed.to_string()
    }
}

fn first_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Random-looking ids in the UI's format.
pub struct Ids(u64);

impl Ids {
    pub fn new(seed: u64) -> Self {
        Ids(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn uid(&mut self) -> String {
        const A: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        (0..10).map(|_| A[(self.next() % 36) as usize] as char).collect()
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImportSummary {
    pub events: usize,
    pub added: usize,
    pub updated: usize,
    pub reviews: usize,
}

fn ensure_class(state: &mut Value, name: &str, ids: &mut Ids) -> String {
    let name = first_chars(if name.is_empty() { "Brightspace" } else { name }.trim(), 60);
    let classes = state["classes"].as_array_mut().expect("classes is an array");
    if let Some(c) = classes.iter().find(|c| key(get(c, "name")).to_lowercase() == name.to_lowercase()) {
        return key(get(c, "id"));
    }
    let id = format!("c_{}", ids.uid());
    let color = CLASS_COLORS[classes.len() % CLASS_COLORS.len()];
    classes.push(json!({"id": id, "name": name, "color": color, "src": "bs"}));
    id
}

/// Import a Brightspace calendar into `state`. Returns `None` when the file has no events.
pub fn import_ics(
    state: &mut Value,
    txt: &str,
    today: &str,
    now_iso: &str,
    ids: &mut Ids,
    tz_offset_min: &dyn Fn(f64) -> i64,
) -> Option<ImportSummary> {
    let evs = parse_ics(txt);
    if evs.is_empty() {
        return None;
    }
    for k in ["classes", "tasks"] {
        if !state[k].is_array() {
            state[k] = json!([]);
        }
    }
    let min = add_days(today, -7);
    let mut sum = ImportSummary { events: evs.len(), ..ImportSummary::default() };
    for ev in &evs {
        let summary = ev.get("SUMMARY").and_then(Value::as_str).unwrap_or("");
        if summary.is_empty() {
            continue;
        }
        let Some(kind) = classify(summary) else { continue };
        let raw_due = ["DUE", "DTEND", "DTSTART"]
            .iter()
            .find_map(|k| ev.get(*k).and_then(Value::as_str).filter(|v| !v.is_empty()))
            .unwrap_or("");
        let Some(due) = (!raw_due.is_empty()).then(|| ics_date(raw_due, tz_offset_min)).flatten() else { continue };
        if due < min {
            continue;
        }
        let title = first_chars(&clean_title(summary), 120);
        let loc = ["LOCATION", "CATEGORIES"]
            .iter()
            .find_map(|k| ev.get(*k).and_then(Value::as_str).filter(|v| !v.is_empty()))
            .unwrap_or("");
        let cls = ensure_class(state, loc, ids);
        let uid_key = ev
            .get("UID")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .map_or_else(|| format!("{title}|{due}"), str::to_string);

        let tasks = state["tasks"].as_array_mut().expect("tasks is an array");
        if let Some(ex) = tasks.iter_mut().find(|x| get(x, "uid").and_then(Value::as_str) == Some(uid_key.as_str())) {
            if !truthy(get(ex, "done")) && (key(get(ex, "due")) != due || key(get(ex, "title")) != title) {
                ex["due"] = json!(due);
                ex["title"] = json!(title);
                sum.updated += 1;
            }
        } else {
            tasks.push(json!({
                "id": ids.uid(), "uid": uid_key, "src": "bs", "title": title, "cls": cls,
                "type": kind, "due": due, "diff": type_diff(kind), "done": false
            }));
            sum.added += 1;
        }
        for &off in review_offsets(kind) {
            let rk = format!("{uid_key}#review{off}");
            let rdue = add_days(&due, -off);
            if let Some(rex) = tasks.iter_mut().find(|x| get(x, "uid").and_then(Value::as_str) == Some(rk.as_str())) {
                if !truthy(get(rex, "done")) && key(get(rex, "due")) != rdue {
                    rex["due"] = json!(rdue);
                }
                continue;
            }
            if rdue.as_str() < today {
                continue;
            }
            let early = off >= 4;
            tasks.push(json!({
                "id": ids.uid(), "uid": rk, "src": "bs", "auto": true,
                "title": format!("{}{}", if early { "Start reviewing: " } else { "Review: " }, title),
                "cls": cls, "type": "Review", "due": rdue, "diff": if early { "C" } else { "D" }, "done": false
            }));
            sum.reviews += 1;
        }
    }
    state["bsLast"] = json!(now_iso);
    state["bsCount"] = json!(evs.len());
    Some(sum)
}

/// Every non-done task due on a date (used for desktop reminders).
pub fn due_on<'a>(state: &'a Value, k: &str) -> Vec<&'a Value> {
    arr(state, "tasks")
        .iter()
        .filter(|x| !truthy(get(x, "done")) && key(get(x, "due")) == k)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEED: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:evt-1@d2l\r\nSUMMARY:Assignment 3 - Due\r\nDTSTART:20261203T045900Z\r\nLOCATION:CHEM 1010 - General Chemistry\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:evt-2@d2l\r\nSUMMARY:Midterm Exam 2 - Availability Ends\r\nDTSTART:20261212T170000Z\r\nLOCATION:MATH 1210 - Calculus I\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:evt-2s@d2l\r\nSUMMARY:Midterm Exam 2 - Availability Starts\r\nDTSTART:20261212T150000Z\r\nLOCATION:MATH 1210 - Calculus I\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:evt-3@d2l\r\nSUMMARY:Quiz 5 - Due\r\nDTSTART:20261204T235900\r\nLOCATION:CHEM 1010 - General\r\n  Chemistry\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    fn utc(_: f64) -> i64 {
        0
    }

    #[test]
    fn parses_and_unfolds() {
        let evs = parse_ics(FEED);
        assert_eq!(evs.len(), 4);
        assert_eq!(evs[3]["LOCATION"], "CHEM 1010 - General Chemistry");
    }

    #[test]
    fn dates_respect_time_zone() {
        assert_eq!(ics_date("20261203T045900Z", &utc).as_deref(), Some("2026-12-03"));
        assert_eq!(ics_date("20261203T045900Z", &|_| -300).as_deref(), Some("2026-12-02"));
        assert_eq!(ics_date("20261203", &utc).as_deref(), Some("2026-12-03"));
        assert_eq!(ics_date("nope", &utc), None);
    }

    #[test]
    fn classifies_like_the_web_app() {
        assert_eq!(classify("Midterm Exam 2 - Availability Starts"), None);
        assert_eq!(classify("Lab 4 opens"), None);
        assert_eq!(classify("Unit test review"), Some("Test"));
        assert_eq!(classify("Attestation form"), Some("Assignment"));
        assert_eq!(classify("Quiz 5 - Due"), Some("Quiz"));
        assert_eq!(classify("Lab report"), Some("Lab"));
        assert_eq!(classify("Laboratory safety"), Some("Assignment"));
        assert_eq!(classify("Read chapter 4"), Some("Reading"));
    }

    #[test]
    fn cleans_titles() {
        assert_eq!(clean_title("Assignment 3 - Due"), "Assignment 3");
        assert_eq!(clean_title("Midterm Exam 2 - Availability Ends"), "Midterm Exam 2");
        assert_eq!(clean_title("Essay draft due"), "Essay draft");
        assert_eq!(clean_title("Due"), "Due");
    }

    #[test]
    fn imports_without_duplicates() {
        let mut st = json!({"classes": [], "tasks": []});
        let mut ids = Ids::new(1);
        let s1 = import_ics(&mut st, FEED, "2026-11-28", "2026-11-28T00:00:00.000Z", &mut ids, &utc).unwrap();
        assert_eq!((s1.added, s1.updated, s1.reviews), (3, 0, 6));
        assert_eq!(st["classes"].as_array().unwrap().len(), 2);
        let s2 = import_ics(&mut st, FEED, "2026-11-28", "2026-11-28T00:00:00.000Z", &mut ids, &utc).unwrap();
        assert_eq!((s2.added, s2.updated, s2.reviews), (0, 0, 0));
        assert_eq!(st["tasks"].as_array().unwrap().len(), 9);
    }
}
