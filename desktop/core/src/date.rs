//! Calendar dates as `YYYY-MM-DD` keys, the format the UI stores history under.
//!
//! Uses plain day numbers (days since 1970-01-01) so the engine needs no time
//! zone database; the caller passes "today" in the user's local time.

/// Days since 1970-01-01 for a proleptic Gregorian date. Month and day may
/// overflow (month 13, day 32) and roll forward like JavaScript's `Date`.
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y + (m - 1).div_euclid(12);
    let m = (m - 1).rem_euclid(12) + 1;
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468 + (d - 1)
}

/// The date for a day number.
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// Day number for a `YYYY-MM-DD` key.
pub fn parse_key(k: &str) -> i64 {
    let mut it = k.split('-').map(|p| p.parse::<i64>().unwrap_or(0));
    let y = it.next().unwrap_or(1970);
    let m = it.next().unwrap_or(1);
    let d = it.next().unwrap_or(1);
    days_from_civil(y, m, d)
}

/// `YYYY-MM-DD` key for a day number.
pub fn key_of(z: i64) -> String {
    let (y, m, d) = civil_from_days(z);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn add_days(k: &str, n: i64) -> String {
    key_of(parse_key(k) + n)
}

/// Day of week with Sunday = 0, like `Date.getDay()`.
pub fn weekday(k: &str) -> i64 {
    (parse_key(k) + 4).rem_euclid(7)
}

/// The Monday that starts this date's week.
pub fn week_key(k: &str) -> String {
    add_days(k, -((weekday(k) + 6) % 7))
}

/// ISO 8601 timestamp (`2026-11-28T14:03:09.120Z`) for milliseconds since the epoch.
pub fn iso_from_ms(ms: f64) -> String {
    let ms = ms.floor() as i64;
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1000 % 60,
        rem % 1000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for k in ["1970-01-01", "2000-02-29", "2026-11-28", "2100-12-31"] {
            assert_eq!(key_of(parse_key(k)), k);
        }
        assert_eq!(add_days("2026-12-31", 1), "2027-01-01");
        assert_eq!(add_days("2028-03-01", -1), "2028-02-29");
    }

    #[test]
    fn weeks_start_monday() {
        assert_eq!(weekday("2026-11-28"), 6); // Saturday
        assert_eq!(week_key("2026-11-28"), "2026-11-23");
        assert_eq!(week_key("2026-11-23"), "2026-11-23");
        assert_eq!(week_key("2026-11-29"), "2026-11-23"); // Sunday belongs to the week before
    }

    #[test]
    fn overflow_rolls_forward() {
        assert_eq!(key_of(days_from_civil(2026, 13, 1)), "2027-01-01");
        assert_eq!(key_of(days_from_civil(2026, 2, 30)), "2026-03-02");
    }

    #[test]
    fn iso_timestamps() {
        assert_eq!(iso_from_ms(0.0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_from_ms(1_795_540_309_123.0), "2026-11-24T17:11:49.123Z");
    }
}
