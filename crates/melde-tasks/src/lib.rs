//! Hausaufgaben and to-dos: grouping for the list, recurrence, the homework
//! streak and "due next lesson" suggestions from the timetable.

use std::collections::BTreeMap;

use chrono::Days;
use melde_core::{Date, Weekday};
use melde_scheduling::Timetable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TaskKind {
    #[default]
    Homework,
    Todo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Recurrence {
    #[default]
    None,
    Daily,
    /// Monday to Friday.
    Weekdays,
    Weekly,
    Biweekly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub id: String,
    pub kind: TaskKind,
    pub due: Option<Date>,
    pub completed_on: Option<Date>,
    pub priority: Priority,
    pub recurrence: Recurrence,
    /// Used only for stable ordering of otherwise equal tasks.
    pub title: String,
}

/// Sections of the task list, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    Overdue,
    Today,
    Tomorrow,
    /// Later this week (after tomorrow, up to Sunday).
    ThisWeek,
    Later,
    NoDate,
    Completed,
}

pub fn group_of(task: &Task, today: Date) -> Group {
    if task.completed_on.is_some() {
        return Group::Completed;
    }
    let Some(due) = task.due else {
        return Group::NoDate;
    };
    let tomorrow = today + Days::new(1);
    let sunday = today + Days::new(u64::from(6 - Weekday::of(today).index()));
    if due < today {
        Group::Overdue
    } else if due == today {
        Group::Today
    } else if due == tomorrow {
        Group::Tomorrow
    } else if due <= sunday {
        Group::ThisWeek
    } else {
        Group::Later
    }
}

/// Non-empty groups in display order, tasks sorted inside each group:
/// open tasks by due date, then priority (high first), then title;
/// completed tasks most recent first.
pub fn grouped(tasks: &[Task], today: Date) -> Vec<(Group, Vec<&Task>)> {
    let mut map: BTreeMap<Group, Vec<&Task>> = BTreeMap::new();
    for t in tasks {
        map.entry(group_of(t, today)).or_default().push(t);
    }
    for (group, list) in &mut map {
        if *group == Group::Completed {
            list.sort_by(|a, b| {
                b.completed_on
                    .cmp(&a.completed_on)
                    .then_with(|| a.title.cmp(&b.title))
            });
        } else {
            list.sort_by(|a, b| {
                a.due
                    .cmp(&b.due)
                    .then_with(|| b.priority.cmp(&a.priority))
                    .then_with(|| a.title.cmp(&b.title))
            });
        }
    }
    map.into_iter().collect()
}

/// Due date of the next instance after a recurring task is completed.
/// Skips periods that already passed, so a late completion does not create
/// an instance that is overdue immediately.
pub fn next_due(task: &Task, completed_on: Date) -> Option<Date> {
    let base = task.due.unwrap_or(completed_on);
    let step = |d: Date| -> Option<Date> {
        match task.recurrence {
            Recurrence::None => None,
            Recurrence::Daily => d.checked_add_days(Days::new(1)),
            Recurrence::Weekly => d.checked_add_days(Days::new(7)),
            Recurrence::Biweekly => d.checked_add_days(Days::new(14)),
            Recurrence::Weekdays => {
                let mut next = d.checked_add_days(Days::new(1))?;
                while matches!(Weekday::of(next), Weekday::Saturday | Weekday::Sunday) {
                    next = next.checked_add_days(Days::new(1))?;
                }
                Some(next)
            }
        }
    };
    let mut next = step(base)?;
    // Bounded: at most ~400 steps for a year-old daily task.
    for _ in 0..1000 {
        if next > completed_on {
            return Some(next);
        }
        next = step(next)?;
    }
    None
}

/// Consecutive days with homework due on which all of it was done on time
/// (completed on or before its due date). Days without homework due neither
/// count nor break the streak. Today counts once everything due today is
/// done, and does not break the streak while the day is still open.
pub fn homework_streak(tasks: &[Task], today: Date) -> u32 {
    let mut by_day: BTreeMap<Date, Vec<&Task>> = BTreeMap::new();
    for t in tasks.iter().filter(|t| t.kind == TaskKind::Homework) {
        if let Some(due) = t.due.filter(|d| *d <= today) {
            by_day.entry(due).or_default().push(t);
        }
    }
    let mut streak = 0;
    for (day, list) in by_day.iter().rev() {
        let on_time = list
            .iter()
            .all(|t| t.completed_on.is_some_and(|done| done <= *day));
        if on_time {
            streak += 1;
        } else if *day == today {
            continue;
        } else {
            break;
        }
    }
    streak
}

