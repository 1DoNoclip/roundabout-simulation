use crate::*;
use bevy::math::cubic_splines::LinearSpline;

pub struct CurvePlugin;

impl Plugin for CurvePlugin {
    fn build(&self, _app: &mut App) {}
}

/// A curve type used for segment evaluators.
pub trait SegmentCurve: CurveLength + IntoEvaluator + Send + Sync + 'static {}
// Blanket implementation.
impl<T> SegmentCurve for T where T: CurveLength + IntoEvaluator + Send + Sync + 'static {}

/// The ability to get a length of a curve.
pub trait CurveLength {
    fn length(&self) -> f32;
}

impl CurveLength for LinearSpline<Vec3> {
    fn length(&self) -> f32 {
        self.points
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .sum()
    }
}

impl CurveLength for CubicBezier<Vec3> {
    fn length(&self) -> f32 {
        const TOTAL_SAMPLES: usize = 1_000;

        match self.to_curve() {
            Ok(curve) => curve
                .iter_positions(TOTAL_SAMPLES)
                .collect::<Vec<_>>()
                .windows(2)
                .map(|pair| pair[0].distance(pair[1]))
                .sum(),
            Err(error) => {
                eprintln!("failed to convert CubicBezier into CubicCurve: {error}");
                0.0
            }
        }
    }
}

/// The ability to convert a curve into an evaluator function.
pub trait IntoEvaluator {
    fn into_evaluator(self) -> Box<dyn Fn(f32) -> Vec3 + Send + Sync + 'static>;
}

impl IntoEvaluator for LinearSpline<Vec3> {
    fn into_evaluator(self) -> Box<dyn Fn(f32) -> Vec3 + Send + Sync + 'static> {
        let curve = self
            .to_curve()
            .expect("failed to convert LinearSpline into CubicCurve");
        Box::new(move |time| curve.sample_clamped(time))
    }
}

impl IntoEvaluator for CubicBezier<Vec3> {
    fn into_evaluator(self) -> Box<dyn Fn(f32) -> Vec3 + Send + Sync + 'static> {
        let curve = self
            .to_curve()
            .expect("failed to convert CubicBezier into CubicCurve");
        Box::new(move |time| curve.sample_clamped(time))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_length() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(3.0, 4.0, 0.0),
        ];
        let curve = LinearSpline::new(points);

        let calculated_length = curve.length();
        let expected_length = 7.0;

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
        let curve = CubicBezier::new([points]);

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
        let curve = CubicBezier::new([points]);

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
