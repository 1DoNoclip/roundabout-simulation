//! Contains `Distance`, `Speed` and `Acceleration` types.

use crate::*;
use uom::ConstZero;

pub(crate) use uom::si::{
    acceleration::meter_per_second_squared,
    f32::{Acceleration, Length, Time as UomTime, Velocity},
    length::meter,
    time::second,
    velocity::{meter_per_second, mile_per_hour},
};

pub(super) struct UnitsPlugin;

impl Plugin for UnitsPlugin {
    fn build(&self, _app: &mut App) {}
}

pub(crate) use distance::*;
mod distance {
    use super::*;

    /// A distance, can be used as a distance between vehicles.
    #[derive(Clone, Copy, Debug, Default, Deref)]
    pub(crate) struct Distance(Length);

    impl Distance {
        pub const ZERO: Self = Distance(Length::ZERO);

        pub fn try_new(length: Length) -> Result<Self, &'static str> {
            if length.get::<meter>() >= 0.0 {
                Ok(Distance(length))
            } else {
                Err("distance cannot be negative")
            }
        }
    }

    #[derive(Clone, Copy, Debug, Deref, DerefMut, Reflect)]
    pub(crate) struct Meters(pub f32);
}

pub(crate) use velocity::*;
mod velocity {
    use super::*;

    /// A speed, can be used for vehicle speed and speed limit.
    #[derive(Clone, Component, Copy, Debug, Deref, Reflect)]
    pub(crate) struct Speed(#[reflect(ignore)] Velocity);

    impl Speed {
        pub const ZERO: Self = Speed(Velocity::ZERO);

        pub fn try_new(velocity: Velocity) -> Result<Self, &'static str> {
            if velocity.get::<meter_per_second>() >= 0.0 {
                Ok(Speed(velocity))
            } else {
                Err("speed cannot be negative")
            }
        }

        // /// Creates a new `Speed`, using `velocity` or clamping to `0.0` if `velocity` is negative.
        // pub fn new_clamped(velocity: Velocity) -> Self {
        //     Speed(velocity.max(Velocity::ZERO))
        // }

        /// Applies acceleration, ensuring that `self.0` always remains non-negative.
        pub fn apply_acceleration(&mut self, acceleration: Acceleration, delta_time: UomTime) {
            let new_velocity = self.0 + acceleration * delta_time;
            self.0 = new_velocity.max(Velocity::ZERO)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn new_valid_speed() {
            let speed_limit = Speed::try_new(Velocity::new::<meter_per_second>(13.4));
            assert!(speed_limit.is_ok())
        }

        #[test]
        fn new_invalid_speed() {
            let speed_limit = Speed::try_new(Velocity::new::<meter_per_second>(-13.4));
            assert!(speed_limit.is_err())
        }
    }
}

pub(crate) use acceleration::*;
mod acceleration {
    use super::*;

    /// An acceleration, can be used for acceleration and deceleration (with negative).
    #[derive(Clone, Component, Copy, Debug, Deref, DerefMut)]
    pub(crate) struct AccelerationComponent(Acceleration);

    impl AccelerationComponent {
        pub const fn new(acceleration: Acceleration) -> Self {
            AccelerationComponent(acceleration)
        }
    }
}
