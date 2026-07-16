use crate::*;

pub struct SpeedLimitPlugin;

impl Plugin for SpeedLimitPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SpeedLimit>();
    }
}

#[derive(Clone, Component, Copy, Debug, Reflect)]
/// A road speed limit.
pub struct SpeedLimit {
    metres_per_second: f32,
}

impl SpeedLimit {
    pub fn new(metres_per_second: f32) -> Result<Self, String> {
        if metres_per_second < 0.0 || metres_per_second.is_nan() {
            Err(format!(
                "metres_per_second cannot be negative, found {metres_per_second}"
            ))
        } else {
            Ok(SpeedLimit { metres_per_second })
        }
    }

    pub fn from_miles_per_hour(miles_per_hour: f32) -> Result<Self, String> {
        let metres_per_second = miles_per_hour * 0.44704;
        SpeedLimit::new(metres_per_second)
    }
}

impl Into<f32> for SpeedLimit {
    fn into(self) -> f32 {
        self.metres_per_second
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_valid_speed() {
        let speed_limit = SpeedLimit::new(13.4);
        assert!(speed_limit.is_ok())
    }

    #[test]
    fn new_invalid_speed() {
        let speed_limit = SpeedLimit::new(-13.4);
        assert!(speed_limit.is_err())
    }

    #[test]
    fn from_miles_per_hour_valid_speed() {
        let speed_limit = SpeedLimit::from_miles_per_hour(30.0);
        assert!(speed_limit.is_ok())
    }

    #[test]
    fn from_miles_per_hour_invalid_speed() {
        let speed_limit = SpeedLimit::from_miles_per_hour(-30.0);
        assert!(speed_limit.is_err())
    }
}
