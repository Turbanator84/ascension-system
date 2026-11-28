//! Calorie and macro targets (Mifflin-St Jeor) and daily food totals.

use crate::js::{get, num, num0, round, s, truthy};
use serde_json::Value;

/// Activity multipliers, keyed like the UI's `ACT` table.
pub fn activity_factor(act: &str) -> f64 {
    match act {
        "sedentary" => 1.2,
        "light" => 1.375,
        "very" => 1.725,
        "athlete" => 1.9,
        _ => 1.55, // "moderate", and the UI's fallback for unknown values
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Targets {
    pub bmr: f64,
    pub tdee: f64,
    pub delta: f64,
    pub kcal: f64,
    pub p: f64,
    pub c: f64,
    pub f: f64,
}

/// Daily targets from the profile: resting burn, maintenance, the cut/bulk
/// adjustment (about 7,700 kcal per kg), and protein/fat/carb grams.
pub fn targets(state: &Value) -> Targets {
    let w = num(get(state, "bw"));
    let h = num(get(state, "hcm"));
    let a = num(get(state, "age"));
    let female = s(state, "sex") == "f";
    let bmr = 10.0 * w + 6.25 * h - 5.0 * a + if female { -161.0 } else { 5.0 };
    let tdee = bmr * activity_factor(s(state, "act"));
    let goal = s(state, "goal");
    let rate = num(get(state, "rate"));
    let delta = match goal {
        "cut" => -rate * 7700.0 / 7.0,
        "bulk" => rate * 7700.0 / 7.0,
        _ => 0.0,
    };
    let floor: f64 = if female { 1200.0 } else { 1500.0 };
    let over = num(get(state, "kcalOverride"));
    let kcal = if over > 0.0 {
        over
    } else {
        floor.max(round((tdee + delta) / 10.0) * 10.0)
    };
    let per_kg = match goal {
        "cut" => 2.2,
        "bulk" => 1.8,
        _ => 1.6,
    };
    let p = round(w * per_kg);
    let f = round(kcal * 0.25 / 9.0);
    let c = 0.0_f64.max(round((kcal - p * 4.0 - f * 9.0) / 4.0));
    Targets { bmr: round(bmr), tdee: round(tdee), delta: round(delta), kcal, p, c, f }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FoodTotals {
    pub k: f64,
    pub p: f64,
    pub c: f64,
    pub f: f64,
    pub n: f64,
}

/// Calories and macros eaten on a day. Older days keep only a summary (`fs`).
pub fn food_totals(day: Option<&Value>) -> FoodTotals {
    let Some(h) = day else { return FoodTotals::default() };
    if let Some(list) = get(h, "f").and_then(Value::as_array).filter(|l| !l.is_empty()) {
        let mut z = FoodTotals { n: list.len() as f64, ..FoodTotals::default() };
        for e in list {
            z.k += num0(get(e, "k"));
            z.p += num0(get(e, "p"));
            z.c += num0(get(e, "c"));
            z.f += num0(get(e, "f"));
        }
        return z;
    }
    if let Some(fs) = get(h, "fs").filter(|v| truthy(Some(v))) {
        return FoodTotals {
            k: num(get(fs, "k")),
            p: num(get(fs, "p")),
            c: num(get(fs, "c")),
            f: num(get(fs, "f")),
            n: num(get(fs, "n")),
        };
    }
    FoodTotals::default()
}

/// The calorie and protein target that applied on a day. A target is saved
/// with the first food logged, so changing your goal later doesn't rewrite history.
pub fn day_target(day: Option<&Value>, current: &Targets) -> (f64, f64) {
    let tg = day.and_then(|h| get(h, "tg"));
    let pick = |k: &str, fallback: f64| {
        let v = tg.and_then(|t| get(t, k));
        if truthy(v) {
            num(v)
        } else {
            fallback
        }
    };
    (pick("k", current.kcal), pick("p", current.p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_profile_targets() {
        // 20-year-old male, 75 kg, 175 cm, moderate activity: BMR 1749, TDEE ~2711.
        let st = json!({"bw":75,"hcm":175,"age":20,"sex":"m","act":"moderate","goal":"maintain","rate":0.5,"kcalOverride":0});
        let t = targets(&st);
        assert_eq!((t.bmr, t.tdee, t.kcal, t.p, t.f, t.c), (1749.0, 2711.0, 2710.0, 120.0, 75.0, 389.0));
    }

    #[test]
    fn cut_applies_deficit_and_floor() {
        let st = json!({"bw":50,"hcm":150,"age":40,"sex":"f","act":"sedentary","goal":"cut","rate":1,"kcalOverride":0});
        assert_eq!(targets(&st).kcal, 1200.0);
    }

    #[test]
    fn totals_prefer_entries_then_summary() {
        let d = json!({"f":[{"k":250,"p":"30"},{"k":"100","p":5}]});
        let z = food_totals(Some(&d));
        assert_eq!((z.k, z.p, z.n), (350.0, 35.0, 2.0));
        let old = json!({"fs":{"k":1800,"p":100,"c":200,"f":60,"n":4}});
        assert_eq!(food_totals(Some(&old)).k, 1800.0);
        assert_eq!(food_totals(None).n, 0.0);
    }
}
