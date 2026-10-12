//! Schedule entities: stored rows plus pure timing math.
//!
//! Timing model: `once` (a single `run_at`), `recurring` (an
//! `interval_secs` cadence), or `weekly` (selected weekdays + local time
//! of day). No cron — the three shapes cover "run this flow at X" without
//! new interaction models. Weekly times are interpreted in the schedule's
//! stored timezone (the creator's local zone).

use std::str::FromStr;

use chrono::{DateTime, Datelike, Utc};

/// Minimum cadence: one minute (bounds worker wake-ups per schedule).
pub const MIN_INTERVAL_SECS: i64 = 60;
/// Maximum cadence: one year.
pub const MAX_INTERVAL_SECS: i64 = 31_536_000;
/// Max recipients per scheduled run (mirrors the ad-hoc send bound).
pub const MAX_RECIPIENTS: usize = 25;

/// One-shot at `run_at`, repeat every `interval_secs`, or fire on
/// selected `days` (0 = Sunday .. 6 = Saturday) at `run_time` ("HH:mm").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleKind {
    Once,
    Recurring,
    Weekly,
}

impl ScheduleKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "once" => Some(Self::Once),
            "recurring" => Some(Self::Recurring),
            "weekly" => Some(Self::Weekly),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Recurring => "recurring",
            Self::Weekly => "weekly",
        }
    }
}

