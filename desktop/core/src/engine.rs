//! Everything derived from your history: XP per path, attribute points,
//! counts, streaks with shields, weekly bosses and coins.
//!
//! This mirrors the UI's `computeJS()` step for step (including the order
//! points are added, which affects floating-point rounding), so the desktop
//! app and the website always agree. `tests/parity` checks that on random data.

use crate::date::{add_days, week_key, weekday};
use crate::js::{arr, get, jnum, key, num, num0, obj, or0, r1, round, s, truthy};
use crate::nutrition::{day_target, food_totals, targets};
use serde_json::{json, Map, Value};

pub const PATHS: [&str; 4] = ["daily", "train", "fuel", "school"];
pub const MAX_SHIELDS: i64 = 3;
pub const BOSS_XP: f64 = 150.0;
pub const BOSS_COINS: f64 = 50.0;
pub const COOP_XP: f64 = 100.0;
pub const COOP_COINS: f64 = 30.0;
pub const BOSSES: [&str; 10] = [
    "Iron Golem", "Shadow Monarch", "Cursed Womb", "Frost Wyvern", "Gate Keeper",
    "Ashen Knight", "Abyss Hydra", "Storm Titan", "Hollow King", "Crimson Drake",
];

/// One day's derived totals.
#[derive(Debug, Clone, Default)]
pub struct Day {
    pub daily: f64,
    pub train: f64,
    pub fuel: f64,
    pub school: f64,
    pub wo: f64,
    pub study: f64,
    pub tasks: f64,
    pub kcal: f64,
    pub prot: f64,
    pub fuel_n: f64,
    pub fuel_on: bool,
    pub clr: bool,
    pub pomo: Option<f64>,
    pub fc: Option<Value>,
    pub read: Option<Value>,
    pub books_done: Option<f64>,
    pub boss: bool,
}

impl Day {
    fn path(&self, p: &str) -> f64 {
        match p {
            "daily" => self.daily,
            "train" => self.train,
            "fuel" => self.fuel,
            _ => self.school,
        }
    }

    fn to_json(&self) -> Value {
        let mut m = Map::new();
        for (k, v) in [
            ("daily", self.daily), ("train", self.train), ("fuel", self.fuel), ("school", self.school),
            ("wo", self.wo), ("study", self.study), ("tasks", self.tasks), ("kcal", self.kcal),
            ("prot", self.prot), ("fuelN", self.fuel_n),
        ] {
            m.insert(k.into(), jnum(v));
        }
        m.insert("fuelOn".into(), Value::Bool(self.fuel_on));
        m.insert("clr".into(), Value::Bool(self.clr));
        if let Some(v) = self.pomo {
            m.insert("pomo".into(), jnum(v));
        }
        if let Some(v) = &self.fc {
            m.insert("fc".into(), v.clone());
        }
        if let Some(v) = &self.read {
            m.insert("read".into(), v.clone());
        }
        if let Some(v) = self.books_done {
            m.insert("booksDone".into(), jnum(v));
        }
        if self.boss {
            m.insert("boss".into(), Value::Bool(true));
        }
        Value::Object(m)
    }
}

/// Insertion-ordered map of day key -> totals, like a JavaScript object.
#[derive(Default)]
struct Days {
    order: Vec<String>,
    map: std::collections::HashMap<String, Day>,
}

impl Days {
    fn entry(&mut self, k: &str) -> &mut Day {
        if !self.map.contains_key(k) {
            self.order.push(k.to_string());
            self.map.insert(k.to_string(), Day::default());
        }
        self.map.get_mut(k).expect("just inserted")
    }
    fn get(&self, k: &str) -> Option<&Day> {
        self.map.get(k)
    }
}

/// Insertion-ordered `key -> number` map.
#[derive(Default)]
struct Tally(Map<String, Value>);

impl Tally {
    fn add(&mut self, k: &str, v: f64) {
        let cur = self.0.get(k).map_or(0.0, |x| or0(num(Some(x))));
        self.0.insert(k.to_string(), json_f(cur + v));
    }
    fn get(&self, k: &str) -> f64 {
        self.0.get(k).map_or(0.0, |x| num(Some(x)))
    }
}