/// Date of the subject's next lesson after `from` (not on `from` itself),
/// within `horizon_days`; the natural due date for new homework.
pub fn next_lesson_date(
    timetable: &Timetable,
    subject_id: &str,
    from: Date,
    horizon_days: u32,
) -> Option<Date> {
    (1..=u64::from(horizon_days))
        .filter_map(|offset| from.checked_add_days(Days::new(offset)))
        .find(|date| {
            timetable
                .occurrences_on(*date)
                .iter()
                .any(|o| !o.cancelled && o.subject_id == subject_id)
        })
}

/// Share of homework due in `first..=last` that was completed on time, if any was due.
pub fn on_time_rate(tasks: &[Task], first: Date, last: Date) -> Option<f64> {
    let due: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.kind == TaskKind::Homework)
        .filter(|t| t.due.is_some_and(|d| d >= first && d <= last))
        .collect();
    if due.is_empty() {
        return None;
    }
    let on_time = due
        .iter()
        .filter(|t| matches!((t.completed_on, t.due), (Some(c), Some(d)) if c <= d))
        .count();
    Some(on_time as f64 / due.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use melde_core::LocalTime;
    use melde_scheduling::{Holiday, Lesson, WeekParity};

    fn d(y: i32, m: u32, day: u32) -> Date {
        Date::from_ymd_opt(y, m, day).expect("valid date")
    }

    fn hw(id: &str, due: Option<Date>, done: Option<Date>) -> Task {
        Task {
            id: id.into(),
            kind: TaskKind::Homework,
            due,
            completed_on: done,
            priority: Priority::Normal,
            recurrence: Recurrence::None,
            title: id.into(),
        }
    }

    // Wednesday 2026-10-21.
    fn today() -> Date {
        d(2026, 10, 21)
    }

    #[test]
    fn groups_by_due_date() {
        let t = today();
        assert_eq!(
            group_of(&hw("a", Some(d(2026, 10, 20)), None), t),
            Group::Overdue
        );
        assert_eq!(group_of(&hw("b", Some(t), None), t), Group::Today);
        assert_eq!(
            group_of(&hw("c", Some(d(2026, 10, 22)), None), t),
            Group::Tomorrow
        );
        assert_eq!(
            group_of(&hw("d", Some(d(2026, 10, 25)), None), t),
            Group::ThisWeek
        );
        assert_eq!(
            group_of(&hw("e", Some(d(2026, 10, 26)), None), t),
            Group::Later
        );
        assert_eq!(group_of(&hw("f", None, None), t), Group::NoDate);
        assert_eq!(
            group_of(&hw("g", Some(d(2026, 10, 1)), Some(t)), t),
            Group::Completed
        );
    }

    #[test]
    fn grouped_sorts_by_due_then_priority() {
        let mut high = hw("z-high", Some(d(2026, 10, 30)), None);
        high.priority = Priority::High;
        let tasks = vec![
            hw("b", Some(d(2026, 10, 30)), None),
            high,
            hw("a", Some(d(2026, 10, 28)), None),
        ];
        let groups = grouped(&tasks, today());
        assert_eq!(groups.len(), 1);
        let ids: Vec<_> = groups[0].1.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["a", "z-high", "b"]);
    }

    #[test]
    fn recurrence_skips_past_periods() {
        let mut t = hw("vocab", Some(d(2026, 10, 19)), None);
        t.recurrence = Recurrence::Weekly;
        // Completed on time.
        assert_eq!(next_due(&t, d(2026, 10, 19)), Some(d(2026, 10, 26)));
        // Completed two weeks late: next instance is still in the future.
        assert_eq!(next_due(&t, d(2026, 11, 3)), Some(d(2026, 11, 9)));
        t.recurrence = Recurrence::Weekdays;
        // Friday → Monday.
        t.due = Some(d(2026, 10, 23));
        assert_eq!(next_due(&t, d(2026, 10, 23)), Some(d(2026, 10, 26)));
        t.recurrence = Recurrence::None;
        assert_eq!(next_due(&t, d(2026, 10, 23)), None);
    }

    #[test]
    fn homework_streak_rules() {
        let t = today();
        let tasks = vec![
            hw("mon", Some(d(2026, 10, 19)), Some(d(2026, 10, 18))),
            hw("tue", Some(d(2026, 10, 20)), Some(d(2026, 10, 20))),
            // Due today, not done yet: day still open.
            hw("wed", Some(t), None),
            // Done late last week: breaks the streak there.
            hw("fri", Some(d(2026, 10, 16)), Some(d(2026, 10, 17))),
            // Future and to-dos are ignored.
            hw("next", Some(d(2026, 10, 30)), None),
        ];
        assert_eq!(homework_streak(&tasks, t), 2);

        let mut todo = hw("todo", Some(d(2026, 10, 20)), None);
        todo.kind = TaskKind::Todo;
        let mut with_todo = tasks.clone();
        with_todo.push(todo);
        assert_eq!(homework_streak(&with_todo, t), 2);

        // Finishing today's homework extends it.
        let mut done_today = tasks;
        done_today[2].completed_on = Some(t);
        assert_eq!(homework_streak(&done_today, t), 3);
        assert_eq!(homework_streak(&[], t), 0);
    }

    #[test]
    fn suggests_next_lesson_and_skips_holidays() {
        let lesson = Lesson {
            id: "l1".into(),
            subject_id: "mathe".into(),
            weekday: Weekday::Monday,
            start: LocalTime::from_hm(8, 0).expect("valid"),
            end: LocalTime::from_hm(8, 45).expect("valid"),
            parity: WeekParity::Every,
            room: None,
        };
        let mut tt = Timetable {
            lessons: vec![lesson],
            ..Timetable::default()
        };
        assert_eq!(
            next_lesson_date(&tt, "mathe", today(), 21),
            Some(d(2026, 10, 26))
        );
        // Not on the same day: a Monday lesson's homework is due next Monday.
        assert_eq!(
            next_lesson_date(&tt, "mathe", d(2026, 10, 26), 21),
            Some(d(2026, 11, 2))
        );
        tt.holidays.push(Holiday {
            name: "Herbstferien".into(),
            first_day: d(2026, 10, 24),
            last_day: d(2026, 11, 1),
        });
        assert_eq!(
            next_lesson_date(&tt, "mathe", today(), 21),
            Some(d(2026, 11, 2))
        );
        assert_eq!(next_lesson_date(&tt, "bio", today(), 21), None);
    }

    #[test]
    fn on_time_rate_counts_only_due_homework() {
        let tasks = vec![
            hw("a", Some(d(2026, 10, 19)), Some(d(2026, 10, 19))),
            hw("b", Some(d(2026, 10, 20)), Some(d(2026, 10, 22))),
            hw("c", Some(d(2026, 10, 20)), None),
            hw("d", Some(d(2026, 10, 30)), None),
        ];
        assert_eq!(
            on_time_rate(&tasks, d(2026, 10, 19), d(2026, 10, 25)),
            Some(1.0 / 3.0)
        );
        assert_eq!(on_time_rate(&tasks, d(2026, 12, 1), d(2026, 12, 7)), None);
    }

    proptest::proptest! {
        #[test]
        fn next_due_is_always_after_completion(offset in 0u64..400, late in 0u64..60, rec in 0u8..4) {
            let due = d(2026, 1, 1) + Days::new(offset);
            let mut t = hw("x", Some(due), None);
            t.recurrence = [Recurrence::Daily, Recurrence::Weekdays, Recurrence::Weekly, Recurrence::Biweekly][rec as usize];
            let done = due + Days::new(late);
            let next = next_due(&t, done).expect("recurring");
            proptest::prop_assert!(next > done);
            if t.recurrence == Recurrence::Weekdays {
                proptest::prop_assert!(!matches!(Weekday::of(next), Weekday::Saturday | Weekday::Sunday));
            }
        }
    }
}