/// Short weekday labels, Sunday-first to match the stored day numbers.
pub const WEEKDAY_LABELS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// A stored schedule. `flow_name`/`flow_definition` ride along via JOIN so
/// list views and the worker never issue a second query.
#[derive(Debug, Clone)]
pub struct Schedule {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub flow_id: String,
    pub flow_name: String,
    pub flow_definition: serde_json::Value,
    pub kind: ScheduleKind,
    pub run_at: Option<DateTime<Utc>>,
    pub interval_secs: Option<i64>,
    /// Weekly days (0 = Sunday .. 6 = Saturday), sorted.
    pub days: Vec<i64>,
    /// Weekly local time ("HH:mm").
    pub run_time: Option<String>,
    pub timezone: String,
    /// Message template picked in the dashboard (rendered at fire time).
    /// `None` = legacy payload rows (channel/title/body/recipients).
    pub template_id: Option<String>,
    pub template_name: Option<String>,
    pub template_content: Option<serde_json::Value>,
    pub channel: String,
    pub title: String,
    pub body: String,
    pub recipients: Vec<String>,
    pub active: bool,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
    pub run_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating a schedule. `run_at` is required for `once`,
/// `interval_secs` for `recurring`, `days` + `run_time` for `weekly`.
/// `template_id` selects a message template; without one the legacy
/// payload fields apply.
#[derive(Debug, Clone)]
pub struct ScheduleInput {
    pub name: String,
    pub flow_id: String,
    pub kind: ScheduleKind,
    pub run_at: Option<DateTime<Utc>>,
    pub interval_secs: Option<i64>,
    pub days: Vec<i64>,
    pub run_time: Option<String>,
    pub timezone: String,
    pub template_id: Option<String>,
    pub channel: String,
    pub title: String,
    pub body: String,
    pub recipients: Vec<String>,
}

/// First fire time for a fresh schedule.
pub fn initial_next_run(
    kind: &ScheduleKind,
    run_at: Option<DateTime<Utc>>,
    interval_secs: Option<i64>,
    days: &[i64],
    run_time: Option<&str>,
    timezone: &str,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    match kind {
        ScheduleKind::Once => run_at,
        ScheduleKind::Recurring => interval_secs.map(|s| now + chrono::Duration::seconds(s)),
        ScheduleKind::Weekly => next_weekly_run(days, run_time, timezone, now),
    }
}

/// Next fire time after a run. Skips missed beats: anchors on `max(now,
/// previous)` so a long-down worker fires once, not a burst of catch-ups.
pub fn advance_next_run(
    kind: &ScheduleKind,
    interval_secs: Option<i64>,
    days: &[i64],
    run_time: Option<&str>,
    timezone: &str,
    previous_next: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    match kind {
        ScheduleKind::Once => None,
        ScheduleKind::Recurring => {
            let anchor = previous_next.map(|p| p.max(now)).unwrap_or(now);
            interval_secs.map(|s| anchor + chrono::Duration::seconds(s))
        }
        ScheduleKind::Weekly => next_weekly_run(days, run_time, timezone, now),
    }
}

/// Parses an IANA timezone name (`None` for unknown strings).
pub fn parse_timezone(name: &str) -> Option<chrono_tz::Tz> {
    chrono_tz::Tz::from_str(name).ok()
}

/// Parses "HH:mm" into (hour, minute).
pub fn parse_run_time(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.split_once(':')?;
    let (h, m) = (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
    if h > 23 || m > 59 {
        return None;
    }
    Some((h, m))
}

/// Sorts/dedupes day numbers (0 = Sunday .. 6 = Saturday); `None` when
/// empty or out of range.
pub fn normalize_days(days: &[i64]) -> Option<Vec<i64>> {
    let mut out: Vec<i64> = days.to_vec();
    out.sort_unstable();
    out.dedup();
    if out.is_empty() || out.iter().any(|d| !(0..=6).contains(d)) {
        return None;
    }
    Some(out)
}

/// Next occurrence of the selected weekdays at `run_time`, interpreted in
/// `timezone` and returned as UTC. Strictly after `now`. Unknown
/// timezones fall back to UTC rather than refusing to schedule.
pub fn next_weekly_run(
    days: &[i64],
    run_time: Option<&str>,
    timezone: &str,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let days = normalize_days(days)?;
    let (hour, minute) = parse_run_time(run_time?)?;
    let tz = parse_timezone(timezone).unwrap_or(chrono_tz::UTC);
    let local_now = now.with_timezone(&tz);
    for offset in 0..8 {
        let date = local_now.date_naive() + chrono::Days::new(offset);
        let weekday = date.weekday().num_days_from_sunday() as i64;
        if !days.contains(&weekday) {
            continue;
        }
        let naive = date
            .and_hms_opt(hour, minute, 0)
            .and_then(|t| t.and_local_timezone(tz).single())
            .map(|d| d.with_timezone(&Utc));
        if let Some(candidate) = naive
            && candidate > now
        {
            return Some(candidate);
        }
    }
    None
}

/// Human summary of the cadence ("Once", "Every 6 hours", "Weekly Mon, Wed 09:00").
pub fn describe_timing(
    kind: &ScheduleKind,
    run_at: Option<DateTime<Utc>>,
    interval_secs: Option<i64>,
    days: &[i64],
    run_time: Option<&str>,
) -> String {
    match kind {
        ScheduleKind::Once => match run_at {
            Some(at) => format!("Once at {}", at.format("%b %-d, %H:%M UTC")),
            None => "Once".to_string(),
        },
        ScheduleKind::Recurring => match interval_secs {
            Some(s) => format!("Every {}", describe_interval(s)),
            None => "Recurring".to_string(),
        },
        ScheduleKind::Weekly => {
            let labels: Vec<&str> = normalize_days(days)
                .unwrap_or_default()
                .iter()
                .map(|d| WEEKDAY_LABELS[*d as usize])
                .collect();
            match (labels.is_empty(), run_time) {
                (false, Some(t)) => format!("Weekly {} {}", labels.join(", "), t),
                (false, None) => format!("Weekly {}", labels.join(", ")),
                _ => "Weekly".to_string(),
            }
        }
    }
}

fn describe_interval(secs: i64) -> String {
    const UNITS: [(i64, &str, &str); 4] = [
        (86_400, "day", "days"),
        (3_600, "hour", "hours"),
        (60, "minute", "minutes"),
        (1, "second", "seconds"),
    ];
    for (unit, one, many) in UNITS {
        if secs % unit == 0 {
            let n = secs / unit;
            return format!("{n} {}", if n == 1 { one } else { many });
        }
    }
    format!("{secs} seconds")
}

/// A template rendered down to sendable content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedContent {
    pub channel: String,
    pub title: String,
    pub body: String,
}

fn text_field(channel: &serde_json::Value, key: &str) -> String {
    channel
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Renders a template's per-channel content map (`{channel: {fields}}`)
/// into one sendable channel + title + body. Prefers email, then short
/// message channels, then push/in-app, then webhook payloads. `None`
/// when the template covers no channels.
pub fn render_template_content(content: &serde_json::Value) -> Option<RenderedContent> {
    let map = content.as_object()?;
    // (channel, title key, body key, body fallback key)
    const ORDER: [(&str, &str, &str, &str); 8] = [
        ("email", "subject", "text", "html"),
        ("sms", "", "sms", ""),
        ("whatsapp", "", "text", ""),
        ("telegram", "", "text", ""),
        ("slack", "", "text", ""),
        ("push", "title", "body", ""),
        ("in_app", "title", "body", ""),
        ("webhook", "", "payload", ""),
    ];
    for (channel, title_key, body_key, fallback_key) in ORDER {
        let entry = map.get(channel)?;
        let mut body = text_field(entry, body_key);
        if body.is_empty() && !fallback_key.is_empty() {
            body = text_field(entry, fallback_key);
        }
        // Skip channels with no body rather than failing the row: a
        // half-filled channel must not shadow a complete one below it.
        if body.is_empty() {
            continue;
        }
        let title = if title_key.is_empty() {
            String::new()
        } else {
            text_field(entry, title_key)
        };
        return Some(RenderedContent {
            channel: channel.to_string(),
            title,
            body,
        });
    }
    None
}

#[cfg(test)]
mod schedule_tests {
    use super::*;
    use chrono::TimeZone;

    fn t() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    #[test]
    fn once_fires_at_run_at_then_stops() {
        let at = t() + chrono::Duration::hours(2);
        assert_eq!(
            initial_next_run(&ScheduleKind::Once, Some(at), None, &[], None, "UTC", t()),
            Some(at)
        );
        assert_eq!(
            advance_next_run(&ScheduleKind::Once, None, &[], None, "UTC", Some(at), at),
            None
        );
    }

    #[test]
    fn recurring_anchors_and_skips_missed_beats() {
        let now = t();
        assert_eq!(
            initial_next_run(
                &ScheduleKind::Recurring,
                None,
                Some(3600),
                &[],
                None,
                "UTC",
                now
            ),
            Some(now + chrono::Duration::hours(1))
        );
        // Missed beats collapse to a single next fire.
        let stale = now - chrono::Duration::hours(5);
        assert_eq!(
            advance_next_run(
                &ScheduleKind::Recurring,
                Some(3600),
                &[],
                None,
                "UTC",
                Some(stale),
                now
            ),
            Some(now + chrono::Duration::hours(1))
        );
    }

    #[test]
    fn weekly_picks_the_next_selected_day() {
        // 2026-01-01 is a Thursday.
        let now = t();
        assert_eq!(
            next_weekly_run(&[1], Some("09:00"), "UTC", now),
            Some(Utc.with_ymd_and_hms(2026, 1, 5, 9, 0, 0).unwrap())
        );
        // Same-day slot already passed → next week.
        let late = Utc.with_ymd_and_hms(2026, 1, 5, 10, 0, 0).unwrap();
        assert_eq!(
            next_weekly_run(&[1], Some("09:00"), "UTC", late),
            Some(Utc.with_ymd_and_hms(2026, 1, 12, 9, 0, 0).unwrap())
        );
        // Timezone shifts the fire time: 09:00 Lagos is 08:00 UTC.
        assert_eq!(
            next_weekly_run(&[1], Some("09:00"), "Africa/Lagos", now),
            Some(Utc.with_ymd_and_hms(2026, 1, 5, 8, 0, 0).unwrap())
        );
        // Unknown zones fall back to UTC rather than refusing.
        assert_eq!(
            next_weekly_run(&[1], Some("09:00"), "Nope/Nowhere", now),
            Some(Utc.with_ymd_and_hms(2026, 1, 5, 9, 0, 0).unwrap())
        );
        assert!(next_weekly_run(&[], Some("09:00"), "UTC", now).is_none());
        assert!(next_weekly_run(&[1], Some("25:00"), "UTC", now).is_none());
    }

    #[test]
    fn timing_descriptions_read_clean() {
        assert_eq!(
            describe_timing(&ScheduleKind::Recurring, None, Some(3600), &[], None),
            "Every 1 hour"
        );
        assert_eq!(
            describe_timing(&ScheduleKind::Recurring, None, Some(86_400), &[], None),
            "Every 1 day"
        );
        assert_eq!(
            describe_timing(&ScheduleKind::Recurring, None, Some(90), &[], None),
            "Every 90 seconds"
        );
        assert_eq!(
            describe_timing(&ScheduleKind::Weekly, None, None, &[1, 3], Some("09:00")),
            "Weekly Mon, Wed 09:00"
        );
    }

    #[test]
    fn templates_render_to_sendable_content() {
        let email = serde_json::json!({
            "email": {"subject": "Hi", "text": "Body", "html": "<p>Body</p>"},
            "sms": {"sms": "Short"},
        });
        assert_eq!(
            render_template_content(&email),
            Some(RenderedContent {
                channel: "email".to_string(),
                title: "Hi".to_string(),
                body: "Body".to_string(),
            })
        );
        // Empty-body channels don't shadow complete ones below.
        let sms_only = serde_json::json!({ "email": {"subject": "Hi"}, "sms": {"sms": "Short"} });
        assert_eq!(
            render_template_content(&sms_only).map(|r| r.channel),
            Some("sms".to_string())
        );
        assert!(render_template_content(&serde_json::json!({})).is_none());
    }
}
