//! Contains `Distance`, `Speed` and `Acceleration` types.

use crate::*;

pub(super) struct UnitsPlugin;

impl Plugin for UnitsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Distance>()
            .register_type::<Speed>()
            .register_type::<Acceleration>();
    }
}

pub(crate) use distance::*;
mod distance {
    use super::*;

    /// A distance, can be used as a distance between vehicles.
    #[derive(Clone, Copy, Debug, Deref, DerefMut, Reflect)]
    pub(crate) struct Distance(Meters);

    impl Distance {
        pub fn try_new(distance: impl Into<Meters>) -> Result<Self, &'static str> {
            let distance = distance.into();
            if *distance >= 0.0 {
                Ok(Distance(distance))
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
    #[derive(Clone, Component, Copy, Debug, Deref, DerefMut, Reflect)]
    pub(crate) struct Speed(MetersPerSecond);

    impl Speed {
        pub const ZERO: Self = Speed(MetersPerSecond(0.0));

        pub fn try_new(speed: impl Into<MetersPerSecond>) -> Result<Self, &'static str> {
            let speed = speed.into();
            if *speed >= 0.0 {
                Ok(Speed(speed))
            } else {
                Err("speed cannot be negative")
            }
        }

        pub fn min(self, other: Speed) -> Self {
            Speed(MetersPerSecond(f32::min(**self, **other)))
        }
    }

    #[derive(Clone, Copy, Debug, Deref, DerefMut, Reflect)]
    pub(crate) struct MetersPerSecond(pub f32);

    #[derive(Clone, Copy, Deref, DerefMut, Reflect)]
    pub(crate) struct MilesPerHour(pub f32);

    impl From<MilesPerHour> for MetersPerSecond {
        fn from(value: MilesPerHour) -> Self {
            MetersPerSecond(*value * 0.44704)
        }
    }

    #[derive(Clone, Copy, Debug, Deref, DerefMut, Reflect)]
    pub(crate) struct KilometersPerHour(pub f32);

    impl From<KilometersPerHour> for MetersPerSecond {
        fn from(value: KilometersPerHour) -> Self {
            MetersPerSecond(*value * (1.0 / 3.6))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn new_valid_speed() {
            let speed_limit = Speed::try_new(MetersPerSecond(13.4));
            assert!(speed_limit.is_ok())
        }

        #[test]
        fn new_invalid_speed() {
            let speed_limit = Speed::try_new(MetersPerSecond(-13.4));
            assert!(speed_limit.is_err())
        }

        #[test]
        fn miles_per_hour_valid_speed() {
            let speed_limit = Speed::try_new(MilesPerHour(30.0));
            assert!(speed_limit.is_ok())
        }

        #[test]
        fn miles_per_hour_invalid_speed() {
            let speed_limit = Speed::try_new(MilesPerHour(-30.0));
            assert!(speed_limit.is_err())
        }
    }
}

pub(crate) use acceleration::*;
mod acceleration {
    use super::*;

    /// An acceleration, can be used for acceleration and deceleration (with negative).
    #[derive(Clone, Component, Copy, Debug, Deref, DerefMut, Reflect)]
    pub(crate) struct Acceleration(pub MetersPerSecondSquared);

    impl Acceleration {
        pub const fn new(meters_per_second_squared: f32) -> Self {
            Acceleration(MetersPerSecondSquared(meters_per_second_squared))
        }
    }

    #[derive(Clone, Copy, Debug, Deref, DerefMut, Reflect)]
    pub(crate) struct MetersPerSecondSquared(pub f32);
}
