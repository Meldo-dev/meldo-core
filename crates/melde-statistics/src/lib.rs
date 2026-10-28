//! Melde statistics. These count *activity* (hand raises / contributions);
//! they never predict or imply an oral grade.

use std::collections::HashMap;

use chrono::Days;
use melde_core::{Date, LocalDateTime, Weekday};

/// One recorded Meldung.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticipationEntry {
    pub id: String,
    /// `None` when it could not be matched to a lesson and the student skipped manual selection.
    pub subject_id: Option<String>,
    pub at: LocalDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectCount {
    pub subject_id: Option<String>,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekSummary {
    pub week_start: Date,
    pub total: u32,
    /// Monday … Sunday.
    pub per_day: [u32; 7],
    /// Highest count first; ties by subject id for stable output.
    pub per_subject: Vec<SubjectCount>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoalProgress {
    pub actual: u32,
    pub target: u32,
    /// `actual / target`, capped at 1.0 for progress rings.
    pub fraction: f64,
    pub remaining: u32,
    pub achieved: bool,
}

/// Monday of the week containing `date`.
pub fn week_start(date: Date) -> Date {
    date - Days::new(u64::from(Weekday::of(date).index()))
}

pub fn week_summary(entries: &[ParticipationEntry], any_day_in_week: Date) -> WeekSummary {
    let start = week_start(any_day_in_week);
    let end = start + Days::new(7);
    let mut per_day = [0u32; 7];
    let mut per_subject: HashMap<Option<&str>, u32> = HashMap::new();
    for e in entries
        .iter()
        .filter(|e| e.at.date >= start && e.at.date < end)
    {
        per_day[Weekday::of(e.at.date).index() as usize] += 1;
        *per_subject.entry(e.subject_id.as_deref()).or_default() += 1;
    }
    let mut per_subject: Vec<SubjectCount> = per_subject
        .into_iter()
        .map(|(s, count)| SubjectCount {
            subject_id: s.map(str::to_owned),
            count,
        })
        .collect();
    per_subject.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.subject_id.cmp(&b.subject_id))
    });
    WeekSummary {
        week_start: start,
        total: per_day.iter().sum(),
        per_day,
        per_subject,
    }
}

/// Count per day for `first..=last`, including days with zero.
pub fn daily_counts(entries: &[ParticipationEntry], first: Date, last: Date) -> Vec<(Date, u32)> {
    let mut counts: HashMap<Date, u32> = HashMap::new();
    for e in entries
        .iter()
        .filter(|e| e.at.date >= first && e.at.date <= last)
    {
        *counts.entry(e.at.date).or_default() += 1;
    }
    first
        .iter_days()
        .take_while(|d| *d <= last)
        .map(|d| (d, counts.get(&d).copied().unwrap_or(0)))
        .collect()
}

/// Consecutive school days with at least one Meldung, ending today.
/// Today without an entry yet does not break the streak (the day is not
/// over); non-school days (weekends, holidays) are skipped.
pub fn streak(
    entries: &[ParticipationEntry],
    today: Date,
    is_school_day: impl Fn(Date) -> bool,
) -> u32 {
    const MAX_LOOKBACK_DAYS: u64 = 400;
    let active: std::collections::HashSet<Date> = entries.iter().map(|e| e.at.date).collect();
    let mut count = 0;
    for offset in 0..MAX_LOOKBACK_DAYS {
        let Some(day) = today.checked_sub_days(Days::new(offset)) else {
            break;
        };
        if !is_school_day(day) {
            continue;
        }
        if active.contains(&day) {
            count += 1;
        } else if day != today {
            break;
        }
    }
    count
}

/// Suggests a weekly Melde target from recent *school* weeks (oldest first):
/// the median of the last four weeks plus about 10 %, and at least one more
/// than the median so there is something to aim for. `None` without history.
pub fn suggest_weekly_target(recent_week_totals: &[u32]) -> Option<u32> {
    let mut recent: Vec<u32> = recent_week_totals.iter().rev().take(4).copied().collect();
    if recent.is_empty() {
        return None;
    }
    recent.sort_unstable();
    let n = recent.len();
    let median = if n % 2 == 1 {
        f64::from(recent[n / 2])
    } else {
        (f64::from(recent[n / 2 - 1]) + f64::from(recent[n / 2])) / 2.0
    };
    let stretched = (median * 1.1).ceil();
    let at_least_one_more = median.floor() + 1.0;
    // Bounded by u32 inputs, so the cast cannot truncate meaningfully.
    Some(stretched.max(at_least_one_more) as u32)
}