/// Keep exact f64 values while counting (no integer conversion).
fn json_f(x: f64) -> Value {
    serde_json::Number::from_f64(x).map_or(Value::Null, Value::Number)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Streak {
    pub cur: i64,
    pub best: i64,
}

/// Earliest date in the save: account creation or the oldest history entry.
pub fn start_key(state: &Value) -> String {
    let mut start = s(state, "created").to_string();
    if let Some(h) = obj(state, "hist") {
        for k in h.keys() {
            if start.is_empty() || k.as_str() < start.as_str() {
                start = k.clone();
            }
        }
    }
    start
}

/// Quests scheduled on a date (repeat day matches and the quest existed by then).
pub fn scheduled_on<'a>(state: &'a Value, k: &str) -> Vec<&'a Value> {
    let w = weekday(k);
    let created = s(state, "created");
    arr(state, "quests")
        .iter()
        .filter(|q| {
            let on = arr(q, "days").iter().any(|d| d.as_f64() == Some(w as f64));
            let qc = get(q, "created").filter(|v| truthy(Some(v))).and_then(Value::as_str).unwrap_or(created);
            on && qc <= k
        })
        .collect()
}

/// A run of days. `test` returns 'y' (counts), 'n' (breaks it) or 's' (skip, e.g. rest day).
/// Today never breaks a streak: it's still in progress.
fn day_streak(start: &str, today: &str, mut test: impl FnMut(&str) -> char) -> Streak {
    let (mut run, mut best) = (0, 0);
    let mut k = start.to_string();
    let mut i = 0;
    while i < 1100 && k.as_str() <= today {
        match test(&k) {
            'y' => {
                run += 1;
                best = best.max(run);
            }
            'n' if k != today => run = 0,
            _ => {}
        }
        i += 1;
        k = add_days(&k, 1);
    }
    Streak { cur: run, best }
}

/// Daily streak with shields: every 7 days in a row earns one (max 3), and a
/// missed day spends one instead of resetting the streak.
fn shield_streak(start: &str, today: &str, mut test: impl FnMut(&str) -> char) -> (Streak, i64, Vec<String>) {
    let (mut run, mut best, mut sh) = (0, 0, 0);
    let mut used = Vec::new();
    let mut k = start.to_string();
    let mut i = 0;
    while i < 1100 && k.as_str() <= today {
        match test(&k) {
            'y' => {
                run += 1;
                best = best.max(run);
                if run % 7 == 0 {
                    sh = MAX_SHIELDS.min(sh + 1);
                }
            }
            'n' if k != today => {
                if sh > 0 && run > 0 {
                    sh -= 1;
                    used.push(k.clone());
                } else {
                    run = 0;
                }
            }
            _ => {}
        }
        i += 1;
        k = add_days(&k, 1);
    }
    (Streak { cur: run, best }, sh, used)
}

/// Weeks in a row hitting the workout target. The current week only counts once it's hit.
fn week_streak(start: &str, today: &str, weeks: &Tally, target: f64) -> Streak {
    let cw = week_key(today);
    let (mut run, mut best) = (0, 0);
    let mut k = week_key(start);
    let mut i = 0;
    while i < 200 && k <= cw {
        if weeks.get(&k) >= target {
            run += 1;
            best = best.max(run);
        } else if k != cw {
            run = 0;
        }
        i += 1;
        k = add_days(&k, 7);
    }
    Streak { cur: run, best }
}

/// The UI's string hash for picking boss names and goals (`(h*31 + charCode) | 0`).
pub fn seed_of(text: &str) -> i64 {
    let mut h: i32 = 7;
    for ch in text.chars() {
        let mut buf = [0u16; 2];
        let unit = ch.encode_utf16(&mut buf)[0];
        h = h.wrapping_mul(31).wrapping_add(i32::from(unit));
    }
    i64::from(h).abs()
}

struct Goal {
    k: &'static str,
    n: f64,
    l: &'static str,
    u: &'static str,
}

fn goal_label(k: &str) -> (&'static str, &'static str) {
    match k {
        "wo" => ("workouts", ""),
        "study" => ("study", "h"),
        "fuel" => ("days on calorie target", ""),
        "clr" => ("days all quests cleared", ""),
        _ => ("tasks finished", ""),
    }
}

fn boss_goals(wk: &str, wk_target: f64) -> Vec<Goal> {
    let third = ["fuel", "clr", "tasks", "clr", "fuel"][(seed_of(wk) % 5) as usize];
    ["wo", "study", third]
        .into_iter()
        .map(|k| {
            let n = match k {
                "wo" => 7.0_f64.min(wk_target + 1.0),
                "study" | "fuel" | "tasks" => 5.0,
                _ => 6.0,
            };
            let (l, u) = goal_label(k);
            Goal { k, n, l, u }
        })
        .collect()
}

