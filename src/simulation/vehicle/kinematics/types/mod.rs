//! Contains `Speed`, `Distance` and `Acceleration` types.

use crate::*;

pub(super) struct TypesPlugin;

impl Plugin for TypesPlugin {
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
