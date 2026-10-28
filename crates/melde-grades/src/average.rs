//! Weighted averages per subject and across subjects.

use melde_core::{ValidationError, ensure_range};

use crate::scale::{GradeScale, RoundingRule};

/// Whether an assessment counts toward the written (schriftlich) or oral
/// (mündlich / sonstige Mitarbeit) part of a subject grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Written,
    Oral,
}

/// One graded assessment, e.g. a Klassenarbeit or a teacher-assigned oral grade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Assessment {
    pub value: f64,
    /// Weight inside its category, e.g. 2 for a double-weighted Klausur.
    pub weight: f64,
    pub category: Category,
}

/// How a subject combines its grades. Weights are relative (50/50, 2:1, …)
/// because rules differ between states, schools and teachers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradingConfig {
    pub scale: GradeScale,
    pub written_weight: f64,
    pub oral_weight: f64,
    pub rounding: RoundingRule,
}

impl GradingConfig {
    pub fn new(scale: GradeScale, written_weight: f64, oral_weight: f64) -> Self {
        Self {
            scale,
            written_weight,
            oral_weight,
            rounding: RoundingRule::default(),
        }
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        ensure_range("written weight", self.written_weight, 0.0, 1000.0)?;
        ensure_range("oral weight", self.oral_weight, 0.0, 1000.0)?;
        if self.written_weight + self.oral_weight <= 0.0 {
            return Err(ValidationError::Invalid(
                "at least one category weight must be positive".into(),
            ));
        }
        Ok(())
    }

    fn weight_of(&self, category: Category) -> f64 {
        match category {
            Category::Written => self.written_weight,
            Category::Oral => self.oral_weight,
        }
    }
}

/// Result of a subject calculation. `exact` is the true weighted average;
/// `rounded` applies the configured rounding and is *not* an official grade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubjectSummary {
    pub exact: Option<f64>,
    pub rounded: Option<f64>,
    pub written_average: Option<f64>,
    pub oral_average: Option<f64>,
    pub assessment_count: u32,
}

/// Validates every input and computes the subject summary.
pub fn subject_summary(
    config: &GradingConfig,
    assessments: &[Assessment],
) -> Result<SubjectSummary, ValidationError> {
    config.validate()?;
    for a in assessments {
        config.scale.validate(a.value)?;
        ensure_range("assessment weight", a.weight, 0.0, 1000.0)?;
        if a.weight == 0.0 {
            return Err(ValidationError::Invalid(
                "assessment weight must be positive".into(),
            ));
        }
    }
    Ok(summarize_unchecked(config, assessments))
}

/// Core calculation without validation; inputs must already be valid.
pub(crate) fn summarize_unchecked(
    config: &GradingConfig,
    assessments: &[Assessment],
) -> SubjectSummary {
    let written = category_average(assessments, Category::Written);
    let oral = category_average(assessments, Category::Oral);

    // A category without grades does not count; the other one carries the full weight.
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;
    for (avg, category) in [(written, Category::Written), (oral, Category::Oral)] {
        if let Some(avg) = avg {
            let w = config.weight_of(category);
            weighted_sum += avg * w;
            weight_total += w;
        }
    }
    let exact = (weight_total > 0.0).then(|| weighted_sum / weight_total);

    SubjectSummary {
        exact,
        rounded: exact.map(|e| config.scale.round(e, config.rounding)),
        written_average: written,
        oral_average: oral,
        assessment_count: u32::try_from(assessments.len()).unwrap_or(u32::MAX),
    }
}

fn category_average(assessments: &[Assessment], category: Category) -> Option<f64> {
    let (sum, weight) = assessments
        .iter()
        .filter(|a| a.category == category)
        .fold((0.0, 0.0), |(s, w), a| {
            (s + a.value * a.weight, w + a.weight)
        });
    (weight > 0.0).then(|| sum / weight)
}

/// Overall average across subjects as `(subject average, subject weight)`.
/// Subjects without an average or with non-positive weight are skipped.
pub fn overall_average(subjects: &[(f64, f64)]) -> Option<f64> {
    let (sum, weight) = subjects
        .iter()
        .filter(|(avg, w)| avg.is_finite() && w.is_finite() && *w > 0.0)
        .fold((0.0, 0.0), |(s, tw), (avg, w)| (s + avg * w, tw + w));
    (weight > 0.0).then(|| sum / weight)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(value: f64, weight: f64, category: Category) -> Assessment {
        Assessment {
            value,
            weight,
            category,
        }
    }

    fn approx(x: Option<f64>, y: f64) -> bool {
        x.is_some_and(|x| (x - y).abs() < 1e-9)
    }

    #[test]
    fn empty_subject_has_no_average() {
        let cfg = GradingConfig::new(GradeScale::Traditional, 1.0, 1.0);
        let s = subject_summary(&cfg, &[]).expect("valid");
        assert_eq!(s.exact, None);
        assert_eq!(s.rounded, None);
    }

    #[test]
    fn fifty_fifty_written_oral() {
        let cfg = GradingConfig::new(GradeScale::Traditional, 1.0, 1.0);
        let s = subject_summary(
            &cfg,
            &[
                a(2.0, 1.0, Category::Written),
                a(3.0, 1.0, Category::Written),
                a(2.0, 1.0, Category::Oral),
            ],
        )
        .expect("valid");
        assert!(approx(s.written_average, 2.5));
        assert!(approx(s.oral_average, 2.0));
        assert!(approx(s.exact, 2.25));
        assert_eq!(s.rounded, Some(2.0));
    }

    #[test]
    fn individual_weights_inside_category() {
        // Klausur counts double: (12*2 + 9*1) / 3 = 11
        let cfg = GradingConfig::new(GradeScale::Points, 2.0, 1.0);
        let s = subject_summary(
            &cfg,
            &[
                a(12.0, 2.0, Category::Written),
                a(9.0, 1.0, Category::Written),
            ],
        )
        .expect("valid");
        assert!(approx(s.exact, 11.0));
    }

    #[test]
    fn missing_category_redistributes_weight() {
        let cfg = GradingConfig::new(GradeScale::Traditional, 0.6, 0.4);
        let s = subject_summary(&cfg, &[a(3.0, 1.0, Category::Oral)]).expect("valid");
        assert!(approx(s.exact, 3.0));
    }

    #[test]
    fn rejects_out_of_scale_values() {
        let cfg = GradingConfig::new(GradeScale::Traditional, 1.0, 1.0);
        assert!(subject_summary(&cfg, &[a(0.0, 1.0, Category::Oral)]).is_err());
        let cfg = GradingConfig::new(GradeScale::Points, 1.0, 1.0);
        assert!(subject_summary(&cfg, &[a(16.0, 1.0, Category::Oral)]).is_err());
        assert!(subject_summary(&cfg, &[a(10.0, 0.0, Category::Oral)]).is_err());
    }

    #[test]
    fn rejects_zero_weight_config() {
        let cfg = GradingConfig::new(GradeScale::Points, 0.0, 0.0);
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn overall_skips_invalid_entries() {
        assert_eq!(overall_average(&[]), None);
        let avg = overall_average(&[(2.0, 1.0), (3.0, 1.0), (f64::NAN, 1.0), (1.0, 0.0)]);
        assert!(approx(avg, 2.5));
    }
}
