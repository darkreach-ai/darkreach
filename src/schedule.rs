//! # Schedule — Cron Expression Parser and Evaluator
//!
//! Minimal 5-field cron parser for the agent schedule executor. Supports the
//! standard cron fields: `minute hour day-of-month month day-of-week`.
//!
//! ## Supported Syntax
//!
//! - Exact values: `0 2 * * *` (daily at 02:00)
//! - Wildcards: `*` (every value)
//! - Step values: `*/5` (every 5 units)
//! - Ranges: `1-5` (values 1 through 5)
//! - Lists: `0,15,30,45` (specific values)
//! - Combined: `1-5/2` (odd values in range 1-5, i.e., 1, 3, 5)
//!
//! ## References
//!
//! - POSIX cron specification (IEEE Std 1003.1)
//! - Vixie cron (most common implementation)

use chrono::{DateTime, Datelike, Timelike, Utc};

/// Check whether `now` matches a 5-field cron expression and enough time has
/// elapsed since `last_fired` to avoid double-firing within the same minute.
///
/// Returns `true` if:
/// 1. `now` matches the cron expression, AND
/// 2. `last_fired` is `None` (never fired) OR `last_fired` is in a different
///    minute than `now`.
pub fn cron_should_fire(
    expr: &str,
    last_fired: Option<&DateTime<Utc>>,
    now: &DateTime<Utc>,
) -> bool {
    let fields = match parse_cron(expr) {
        Some(f) => f,
        None => return false,
    };

    if !matches_cron(&fields, now) {
        return false;
    }

    // Prevent double-fire in the same minute
    if let Some(last) = last_fired {
        if last.date_naive() == now.date_naive()
            && last.hour() == now.hour()
            && last.minute() == now.minute()
        {
            return false;
        }
    }

    true
}

/// Parsed 5-field cron expression.
struct CronFields {
    minute: Vec<u32>,
    hour: Vec<u32>,
    dom: Vec<u32>,
    month: Vec<u32>,
    dow: Vec<u32>,
}

/// Parse a 5-field cron expression into expanded value sets.
///
/// Returns `None` if the expression is malformed (wrong number of fields,
/// invalid syntax in any field, or out-of-range values).
fn parse_cron(expr: &str) -> Option<CronFields> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }

    Some(CronFields {
        minute: parse_field(parts[0], 0, 59)?,
        hour: parse_field(parts[1], 0, 23)?,
        dom: parse_field(parts[2], 1, 31)?,
        month: parse_field(parts[3], 1, 12)?,
        dow: parse_field(parts[4], 0, 6)?,
    })
}

/// Parse a single cron field into a sorted list of matching values.
///
/// Handles: `*`, `*/N`, `N`, `N-M`, `N-M/S`, `N,M,O`.
fn parse_field(field: &str, min: u32, max: u32) -> Option<Vec<u32>> {
    let mut values = Vec::new();

    for part in field.split(',') {
        if part == "*" {
            return Some((min..=max).collect());
        }

        if let Some(step_str) = part.strip_prefix("*/") {
            let step: u32 = step_str.parse().ok()?;
            if step == 0 {
                return None;
            }
            let mut v = min;
            while v <= max {
                values.push(v);
                v += step;
            }
            continue;
        }

        if part.contains('/') {
            // range/step: e.g., "1-5/2"
            let slash_parts: Vec<&str> = part.splitn(2, '/').collect();
            let step: u32 = slash_parts[1].parse().ok()?;
            if step == 0 {
                return None;
            }
            let (range_min, range_max) = parse_range(slash_parts[0], min, max)?;
            let mut v = range_min;
            while v <= range_max {
                values.push(v);
                v += step;
            }
            continue;
        }

        if part.contains('-') {
            let (range_min, range_max) = parse_range(part, min, max)?;
            values.extend(range_min..=range_max);
            continue;
        }

        // Single value
        let v: u32 = part.parse().ok()?;
        if v < min || v > max {
            return None;
        }
        values.push(v);
    }

    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    values.dedup();
    Some(values)
}

/// Parse a "min-max" range string.
fn parse_range(s: &str, field_min: u32, field_max: u32) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.splitn(2, '-').collect();
    if parts.len() != 2 {
        return None;
    }
    let lo: u32 = parts[0].parse().ok()?;
    let hi: u32 = parts[1].parse().ok()?;
    if lo > hi || lo < field_min || hi > field_max {
        return None;
    }
    Some((lo, hi))
}

/// Check if a DateTime matches the parsed cron fields.
fn matches_cron(fields: &CronFields, dt: &DateTime<Utc>) -> bool {
    let minute = dt.minute();
    let hour = dt.hour();
    let dom = dt.day();
    let month = dt.month();
    // chrono: Monday=0..Sunday=6 via weekday().num_days_from_monday()
    // cron: Sunday=0, Monday=1..Saturday=6
    let dow = dt.weekday().num_days_from_sunday();

    fields.minute.contains(&minute)
        && fields.hour.contains(&hour)
        && fields.dom.contains(&dom)
        && fields.month.contains(&month)
        && fields.dow.contains(&dow)
}