#[derive(Default, Clone, Copy)]
struct Cum {
    wo: f64,
    study: f64,
    fuel: f64,
    clr: f64,
    tasks: f64,
}

impl Cum {
    fn get(&self, k: &str) -> f64 {
        match k {
            "wo" => self.wo,
            "study" => self.study,
            "fuel" => self.fuel,
            "clr" => self.clr,
            _ => self.tasks,
        }
    }
}

fn week_cum(wk: &str, days: &Days, upto: Option<i64>) -> Cum {
    let mut c = Cum::default();
    let n = upto.map_or(7, |u| u + 1);
    for i in 0..n {
        let Some(d) = days.get(&add_days(wk, i)) else { continue };
        c.wo += d.wo;
        c.study += d.study / 60.0;
        c.fuel += f64::from(u8::from(d.fuel_on));
        c.clr += f64::from(u8::from(d.clr));
        c.tasks += d.tasks;
    }
    c
}

fn boss_for(wk: &str, days: &Days, wk_target: f64) -> (Value, Option<String>) {
    let goals = boss_goals(wk, wk_target);
    let mut won_on = None;
    for i in 0..7 {
        let c = week_cum(wk, days, Some(i));
        let day = add_days(wk, i);
        if goals.iter().all(|g| c.get(g.k) >= g.n) && days.get(&day).is_some() {
            won_on = Some(day);
            break;
        }
    }
    let c = week_cum(wk, days, None);
    let name = BOSSES[(seed_of(&format!("{wk}b")) % BOSSES.len() as i64) as usize];
    let goals_json: Vec<Value> = goals
        .iter()
        .map(|g| json!({"k": g.k, "n": jnum(g.n), "l": g.l, "u": g.u, "v": jnum(r1(c.get(g.k)))}))
        .collect();
    let v = json!({
        "wk": wk,
        "name": name,
        "goals": goals_json,
        "won": won_on.is_some(),
        "wonOn": won_on.clone().map_or(Value::Null, Value::String),
    });
    (v, won_on)
}

fn sets_of(w: &Value) -> f64 {
    if truthy(get(w, "ex")) {
        arr(w, "ex").iter().fold(0.0, |n, e| n + num0(get(e, "sets")))
    } else {
        num0(get(w, "ns"))
    }
}

const COUNTS: [&str; 18] = [
    "quests", "clears", "workouts", "sets", "km", "trainMin", "fuelDays", "fuelOn", "studyMin",
    "tasksDone", "tasksOnTime", "prs", "weightLogs", "pomo", "cards", "pages", "books", "bosses",
];

