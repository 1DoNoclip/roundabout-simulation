//! Contains the trait implementations to convert Bevy splines into Segments.

use crate::*;

pub(super) struct CurvePlugin;

impl Plugin for CurvePlugin {
    fn build(&self, _app: &mut App) {}
}

/// A curve type used for segment evaluators.
pub(crate) trait SegmentCurve: CurveLength + IntoEvaluator + Send + Sync + 'static {}
// Blanket implementation.
impl<T> SegmentCurve for T where T: CurveLength + IntoEvaluator + Send + Sync + 'static {}

/// The ability to get a length of a curve.
pub(crate) trait CurveLength {
    fn length(&self) -> f32;
}

/// The ability to convert a curve into an evaluator function.
pub(crate) trait IntoEvaluator {
    fn into_evaluator(self) -> Box<dyn Fn(f32) -> Vec3 + Send + Sync + 'static>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_length() {
        let points = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(3.0, 4.0, 0.0)];
        let curve = StraightLinePoints(points);

        let calculated_length = curve.length();
        let expected_length = 5.0;

        let epsilon = 0.001;
        assert!(
            (calculated_length - expected_length).abs() < epsilon,
            "Expected length to be roughly {expected_length}, got {calculated_length}"
        );
    }

    #[test]
    fn straight_bezier_length() {
        // Arrange: A perfectly straight line along the X-axis using 4 control points.
        // Total length = 10.0
        let points = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(3.333, 0.0, 0.0),
            Vec3::new(6.666, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
        ];
        let curve = DeflectionCurvePoints(points);

        let calculated_length = curve.length();
        let expected_length = 10.0;

        let epsilon = 0.001;
        assert!(
            (calculated_length - expected_length).abs() < epsilon,
            "Expected Bézier length to be roughly {expected_length}, got {calculated_length}"
        );
    }

    #[test]
    fn curved_bezier_length() {
        // Arrange: A 90-degree corner curve mapped via a single 4-point Bézier segment.
        // Start at (0, 0, 0), pull towards (10, 0, 0), pull towards (10, 10, 0), end at (10, 10, 0).
        let points = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
        ];
        let curve = DeflectionCurvePoints(points);

        let calculated_length = curve.length();
        let expected_length = 15.864;

        let epsilon = 0.005;
        assert!(
            (calculated_length - expected_length).abs() < epsilon,
            "Expected curved Bézier length to be roughly {expected_length}, got {calculated_length}"
        );

        // The smoothed curve must cut the corner and be shorter than the raw path bounding box lines (20.0).
        assert!(
            calculated_length < 20.0,
            "A smoothed Bézier must cut the corner and be shorter than the raw control point distance."
        );
    }
}
