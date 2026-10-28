//! Melde Tracker statistics exports.

use melde_core::{Date, LocalDateTime, Weekday};
use melde_scheduling::Timetable;
use melde_statistics as st;

use crate::timetable::FfiTimetable;
use crate::{FfiDate, FfiDateTime, MeldeError};

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiParticipationEntry {
    pub id: String,
    pub subject_id: Option<String>,
    pub at: FfiDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiSubjectCount {
    pub subject_id: Option<String>,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiWeekSummary {
    pub week_start: FfiDate,
    pub total: u32,
    /// Seven values, Monday first.
    pub per_day: Vec<u32>,
    pub per_subject: Vec<FfiSubjectCount>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FfiDayCount {
    pub date: FfiDate,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct FfiGoalProgress {
    pub actual: u32,
    pub target: u32,
    pub fraction: f64,
    pub remaining: u32,
    pub achieved: bool,
}

fn entries(list: Vec<FfiParticipationEntry>) -> Result<Vec<st::ParticipationEntry>, MeldeError> {
    list.into_iter()
        .map(|e| {
            Ok(st::ParticipationEntry {
                id: e.id,
                subject_id: e.subject_id,
                at: LocalDateTime::try_from(e.at)?,
            })
        })
        .collect()
}

#[uniffi::export]
pub fn participation_week_summary(
    entries: Vec<FfiParticipationEntry>,
    any_day_in_week: FfiDate,
) -> Result<FfiWeekSummary, MeldeError> {
    let s = st::week_summary(&self::entries(entries)?, any_day_in_week.try_into()?);
    Ok(FfiWeekSummary {
        week_start: s.week_start.into(),
        total: s.total,
        per_day: s.per_day.to_vec(),
        per_subject: s
            .per_subject
            .into_iter()
            .map(|c| FfiSubjectCount {
                subject_id: c.subject_id,
                count: c.count,
            })
            .collect(),
    })
}

#[uniffi::export]
pub fn participation_daily_counts(
    entries: Vec<FfiParticipationEntry>,
    first: FfiDate,
    last: FfiDate,
) -> Result<Vec<FfiDayCount>, MeldeError> {
    let counts = st::daily_counts(
        &self::entries(entries)?,
        first.try_into()?,
        last.try_into()?,
    );
    Ok(counts
        .into_iter()
        .map(|(date, count)| FfiDayCount {
            date: date.into(),
            count,
        })
        .collect())
}

/// Streak of school days with a Meldung. A school day is a weekday that is
/// not a holiday and, once a timetable exists, has at least one lesson.
#[uniffi::export]
pub fn participation_streak(
    entries: Vec<FfiParticipationEntry>,
    timetable: FfiTimetable,
    today: FfiDate,
) -> Result<u32, MeldeError> {
    let tt: Timetable = timetable.try_into()?;
    let has_lessons = !tt.lessons.is_empty();
    let is_school_day = |d: Date| {
        let weekday = !matches!(Weekday::of(d), Weekday::Saturday | Weekday::Sunday);
        let scheduled = !has_lessons || tt.occurrences_on(d).iter().any(|o| !o.cancelled);
        weekday && !tt.is_holiday(d) && scheduled
    };
    Ok(st::streak(
        &self::entries(entries)?,
        today.try_into()?,
        is_school_day,
    ))
}

#[uniffi::export]
pub fn participation_suggest_weekly_target(recent_week_totals: Vec<u32>) -> Option<u32> {
    st::suggest_weekly_target(&recent_week_totals)
}

#[uniffi::export]
pub fn participation_goal_progress(actual: u32, target: u32) -> FfiGoalProgress {
    let p = st::goal_progress(actual, target);
    FfiGoalProgress {
        actual: p.actual,
        target: p.target,
        fraction: p.fraction,
        remaining: p.remaining,
        achieved: p.achieved,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn week_summary_has_seven_days() {
        let entry = FfiParticipationEntry {
            id: "e1".into(),
            subject_id: Some("mathe".into()),
            at: FfiDateTime {
                date: FfiDate {
                    year: 2026,
                    month: 10,
                    day: 21,
                },
                minute_of_day: 600,
            },
        };
        let s = participation_week_summary(
            vec![entry],
            FfiDate {
                year: 2026,
                month: 10,
                day: 19,
            },
        )
        .expect("valid");
        assert_eq!(s.per_day, vec![0, 0, 1, 0, 0, 0, 0]);
        assert_eq!(s.total, 1);
    }
}
