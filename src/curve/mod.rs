use crate::*;

pub mod circle;

pub trait CurveLength {
    fn length(&self) -> f32;
}

impl<C> CurveLength for C
where
    C: enterpolation::Curve<f32, Output = Vec3>,
{
    fn length(&self) -> f32 {
        let total_samples = 1000;
        let mut total_length = 0.0;

        let domain = self.domain();
        let (start_time, end_time) = (domain[0], domain[1]);
        let step = (end_time - start_time) / (total_samples as f32);

        let mut previous_point = self.eval(start_time);

        for i in 1..=total_samples {
            let time = start_time + (i as f32) * step;
            let current_point = self.eval(time);

            let distance = current_point.distance(previous_point);
            total_length += distance;

            previous_point = current_point;
        }

        total_length
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use enterpolation::{bspline::BSpline, linear::Linear};

    #[test]
    fn test_linear_length() {
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
            .unwrap();

        let calculated_length = curve.length();
        let expected_length = 7.0;

        let epsilon = 0.001;
        assert!(
            (calculated_length - expected_length).abs() < epsilon,
            "Expected length to be roughly {expected_length}, got {calculated_length}"
        );
    }

    #[test]
    fn test_straight_bspline_length() {
        // Arrange points in a perfectly straight line along the X-axis.
        // Total geometric length = 10.0
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
            .unwrap();

        let calculated_length = curve.length();
        let expected_length = 10.0;

        // Splines require high-resolution sampling to approximate accurately,
        let epsilon = 0.001;
        assert!(
            (calculated_length - expected_length).abs() < epsilon,
            "Expected B-Spline length to be roughly {expected_length}, got {calculated_length}"
        );
    }
}
