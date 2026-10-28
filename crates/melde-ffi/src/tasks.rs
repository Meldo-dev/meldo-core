//! Hausaufgaben / to-do exports.

use melde_core::Date;
use melde_scheduling::Timetable;
use melde_tasks as t;

use crate::timetable::FfiTimetable;
use crate::{FfiDate, MeldeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiTaskKind {
    Homework,
    Todo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiPriority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiRecurrence {
    Never,
    Daily,
    Weekdays,
    Weekly,
    Biweekly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiTaskGroup {
    Overdue,
    Today,
    Tomorrow,
    ThisWeek,
    Later,
    NoDate,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiTask {
    pub id: String,
    pub title: String,
    pub kind: FfiTaskKind,
    pub due: Option<FfiDate>,
    pub completed_on: Option<FfiDate>,
    pub priority: FfiPriority,
    pub recurrence: FfiRecurrence,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiTaskSection {
    pub group: FfiTaskGroup,
    /// Task ids in display order.
    pub task_ids: Vec<String>,
}

impl TryFrom<FfiTask> for t::Task {
    type Error = MeldeError;

    fn try_from(f: FfiTask) -> Result<Self, Self::Error> {
        Ok(t::Task {
            id: f.id,
            title: f.title,
            kind: match f.kind {
                FfiTaskKind::Homework => t::TaskKind::Homework,
                FfiTaskKind::Todo => t::TaskKind::Todo,
            },
            due: f.due.map(Date::try_from).transpose()?,
            completed_on: f.completed_on.map(Date::try_from).transpose()?,
            priority: match f.priority {
                FfiPriority::Low => t::Priority::Low,
                FfiPriority::Normal => t::Priority::Normal,
                FfiPriority::High => t::Priority::High,
            },
            recurrence: match f.recurrence {
                FfiRecurrence::Never => t::Recurrence::None,
                FfiRecurrence::Daily => t::Recurrence::Daily,
                FfiRecurrence::Weekdays => t::Recurrence::Weekdays,
                FfiRecurrence::Weekly => t::Recurrence::Weekly,
                FfiRecurrence::Biweekly => t::Recurrence::Biweekly,
            },
        })
    }
}

fn tasks(list: Vec<FfiTask>) -> Result<Vec<t::Task>, MeldeError> {
    list.into_iter().map(TryInto::try_into).collect()
}

#[uniffi::export]
pub fn tasks_grouped(
    tasks: Vec<FfiTask>,
    today: FfiDate,
) -> Result<Vec<FfiTaskSection>, MeldeError> {
    let tasks = self::tasks(tasks)?;
    Ok(t::grouped(&tasks, today.try_into()?)
        .into_iter()
        .map(|(group, list)| FfiTaskSection {
            group: match group {
                t::Group::Overdue => FfiTaskGroup::Overdue,
                t::Group::Today => FfiTaskGroup::Today,
                t::Group::Tomorrow => FfiTaskGroup::Tomorrow,
                t::Group::ThisWeek => FfiTaskGroup::ThisWeek,
                t::Group::Later => FfiTaskGroup::Later,
                t::Group::NoDate => FfiTaskGroup::NoDate,
                t::Group::Completed => FfiTaskGroup::Completed,
            },
            task_ids: list.into_iter().map(|t| t.id.clone()).collect(),
        })
        .collect())
}

/// Due date for the next instance of a recurring task completed on `completed_on`.
#[uniffi::export]
pub fn tasks_next_due(task: FfiTask, completed_on: FfiDate) -> Result<Option<FfiDate>, MeldeError> {
    let task: t::Task = task.try_into()?;
    Ok(t::next_due(&task, completed_on.try_into()?).map(Into::into))
}

#[uniffi::export]
pub fn tasks_homework_streak(tasks: Vec<FfiTask>, today: FfiDate) -> Result<u32, MeldeError> {
    Ok(t::homework_streak(&self::tasks(tasks)?, today.try_into()?))
}

#[uniffi::export]
pub fn tasks_next_lesson_date(
    timetable: FfiTimetable,
    subject_id: String,
    from: FfiDate,
    horizon_days: u32,
) -> Result<Option<FfiDate>, MeldeError> {
    let tt: Timetable = timetable.try_into()?;
    Ok(t::next_lesson_date(&tt, &subject_id, from.try_into()?, horizon_days).map(Into::into))
}

#[uniffi::export]
pub fn tasks_on_time_rate(
    tasks: Vec<FfiTask>,
    first: FfiDate,
    last: FfiDate,
) -> Result<Option<f64>, MeldeError> {
    Ok(t::on_time_rate(
        &self::tasks(tasks)?,
        first.try_into()?,
        last.try_into()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(day: u8) -> FfiDate {
        FfiDate {
            year: 2026,
            month: 10,
            day,
        }
    }

    fn task(id: &str, due: u8, done: Option<u8>) -> FfiTask {
        FfiTask {
            id: id.into(),
            title: id.into(),
            kind: FfiTaskKind::Homework,
            due: Some(date(due)),
            completed_on: done.map(date),
            priority: FfiPriority::Normal,
            recurrence: FfiRecurrence::Never,
        }
    }

    #[test]
    fn sections_and_streak_cross_the_boundary() {
        let tasks = vec![
            task("late", 20, None),
            task("today", 21, None),
            task("done", 19, Some(19)),
        ];
        let sections = tasks_grouped(tasks.clone(), date(21)).expect("valid");
        let groups: Vec<_> = sections.iter().map(|s| s.group).collect();
        assert_eq!(
            groups,
            [
                FfiTaskGroup::Overdue,
                FfiTaskGroup::Today,
                FfiTaskGroup::Completed
            ]
        );
        // "late" (due yesterday, not done) breaks the streak.
        assert_eq!(tasks_homework_streak(tasks, date(21)).expect("valid"), 0);
    }

    #[test]
    fn rejects_invalid_dates() {
        let mut bad = task("x", 21, None);
        bad.due = Some(FfiDate {
            year: 2026,
            month: 2,
            day: 30,
        });
        assert!(tasks_grouped(vec![bad], date(21)).is_err());
    }
}
