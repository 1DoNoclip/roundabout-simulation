use bevy::prelude::*;
use enterpolation::{Curve, linear::Linear};

pub fn length<C>(curve: &C) -> f32
where
    C: Curve<f32, Output = Vec3>,
{
    let total_samples = 1000;
    let mut total_length = 0.0;

    let domain = curve.domain();
    let (start_time, end_time) = (domain[0], domain[1]);
    let step = (end_time - start_time) / (total_samples as f32);

    let mut previous_point = curve.eval(start_time);

    for i in 1..=total_samples {
        let time = start_time + (i as f32) * step;
        let current_point = curve.eval(time);

        let distance = current_point.distance(previous_point);
        total_length += distance;

        previous_point = current_point;
    }

    total_length
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let calculated_length = length(&curve);
        let expected_length = 7.0;

        let epsilon = 0.001;
        assert!(
            (calculated_length - expected_length).abs() < epsilon,
            "Expected length to be roughly {expected_length}, got {calculated_length}"
        );
    }
}
