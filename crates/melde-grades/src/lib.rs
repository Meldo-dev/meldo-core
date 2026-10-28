//! Notenrechner and Notenprognose.
//!
//! Calculations are exact weighted averages. Rounding is applied separately
//! and configurably; neither value is an official grade, which only the
//! teacher assigns.

mod average;
mod forecast;
mod scale;

pub use average::{
    Assessment, Category, GradingConfig, SubjectSummary, overall_average, subject_summary,
};
pub use forecast::{Forecast, ForecastOutcome, PlannedAssessment, forecast};
pub use scale::{
    GradeScale, RoundingRule, Tendency, nearest_traditional, points_to_traditional,
    traditional_to_points, traditional_value,
};
