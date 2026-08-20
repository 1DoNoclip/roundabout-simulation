//! Contains `Speed`, `Distance` and `Acceleration` types.

use crate::*;

pub(super) struct KinematicsPlugin;

impl Plugin for KinematicsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Acceleration>()
            .register_type::<Distance>()
            .register_type::<Speed>();
    }
}

/// An acceleration, can be used for acceleration and deceleration (with negative).
#[derive(Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct Acceleration {
    metres_per_second_squared: f32,
}

impl Acceleration {
    pub const fn new(metres_per_second_squared: f32) -> Self {
        Acceleration {
            metres_per_second_squared,
        }
    }
}

/// A distance, can be used as a distance between vehicles.
#[derive(Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct Distance {
    metres: f32,
}

impl Distance {
    pub fn try_new(metres: f32) -> Result<Self, String> {
        if metres >= 0.0 {
            Ok(Distance { metres })
        } else {
            Err(format!("metres cannot be negative, found {metres}"))
        }
    }
}

/// A speed, can be used for vehicle speed and speed limit.
#[derive(Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct Speed {
    metres_per_second: f32,
}

impl Speed {
    pub fn new(metres_per_second: f32) -> Result<Self, String> {
        if metres_per_second < 0.0 || metres_per_second.is_nan() {
            Err(format!(
                "metres_per_second cannot be negative, found {metres_per_second}"
            ))
        } else {
            Ok(Speed { metres_per_second })
        }
    }

    /// Creates a `Speed` by converting `miles_per_hour` into metres per second.
    pub fn from_miles_per_hour(miles_per_hour: f32) -> Result<Self, String> {
        let metres_per_second = miles_per_hour * 0.44704;
        Speed::new(metres_per_second)
    }
}

pub(in crate::simulation) fn calculate_time_to_arrival(
    distance: Distance,
    speed: Speed,
    acceleration: Acceleration,
) -> Duration {
    // Dereference newtypes into primitives.
    let (distance, speed, acceleration) = (*distance, *speed, *acceleration);

    // Uses s = ut + (1/2)at^2
    if acceleration.abs() > 0.01 {
        // If accelerating / decelerating significantly, solve the quadratic equation.
        // v^2 + 2as.
        let discriminant = speed * speed + 2.0 * acceleration * distance;
        if discriminant > 0.0 {
            let time = (-speed + discriminant.sqrt()) / acceleration;
            if time > 0.0 {
                return Duration::from_secs_f32(time);
            }
        }
    }
    // Fallback to using constant speed time-to-arrival.
    Duration::from_secs_f32(distance / speed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_valid_speed() {
        let speed_limit = Speed::new(13.4);
        assert!(speed_limit.is_ok())
    }

    #[test]
    fn new_invalid_speed() {
        let speed_limit = Speed::new(-13.4);
        assert!(speed_limit.is_err())
    }

    #[test]
    fn from_miles_per_hour_valid_speed() {
        let speed_limit = Speed::from_miles_per_hour(30.0);
        assert!(speed_limit.is_ok())
    }

    #[test]
    fn from_miles_per_hour_invalid_speed() {
        let speed_limit = Speed::from_miles_per_hour(-30.0);
        assert!(speed_limit.is_err())
    }
}
