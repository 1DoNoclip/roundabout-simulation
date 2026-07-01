use bevy::prelude::*;
use enterpolation::{Curve, Signal};
use std::f32::consts::TAU;

pub struct Circle {
    pub centre: Vec3,
    pub radius: f32,
}

impl Circle {
    pub fn new(centre: Vec3, radius: f32) -> Self {
        Circle { centre, radius }
    }
}

impl Signal<f32> for Circle {
    type Output = Vec3;

    fn eval(&self, input: f32) -> Self::Output {
        // The remainder of input / 1.0 (so that input is between 0.0 and 1.0).
        let progress = input.rem_euclid(1.0);
        // The angle around the circle in rads. TAU = 2π rads / 360°.
        let angle = progress * TAU;

        // The position relating to the progress around the circle.
        Vec3::new(
            self.centre.x + self.radius * angle.cos(),
            self.centre.y,
            self.centre.z + self.radius * angle.sin(),
        )
    }
}

impl Curve<f32> for Circle {
    fn domain(&self) -> [f32; 2] {
        [0.0, 1.0]
    }
}