/// Derive every total from the saved state. `today` is the user's local date (`YYYY-MM-DD`).
#[allow(clippy::too_many_lines)]
pub fn compute(state: &Value, today: &str) -> Value {
    let t = today;
    let tg = targets(state);
    let wk_target = num(get(state, "wkTarget"));
    let water_target = num(get(state, "waterTarget"));
    let start = start_key(state);

    let mut days = Days::default();
    let mut pts = Tally::default();
    for st in ["STR", "AGI", "VIT", "INT", "WIL"] {
        pts.add(st, 0.0);
    }
    let mut skill = Tally::default();
    let mut cls = Tally::default();
    let mut cls_min = Tally::default();
    let mut weeks = Tally::default();
    let mut cnt = Tally::default();
    for c in COUNTS {
        cnt.add(c, 0.0);
    }

    let read_skill = arr(state, "skills").iter().find_map(|x| {
        let name = key(get(x, "name")).to_lowercase();
        name.contains("read").then(|| key(get(x, "id")))
    });

    let mut hist_keys: Vec<&String> = obj(state, "hist").map(|h| h.keys().collect()).unwrap_or_default();
    hist_keys.sort();
    let hist = obj(state, "hist");

    for k in hist_keys {
        let h = &hist.expect("hist keys come from hist")[k];
        let d = days.entry(k);
        if let Some(a) = obj(h, "a") {
            for aw in a.values() {
                d.daily += num(get(aw, "xp"));
                pts.add(&key(get(aw, "stat")), num(get(aw, "pts")));
                if truthy(get(aw, "skill")) {
                    skill.add(&key(get(aw, "skill")), num(get(aw, "pts")));
                }
                cnt.add("quests", 1.0);
            }
        }
        if truthy(get(h, "clr")) {
            d.clr = true;
            cnt.add("clears", 1.0);
            if let Some(ca) = get(h, "ca").filter(|v| truthy(Some(v))) {
                d.daily += num(get(ca, "xp"));
                pts.add("WIL", num(get(ca, "pts")));
            }
        }
        for w in arr(h, "w") {
            let sets = sets_of(w);
            let mins = num0(get(w, "min")).min(150.0);
            d.train += round(15.0 + mins * 0.4 + sets.min(25.0));
            match s(w, "type") {
                "strength" => pts.add("STR", 1.5 + sets.min(24.0) * 0.15),
                "cardio" => pts.add("AGI", 1.5 + num0(get(w, "km")).min(20.0) * 0.3),
                "sport" => {
                    pts.add("AGI", 1.5);
                    pts.add("STR", 0.5);
                }
                _ => pts.add("VIT", 1.5),
            }
            pts.add("VIT", mins / 60.0);
            d.wo += 1.0;
            cnt.add("workouts", 1.0);
            cnt.add("sets", sets);
            cnt.add("km", num0(get(w, "km")));
            cnt.add("trainMin", num0(get(w, "min")));
            weeks.add(&week_key(k), 1.0);
        }
        if truthy(get(h, "prx")) {
            let prx = num(get(h, "prx"));
            d.train += prx;
            cnt.add("prs", round(prx / 30.0));
        }
        let z = food_totals(Some(h));
        if z.n != 0.0 && !z.n.is_nan() {
            d.kcal = z.k;
            d.prot = z.p;
            d.fuel_n = z.n;
            d.fuel += 5.0 + if z.n >= 3.0 { 5.0 } else { 0.0 };
            cnt.add("fuelDays", 1.0);
            let (tk, tp) = day_target(Some(h), &tg);
            if (z.k - tk).abs() <= 0.1 * tk {
                d.fuel += 20.0;
                d.fuel_on = true;
                pts.add("VIT", 1.5);
                cnt.add("fuelOn", 1.0);
            }
            if z.p >= tp * 0.95 {
                d.fuel += 10.0;
                pts.add("STR", 0.5);
            }
        }
        if truthy(get(h, "wt")) {
            d.fuel += 3.0;
            cnt.add("weightLogs", 1.0);
        }
        if num0(get(h, "wa")) >= water_target {
            d.fuel += 5.0;
            pts.add("VIT", 0.5);
        }
        let mut sm = 0.0;
        for sess in arr(h, "s") {
            sm += num0(get(sess, "min"));
            if truthy(get(sess, "pomo")) {
                let p = num(get(sess, "pomo"));
                d.school += 10.0 * p;
                d.pomo = Some(or0(d.pomo.unwrap_or(0.0)) + p);
                cnt.add("pomo", p);
            }
            if truthy(get(sess, "cls")) {
                let c = key(get(sess, "cls"));
                cls.add(&c, num(get(sess, "min")) * 0.05);
                cls_min.add(&c, num0(get(sess, "min")));
            }
        }
        if sm != 0.0 {
            d.study = sm;
            d.school += round(sm.min(240.0) * 0.3);
            pts.add("INT", sm * 0.03);
            cnt.add("studyMin", sm);
        }
        if truthy(get(h, "fc")) {
            let fc = num(get(h, "fc"));
            d.fc = get(h, "fc").cloned();
            d.school += fc.min(40.0);
            pts.add("INT", fc.min(80.0) * 0.04);
            cnt.add("cards", fc);
        }
        if truthy(get(h, "rd")) {
            let rd = num(get(h, "rd"));
            d.read = get(h, "rd").cloned();
            d.school += 30.0_f64.min(round(rd * 0.2));
            pts.add("INT", rd.min(300.0) * 0.02);
            if let Some(rs) = &read_skill {
                skill.add(rs, rd.min(300.0) * 0.03);
            }
            cnt.add("pages", rd);
        }
    }

    for b in arr(state, "books") {
        if s(b, "status") == "done" && truthy(get(b, "fin")) {
            let d = days.entry(&key(get(b, "fin")));
            d.school += 50.0;
            d.books_done = Some(or0(d.books_done.unwrap_or(0.0)) + 1.0);
            pts.add("INT", 5.0);
            cnt.add("books", 1.0);
        }
    }

    for tk in arr(state, "tasks") {
        if !truthy(get(tk, "done")) || !truthy(get(tk, "award")) {
            continue;
        }
        let a = &tk["award"];
        let dk = [get(a, "day"), get(tk, "doneOn")]
            .into_iter()
            .find(|v| truthy(*v))
            .map_or_else(|| t.to_string(), |v| key(v));
        let d = days.entry(&dk);
        d.school += num(get(a, "xp"));
        d.tasks += 1.0;
        let stat = if truthy(get(a, "stat")) { key(get(a, "stat")) } else { "INT".to_string() };
        pts.add(&stat, num(get(a, "pts")));
        if truthy(get(tk, "cls")) {
            cls.add(&key(get(tk, "cls")), num(get(a, "pts")));
        }
        cnt.add("tasksDone", 1.0);
        if !truthy(get(tk, "due")) || dk.as_str() <= key(get(tk, "due")).as_str() {
            cnt.add("tasksOnTime", 1.0);
        }
    }

    let mut xp = Tally::default();
    for p in PATHS {
        xp.add(p, 0.0);
    }
    let week_list: Vec<(String, f64)> = weeks.0.iter().map(|(k, v)| (k.clone(), num(Some(v)))).collect();
    for (_, n) in &week_list {
        if *n >= wk_target {
            xp.add("train", 40.0);
            pts.add("WIL", 2.0);
        }
    }

    // Weekly bosses are derived from each week's activity, so edits and undo stay consistent.
    let mut boss = Map::new();
    let mut boss_coins = 0.0;
    let mut wk = week_key(&start);
    let cw = week_key(t);
    let mut i = 0;
    while i < 200 && wk <= cw {
        let (b, won_on) = boss_for(&wk, &days, wk_target);
        boss.insert(wk.clone(), b);
        if let Some(day) = won_on {
            let d = days.entry(&day);
            d.daily += BOSS_XP;
            d.boss = true;
            pts.add("WIL", 5.0);
            boss_coins += BOSS_COINS;
            cnt.add("bosses", 1.0);
        }
        i += 1;
        wk = add_days(&wk, 7);
    }
    if let Some(coop) = obj(state, "coop") {
        for (wk, v) in coop {
            if truthy(Some(v)) {
                let day = if *v == Value::Bool(true) { add_days(wk, 6) } else { key(Some(v)) };
                let d = days.entry(&day);
                d.daily += COOP_XP;
                pts.add("WIL", 3.0);
                boss_coins += COOP_COINS;
            }
        }
    }

    for k in &days.order {
        let d = &days.map[k];
        for p in PATHS {
            xp.add(p, d.path(p));
        }
    }
    let total: f64 = PATHS.iter().map(|p| xp.get(p)).sum();
    let coins_earned = (total / 10.0).floor() + boss_coins;
    let spent: f64 = arr(state, "buys").iter().map(|b| num0(get(b, "cost"))).sum();

    let daily_test = |k: &str| {
        if days.get(k).is_some_and(|d| d.clr) {
            'y'
        } else if scheduled_on(state, k).is_empty() {
            's'
        } else {
            'n'
        }
    };
    let (daily, shields, used) = shield_streak(&start, t, daily_test);
    let train = week_streak(&start, t, &weeks, wk_target);
    let fuel = day_streak(&start, t, |k| if days.get(k).is_some_and(|d| d.fuel_on) { 'y' } else { 'n' });
    let school = day_streak(&start, t, |k| {
        if days.get(k).is_some_and(|d| d.study != 0.0 || d.tasks != 0.0) { 'y' } else { 'n' }
    });
    let active = day_streak(&start, t, |k| {
        if days.get(k).is_some_and(|d| d.daily != 0.0 || d.train != 0.0 || d.fuel != 0.0 || d.school != 0.0) {
            'y'
        } else {
            'n'
        }
    });

    let mut pts_out = Map::new();
    for (k, v) in &pts.0 {
        pts_out.insert(k.clone(), jnum(r1(num(Some(v)))));
    }
    let tidy = |t: Tally| Value::Object(t.0.into_iter().map(|(k, v)| (k, jnum(num(Some(&v))))).collect());
    let mut days_out = Map::new();
    for k in &days.order {
        days_out.insert(k.clone(), days.map[k].to_json());
    }
    let st = |s: Streak| json!({"cur": s.cur, "best": s.best});

    json!({
        "days": days_out,
        "xp": tidy(xp),
        "pts": pts_out,
        "skill": tidy(skill),
        "cls": tidy(cls),
        "clsMin": tidy(cls_min),
        "weeks": tidy(weeks),
        "cnt": tidy(cnt),
        "boss": boss,
        "total": jnum(total),
        "coinsEarned": jnum(coins_earned),
        "coins": jnum(coins_earned - spent),
        "streak": {
            "daily": {"cur": daily.cur, "best": daily.best, "shields": shields, "used": used},
            "train": st(train),
            "fuel": st(fuel),
            "school": st(school),
            "active": st(active),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Value {
        json!({
            "created": "2026-11-01", "bw": 75, "hcm": 175, "age": 20, "sex": "m", "act": "moderate",
            "goal": "maintain", "rate": 0.5, "kcalOverride": 0, "wkTarget": 3, "waterTarget": 8,
            "skills": [{"id": "sk_read", "name": "Reading", "stat": "INT"}],
            "quests": [{"id": "q1", "days": [0,1,2,3,4,5,6], "created": "2026-11-01"}],
            "tasks": [], "books": [], "buys": [], "coop": {}, "hist": {}
        })
    }

    #[test]
    fn empty_history_has_no_xp() {
        let r = compute(&base(), "2026-11-28");
        assert_eq!(r["total"], 0);
        assert_eq!(r["streak"]["daily"]["cur"], 0);
        assert_eq!(r["pts"]["STR"], 0);
    }

    #[test]
    fn quest_and_clear_award_xp_and_streak() {
        let mut st = base();
        for d in ["2026-11-26", "2026-11-27", "2026-11-28"] {
            st["hist"][d] = json!({"a": {"q1": {"xp": 25, "pts": 5, "stat": "STR"}}, "clr": true, "ca": {"xp": 25, "pts": 4.1}});
        }
        let r = compute(&st, "2026-11-28");
        assert_eq!(r["xp"]["daily"], 150);
        assert_eq!(r["pts"]["STR"], 15);
        assert_eq!(r["streak"]["daily"]["cur"], 3);
    }

    #[test]
    fn shields_save_a_missed_day() {
        let mut st = base();
        for i in 0..7 {
            st["hist"][add_days("2026-11-10", i)] = json!({"a": {}, "clr": true});
        }
        // 2026-11-17 missed, then two more cleared days.
        st["hist"]["2026-11-18"] = json!({"clr": true});
        st["hist"]["2026-11-19"] = json!({"clr": true});
        let r = compute(&st, "2026-11-19");
        assert_eq!(r["streak"]["daily"]["used"], json!(["2026-11-17"]));
        assert_eq!(r["streak"]["daily"]["cur"], 9);
        assert_eq!(r["streak"]["daily"]["shields"], 0);
    }

    #[test]
    fn workouts_hit_weekly_target_and_bonus() {
        let mut st = base();
        for d in ["2026-11-23", "2026-11-24", "2026-11-25"] {
            st["hist"][d] = json!({"w": [{"type": "strength", "min": 60, "ex": [{"sets": 3}, {"sets": "4"}]}]});
        }
        let r = compute(&st, "2026-11-28");
        // Each: round(15 + 24 + 7) = 46; three of them + 40 weekly bonus.
        assert_eq!(r["xp"]["train"], 46 * 3 + 40);
        assert_eq!(r["streak"]["train"]["cur"], 1);
        assert_eq!(r["weeks"]["2026-11-23"], 3);
    }

    #[test]
    fn on_target_food_day() {
        let mut st = base();
        st["hist"]["2026-11-28"] = json!({"f": [{"k": 2700, "p": 130}], "tg": {"k": 2710, "p": 120}});
        let r = compute(&st, "2026-11-28");
        assert_eq!(r["days"]["2026-11-28"]["fuelOn"], true);
        assert_eq!(r["xp"]["fuel"], 5 + 20 + 10);
        assert_eq!(r["streak"]["fuel"]["cur"], 1);
    }

    #[test]
    fn boss_goals_are_stable() {
        assert_eq!(seed_of("2026-11-23"), seed_of("2026-11-23"));
        let g = boss_goals("2026-11-23", 3.0);
        assert_eq!(g[0].k, "wo");
        assert_eq!(g[0].n, 4.0);
        assert_eq!(g[1].u, "h");
    }

    #[test]
    fn coins_subtract_purchases() {
        let mut st = base();
        st["hist"]["2026-11-28"] = json!({"a": {"q1": {"xp": 250, "pts": 1, "stat": "WIL"}}});
        st["buys"] = json!([{"cost": 10}]);
        let r = compute(&st, "2026-11-28");
        assert_eq!(r["coinsEarned"], 25);
        assert_eq!(r["coins"], 15);
    }
}
