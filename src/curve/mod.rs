use bevy::prelude::*;
use enterpolation::{Curve, Equidistant, Identity, Signal, linear::Linear};

pub fn linear_length(linear: Linear<Equidistant<f32>, [Vec3; 2], Identity>) -> f32 {
    let total_samples = 1000;
    let mut total_length = 0.0;

    let domain = linear.domain();
    let (start_time, end_time) = (domain[0], domain[1]);
    let step = (end_time - start_time) / (total_samples as f32);

    let mut previous_point = linear.eval(start_time);

    for i in 1..=total_samples {
        let time = start_time + (i as f32) * step;
        let current_point = linear.eval(time);

        let distance = current_point.distance(previous_point);
        total_length += distance;

        previous_point = current_point;
    }

    total_length
}
