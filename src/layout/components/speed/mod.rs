use std::ops::{Deref, DerefMut};

use crate::*;

pub struct SpeedPlugin;

impl Plugin for SpeedPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Speed>();
    }
}

#[derive(Clone, Copy, Debug, Reflect)]
/// A speed, can be used for vehicle speed and speed limit.
pub struct Speed {
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

    pub fn from_miles_per_hour(miles_per_hour: f32) -> Result<Self, String> {
        let metres_per_second = miles_per_hour * 0.44704;
        Speed::new(metres_per_second)
    }
}

impl Deref for Speed {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.metres_per_second
    }
}

impl DerefMut for Speed {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.metres_per_second
    }
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
