use crate::*;

pub(crate) use uom::si::{
    f32::Frequency,
    frequency::{cycle_per_minute, hertz},
};

pub(super) struct StatisticsPlugin;

impl Plugin for StatisticsPlugin {
    fn build(&self, _app: &mut App) {}
}

/// A collection of statistics to be used later when analysing the results.
#[derive(Default, Resource)]
pub(crate) struct Statistics {
    /// The number of vehicles which have fully traversed the junction and have now despawned.
    total_vehicles_passed: u32,
    /// The minimum time-to-collision.
    minimum_time_to_collision: UomTime,
    /// The maximum change in velocity that would happen if two vehicles were to crash.
    max_delta_v: Velocity,
    /// The mean maximum deceleration to avoid a collision.
    mean_maximum_deceleration: Acceleration,
    average_queue_length: Length,
    maximum_queue_length: Length,
    /// The (time it would take for a vehicle to traverse an empty road) - (actual time taken).
    delay_per_vehicle: UomTime,
    spawn_rate: Frequency,
    /// Can be compared against `spawn_rate` to determine maximum junction capacity.
    completed_trip_rate: Frequency,
}

impl Statistics {
    pub fn increment_total_vehicles_passed(&mut self) {
        self.total_vehicles_passed += 1;
    }
}
