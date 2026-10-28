//! Notenrechner / Notenprognose exports.

use melde_grades as g;

use crate::MeldeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiGradeScale {
    Traditional,
    Points,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiRoundingRule {
    HalfTowardBetter,
    HalfTowardWorse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiCategory {
    Written,
    Oral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiTendency {
    Plus,
    None,
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct FfiGradingConfig {
    pub scale: FfiGradeScale,
    pub written_weight: f64,
    pub oral_weight: f64,
    pub rounding: FfiRoundingRule,
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct FfiAssessment {
    pub value: f64,
    pub weight: f64,
    pub category: FfiCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct FfiPlannedAssessment {
    pub weight: f64,
    pub category: FfiCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct FfiSubjectSummary {
    /// Exact weighted average; `None` without grades.
    pub exact: Option<f64>,
    /// Rounded with the configured rule. Not an official grade.
    pub rounded: Option<f64>,
    pub written_average: Option<f64>,
    pub oral_average: Option<f64>,
    pub assessment_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Enum)]
pub enum FfiForecastOutcome {
    AlreadySecured,
    Reachable { required: f64 },
    Unreachable { best_possible_average: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct FfiForecast {
    pub outcome: FfiForecastOutcome,
    pub worst_case_average: f64,
    pub best_case_average: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct FfiSubjectWeight {
    pub average: f64,
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FfiTraditionalGrade {
    pub grade: u8,
    pub tendency: FfiTendency,
}

impl From<FfiGradeScale> for g::GradeScale {
    fn from(s: FfiGradeScale) -> Self {
        match s {
            FfiGradeScale::Traditional => g::GradeScale::Traditional,
            FfiGradeScale::Points => g::GradeScale::Points,
        }
    }
}

impl From<FfiCategory> for g::Category {
    fn from(c: FfiCategory) -> Self {
        match c {
            FfiCategory::Written => g::Category::Written,
            FfiCategory::Oral => g::Category::Oral,
        }
    }
}

impl From<FfiTendency> for g::Tendency {
    fn from(t: FfiTendency) -> Self {
        match t {
            FfiTendency::Plus => g::Tendency::Plus,
            FfiTendency::None => g::Tendency::None,
            FfiTendency::Minus => g::Tendency::Minus,
        }
    }
}

impl From<g::Tendency> for FfiTendency {
    fn from(t: g::Tendency) -> Self {
        match t {
            g::Tendency::Plus => FfiTendency::Plus,
            g::Tendency::None => FfiTendency::None,
            g::Tendency::Minus => FfiTendency::Minus,
        }
    }
}

impl From<FfiRoundingRule> for g::RoundingRule {
    fn from(r: FfiRoundingRule) -> Self {
        match r {
            FfiRoundingRule::HalfTowardBetter => g::RoundingRule::HalfTowardBetter,
            FfiRoundingRule::HalfTowardWorse => g::RoundingRule::HalfTowardWorse,
        }
    }
}

impl From<FfiGradingConfig> for g::GradingConfig {
    fn from(c: FfiGradingConfig) -> Self {
        g::GradingConfig {
            scale: c.scale.into(),
            written_weight: c.written_weight,
            oral_weight: c.oral_weight,
            rounding: c.rounding.into(),
        }
    }
}

impl From<FfiAssessment> for g::Assessment {
    fn from(a: FfiAssessment) -> Self {
        g::Assessment {
            value: a.value,
            weight: a.weight,
            category: a.category.into(),
        }
    }
}

fn assessments(list: Vec<FfiAssessment>) -> Vec<g::Assessment> {
    list.into_iter().map(Into::into).collect()
}

#[uniffi::export]
pub fn grade_subject_summary(
    config: FfiGradingConfig,
    assessments: Vec<FfiAssessment>,
) -> Result<FfiSubjectSummary, MeldeError> {
    let s = g::subject_summary(&config.into(), &self::assessments(assessments))?;
    Ok(FfiSubjectSummary {
        exact: s.exact,
        rounded: s.rounded,
        written_average: s.written_average,
        oral_average: s.oral_average,
        assessment_count: s.assessment_count,
    })
}

#[uniffi::export]
pub fn grade_forecast(
    config: FfiGradingConfig,
    existing: Vec<FfiAssessment>,
    planned: Vec<FfiPlannedAssessment>,
    target_grade: f64,
) -> Result<FfiForecast, MeldeError> {
    let planned: Vec<g::PlannedAssessment> = planned
        .into_iter()
        .map(|p| g::PlannedAssessment {
            weight: p.weight,
            category: p.category.into(),
        })
        .collect();
    let f = g::forecast(
        &config.into(),
        &assessments(existing),
        &planned,
        target_grade,
    )?;
    Ok(FfiForecast {
        outcome: match f.outcome {
            g::ForecastOutcome::AlreadySecured => FfiForecastOutcome::AlreadySecured,
            g::ForecastOutcome::Reachable { required } => {
                FfiForecastOutcome::Reachable { required }
            }
            g::ForecastOutcome::Unreachable {
                best_possible_average,
            } => FfiForecastOutcome::Unreachable {
                best_possible_average,
            },
        },
        worst_case_average: f.worst_case_average,
        best_case_average: f.best_case_average,
    })
}

#[uniffi::export]
pub fn grade_overall_average(subjects: Vec<FfiSubjectWeight>) -> Option<f64> {
    let pairs: Vec<(f64, f64)> = subjects.iter().map(|s| (s.average, s.weight)).collect();
    g::overall_average(&pairs)
}

#[uniffi::export]
pub fn grade_points_to_traditional(points: u8) -> Result<FfiTraditionalGrade, MeldeError> {
    let (grade, tendency) = g::points_to_traditional(points)?;
    Ok(FfiTraditionalGrade {
        grade,
        tendency: tendency.into(),
    })
}

#[uniffi::export]
pub fn grade_traditional_to_points(grade: u8, tendency: FfiTendency) -> Result<u8, MeldeError> {
    Ok(g::traditional_to_points(grade, tendency.into())?)
}

#[uniffi::export]
pub fn grade_traditional_value(
    grade: u8,
    tendency: FfiTendency,
    step: f64,
) -> Result<f64, MeldeError> {
    Ok(g::traditional_value(grade, tendency.into(), step)?)
}

/// A subject's grade as it reads on the 1–6 scale: the grade with tendency
/// closest to the exact average (2,25 → 2-). An estimate, not an official grade.
#[uniffi::export]
pub fn grade_nearest_traditional(
    average: f64,
    step: f64,
    rounding: FfiRoundingRule,
) -> Result<FfiTraditionalGrade, MeldeError> {
    let (grade, tendency) = g::nearest_traditional(average, step, rounding.into())?;
    Ok(FfiTraditionalGrade {
        grade,
        tendency: tendency.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_traditional_crosses_the_boundary() {
        let g = grade_nearest_traditional(2.25, 0.33, FfiRoundingRule::HalfTowardBetter)
            .expect("valid");
        assert_eq!((g.grade, g.tendency), (2, FfiTendency::Minus));
        assert!(matches!(
            grade_nearest_traditional(7.0, 0.33, FfiRoundingRule::HalfTowardBetter),
            Err(MeldeError::InvalidInput { .. })
        ));
    }

    #[test]
    fn forecast_crosses_boundary() {
        let config = FfiGradingConfig {
            scale: FfiGradeScale::Points,
            written_weight: 1.0,
            oral_weight: 1.0,
            rounding: FfiRoundingRule::HalfTowardBetter,
        };
        let existing = vec![
            FfiAssessment {
                value: 9.0,
                weight: 1.0,
                category: FfiCategory::Written,
            },
            FfiAssessment {
                value: 10.0,
                weight: 1.0,
                category: FfiCategory::Written,
            },
        ];
        let planned = vec![FfiPlannedAssessment {
            weight: 1.0,
            category: FfiCategory::Written,
        }];
        let f = grade_forecast(config, existing, planned, 11.0).expect("valid");
        assert_eq!(f.outcome, FfiForecastOutcome::Reachable { required: 13.0 });
    }

    #[test]
    fn validation_errors_are_mapped() {
        let config = FfiGradingConfig {
            scale: FfiGradeScale::Traditional,
            written_weight: 1.0,
            oral_weight: 1.0,
            rounding: FfiRoundingRule::HalfTowardBetter,
        };
        let bad = vec![FfiAssessment {
            value: 7.0,
            weight: 1.0,
            category: FfiCategory::Oral,
        }];
        assert!(matches!(
            grade_subject_summary(config, bad),
            Err(MeldeError::InvalidInput { .. })
        ));
    }
}