pub fn goal_progress(actual: u32, target: u32) -> GoalProgress {
    let fraction = if target == 0 {
        1.0
    } else {
        (f64::from(actual) / f64::from(target)).min(1.0)
    };
    GoalProgress {
        actual,
        target,
        fraction,
        remaining: target.saturating_sub(actual),
        achieved: actual >= target,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melde_core::LocalTime;

    fn d(y: i32, m: u32, day: u32) -> Date {
        Date::from_ymd_opt(y, m, day).expect("valid date")
    }

    fn entry(date: Date, subject: Option<&str>) -> ParticipationEntry {
        ParticipationEntry {
            id: format!("{date}-{subject:?}"),
            subject_id: subject.map(str::to_owned),
            at: LocalDateTime::new(date, LocalTime::from_hm(9, 0).expect("valid")),
        }
    }

    fn weekdays_only(date: Date) -> bool {
        !matches!(Weekday::of(date), Weekday::Saturday | Weekday::Sunday)
    }

    #[test]
    fn week_summary_counts_per_day_and_subject() {
        let mon = d(2026, 10, 19);
        let entries = vec![
            entry(mon, Some("mathe")),
            entry(mon, Some("mathe")),
            entry(d(2026, 10, 21), Some("deutsch")),
            entry(d(2026, 10, 21), None),
            entry(d(2026, 10, 26), Some("mathe")), // next week
        ];
        let s = week_summary(&entries, d(2026, 10, 22));
        assert_eq!(s.week_start, mon);
        assert_eq!(s.total, 4);
        assert_eq!(s.per_day, [2, 0, 2, 0, 0, 0, 0]);
        assert_eq!(
            s.per_subject[0],
            SubjectCount {
                subject_id: Some("mathe".into()),
                count: 2
            }
        );
        assert_eq!(s.per_subject.len(), 3);
    }

    #[test]
    fn daily_counts_include_empty_days() {
        let entries = vec![entry(d(2026, 10, 20), None)];
        let c = daily_counts(&entries, d(2026, 10, 19), d(2026, 10, 21));
        assert_eq!(
            c,
            vec![
                (d(2026, 10, 19), 0),
                (d(2026, 10, 20), 1),
                (d(2026, 10, 21), 0)
            ]
        );
    }

    #[test]
    fn streak_skips_weekends_and_tolerates_today() {
        // Thu 15, Fri 16, (weekend), Mon 19 active; today Tue 20 without entry yet.
        let entries: Vec<_> = [d(2026, 10, 15), d(2026, 10, 16), d(2026, 10, 19)]
            .into_iter()
            .map(|day| entry(day, None))
            .collect();
        assert_eq!(streak(&entries, d(2026, 10, 20), weekdays_only), 3);
        // Missing Wednesday 14 ended it; a gap on a school day breaks the streak.
        assert_eq!(streak(&entries, d(2026, 10, 22), weekdays_only), 0);
        assert_eq!(streak(&[], d(2026, 10, 20), weekdays_only), 0);
    }

    #[test]
    fn target_suggestion_is_modest() {
        assert_eq!(suggest_weekly_target(&[]), None);
        assert_eq!(suggest_weekly_target(&[0]), Some(1));
        assert_eq!(suggest_weekly_target(&[8, 10, 12]), Some(11));
        // Only the last four weeks count: median of 10, 10, 20, 20 = 15 → 17.
        assert_eq!(suggest_weekly_target(&[100, 10, 10, 20, 20]), Some(17));
    }

    #[test]
    fn goal_progress_caps_fraction() {
        let p = goal_progress(15, 10);
        assert!(p.achieved);
        assert_eq!((p.fraction, p.remaining), (1.0, 0));
        let p = goal_progress(3, 12);
        assert_eq!((p.fraction, p.remaining, p.achieved), (0.25, 9, false));
        assert!(goal_progress(0, 0).achieved);
    }

    proptest::proptest! {
        #[test]
        fn suggestion_never_below_recent_median_plus_one(weeks in proptest::collection::vec(0u32..60, 1..10)) {
            let target = suggest_weekly_target(&weeks).expect("history present");
            let mut recent: Vec<u32> = weeks.iter().rev().take(4).copied().collect();
            recent.sort_unstable();
            let lower_median = recent[(recent.len() - 1) / 2];
            proptest::prop_assert!(target > lower_median);
            proptest::prop_assert!(target <= recent[recent.len() - 1] * 2 + 1);
        }
    }
}