/// Map event type string from Event enum to schedule event_filter format.
pub fn event_type_for_schedule(event: &crate::events::Event) -> Option<&'static str> {
    match event {
        crate::events::Event::PrimeFound { .. } => Some("PrimeFound"),
        crate::events::Event::SearchCompleted { .. } => Some("SearchCompleted"),
        crate::events::Event::Error { .. } => Some("Error"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(y: i32, mo: u32, d: u32, h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, m, 0).unwrap()
    }

    // ── parse_field ──────────────────────────────────────────────────

    #[test]
    fn parse_field_wildcard() {
        let vals = parse_field("*", 0, 59).unwrap();
        assert_eq!(vals.len(), 60);
        assert_eq!(vals[0], 0);
        assert_eq!(vals[59], 59);
    }

    #[test]
    fn parse_field_step() {
        let vals = parse_field("*/15", 0, 59).unwrap();
        assert_eq!(vals, vec![0, 15, 30, 45]);
    }

    #[test]
    fn parse_field_exact() {
        let vals = parse_field("5", 0, 59).unwrap();
        assert_eq!(vals, vec![5]);
    }

    #[test]
    fn parse_field_range() {
        let vals = parse_field("1-5", 0, 6).unwrap();
        assert_eq!(vals, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn parse_field_list() {
        let vals = parse_field("0,15,30,45", 0, 59).unwrap();
        assert_eq!(vals, vec![0, 15, 30, 45]);
    }

    #[test]
    fn parse_field_range_step() {
        let vals = parse_field("1-10/3", 0, 59).unwrap();
        assert_eq!(vals, vec![1, 4, 7, 10]);
    }

    #[test]
    fn parse_field_out_of_range_fails() {
        assert!(parse_field("60", 0, 59).is_none());
    }

    #[test]
    fn parse_field_zero_step_fails() {
        assert!(parse_field("*/0", 0, 59).is_none());
    }

    // ── cron_should_fire ─────────────────────────────────────────────

    #[test]
    fn daily_at_2am() {
        let now = utc(2026, 2, 22, 2, 0);
        assert!(cron_should_fire("0 2 * * *", None, &now));
    }

    #[test]
    fn daily_at_2am_wrong_hour() {
        let now = utc(2026, 2, 22, 3, 0);
        assert!(!cron_should_fire("0 2 * * *", None, &now));
    }

    #[test]
    fn every_5_minutes() {
        let now = utc(2026, 2, 22, 12, 15);
        assert!(cron_should_fire("*/5 * * * *", None, &now));
    }

    #[test]
    fn every_5_minutes_not_matching() {
        let now = utc(2026, 2, 22, 12, 13);
        assert!(!cron_should_fire("*/5 * * * *", None, &now));
    }

    #[test]
    fn no_double_fire_same_minute() {
        let now = utc(2026, 2, 22, 2, 0);
        let last = utc(2026, 2, 22, 2, 0);
        assert!(!cron_should_fire("0 2 * * *", Some(&last), &now));
    }

    #[test]
    fn fires_if_last_was_different_minute() {
        let now = utc(2026, 2, 22, 2, 0);
        let last = utc(2026, 2, 21, 2, 0);
        assert!(cron_should_fire("0 2 * * *", Some(&last), &now));
    }

    #[test]
    fn weekday_filter() {
        // 2026-02-22 is a Sunday (dow=0)
        let now = utc(2026, 2, 22, 0, 0);
        assert!(cron_should_fire("0 0 * * 0", None, &now)); // Sunday
        assert!(!cron_should_fire("0 0 * * 1", None, &now)); // Monday
    }

    #[test]
    fn weekday_range_mon_fri() {
        // 2026-02-23 is a Monday (dow=1)
        let now = utc(2026, 2, 23, 9, 0);
        assert!(cron_should_fire("0 9 * * 1-5", None, &now));
    }

    #[test]
    fn month_filter() {
        let jan = utc(2026, 1, 15, 0, 0);
        let feb = utc(2026, 2, 15, 0, 0);
        assert!(cron_should_fire("0 0 15 1 *", None, &jan));
        assert!(!cron_should_fire("0 0 15 1 *", None, &feb));
    }

    #[test]
    fn invalid_expression_returns_false() {
        let now = utc(2026, 2, 22, 0, 0);
        assert!(!cron_should_fire("bad", None, &now));
        assert!(!cron_should_fire("* * *", None, &now));
        assert!(!cron_should_fire("", None, &now));
    }

    #[test]
    fn complex_expression() {
        // At minute 0 and 30, hours 9-17, Mon-Fri
        let now = utc(2026, 2, 23, 9, 30); // Monday 09:30
        assert!(cron_should_fire("0,30 9-17 * * 1-5", None, &now));
    }
}
