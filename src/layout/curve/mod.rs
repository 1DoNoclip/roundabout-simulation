use crate::*;

pub mod circle;

use bevy::math::cubic_splines::LinearSpline;
pub use circle::*;

pub struct CurvePlugin;

impl Plugin for CurvePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CirclePlugin);
    }
}

pub trait CurveLength {
    fn length(&self) -> f32;
}

pub trait IntoEvaluator {
    fn into_evaluator(self) -> Box<dyn Fn(f32) -> Vec3 + Send + Sync + 'static>;
}

impl CurveLength for LinearSpline<Vec3> {
    fn length(&self) -> f32 {
        self.points
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .sum()
    }
}

impl IntoEvaluator for LinearSpline<Vec3> {
    fn into_evaluator(self) -> Box<dyn Fn(f32) -> Vec3 + Send + Sync + 'static> {
        let curve = self
            .to_curve()
            .expect("failed to convert LinearSpline into CubicCurve");
        Box::new(move |time| curve.sample_clamped(time))
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

impl IntoEvaluator for CubicBezier<Vec3> {
    fn into_evaluator(self) -> Box<dyn Fn(f32) -> Vec3 + Send + Sync + 'static> {
        let curve = self
            .to_curve()
            .expect("failed to convert CubicBezier into CubicCurve");
        Box::new(move |time| curve.sample_clamped(time))
    }
}

// impl CurveLength for CubicCurve<Vec3> {
//     fn length(&self) -> f32 {
//         const TOTAL_SAMPLES: usize = 1_000;

//         self.iter_positions(TOTAL_SAMPLES)
//             .collect::<Vec<_>>()
//             .windows(2)
//             .map(|pair| pair[0].distance(pair[1]))
//             .sum()
//         }
// }

// impl<C> CurveLength for C
// where
//     C: Curve<Vec3>,
// {
//     /// Gets the length of the curve using numerical integration via chord length approximation.
//     fn length(&self) -> f32 {
//         const TOTAL_SAMPLES: usize = 1_000;
//         let mut total_length = 0.0;

//         let domain = self.domain();
//         let (start_time, end_time) = (domain[0], domain[1]);
//         let step = (end_time - start_time) / (TOTAL_SAMPLES as f32);

//         let mut previous_point = self.eval(start_time);

//         // Calculate the straight line distance between the points.
//         for i in 1..=TOTAL_SAMPLES {
//             let time = start_time + (i as f32) * step;
//             let current_point = self.eval(time);

//             let distance = current_point.distance(previous_point);
//             total_length += distance;

//             previous_point = current_point;
//         }

//         total_length
//     }
// }

// impl<C> CurveLength for C
// where
//     C: Curve<Vec3>,
// {
//     /// Gets the length of the curve using numerical integration via chord length approximation.
//     fn length(&self) -> f32 {
//         const TOTAL_SAMPLES: usize = 1_000;
//         let mut total_length = 0.0;

//         // Bevy's domain() returns an Interval struct containing the boundaries
//         let domain = self.domain();
//         let start_time = domain.start();
//         let end_time = domain.end();
//         let step = (end_time - start_time) / (TOTAL_SAMPLES as f32);

//         // Bevy uses .sample(t) instead of .eval(t)
//         let mut previous_point = self.sample_clamped(start_time);

//         for i in 1..=TOTAL_SAMPLES {
//             let time = start_time + (i as f32) * step;
//             let current_point = self.sample_clamped(time);

//             total_length += current_point.distance(previous_point);
//             previous_point = current_point;
//         }

//         total_length
//     }
// }

// impl CurveLength for LinearSpline<Vec3> {
//     fn length(&self) -> f32 {
//         self.points
//             .windows(2)
//             .map(|pair| pair[0].distance(pair[1]))
//             .sum()
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use enterpolation::{bspline::BSpline, linear::Linear};

    #[test]
    fn linear_length() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(3.0, 4.0, 0.0),
        ];

        let curve = Linear::builder()
            .elements(points)
            .equidistant::<f32>()
            .normalized()
            .build()
            .expect("failed to create Linear curve");

        let calculated_length = curve.length();
        let expected_length = 7.0;

        let epsilon = 0.001;
        assert!(
            (calculated_length - expected_length).abs() < epsilon,
            "Expected length to be roughly {expected_length}, got {calculated_length}"
        );
    }

    #[test]
    fn straight_bspline_length() {
        // Arrange points in a perfectly straight line along the X-axis.
        // Total length = 10.0
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.5, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(7.5, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
        ];

        let curve = BSpline::builder()
            .clamped()
            .elements(points)
            .equidistant::<f32>()
            .degree(3)
            .normalized()
            .constant::<4>()
            .build()
            .expect("failed to create B-spline");

        let calculated_length = curve.length();
        let expected_length = 10.0;

        // Splines require high-resolution sampling to approximate accurately,
        let epsilon = 0.001;
        assert!(
            (calculated_length - expected_length).abs() < epsilon,
            "Expected B-Spline length to be roughly {expected_length}, got {calculated_length}"
        );
    }

    #[test]
    fn curved_bspline_length() {
        // Arrange points in a 90-degree bend (L-shape).
        // Because the B-Spline rounds the corner, the actual arc length contracts to ~18.021.
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 5.0),
            Vec3::new(10.0, 0.0, 10.0),
        ];

        let curve = BSpline::builder()
            .clamped()
            .elements(points)
            .equidistant::<f32>()
            .degree(3)
            .normalized()
            .constant::<4>()
            .build()
            .expect("failed to create B-spline");

        let calculated_length = curve.length();
        // Calculated via 1000-point integration.
        let expected_length = 18.021;

        // Use a slightly wider epsilon margin due to less accuracy in curved B-spline.
        let epsilon = 0.005;
        assert!(
            (calculated_length - expected_length).abs() < epsilon,
            "Expected curved B-Spline length to be roughly {expected_length}, got {calculated_length}"
        );

        // The smoothed curve must be shorter than the distance along X and Y bounding box.
        assert!(
            calculated_length < 20.0,
            "A smoothed B-Spline must cut the corner and be shorter than the raw point distance."
        );
    }
}
