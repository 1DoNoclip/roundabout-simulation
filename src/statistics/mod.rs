use bevy::prelude::*;

#[derive(Default, Reflect, Resource)]
/// A collection of statistics to be used later when analysing the results.
pub struct Statistics {
    /// The number of vehicles which have fully traversed the junction and have now despawned.
    pub total_vehicles_passed: usize,
}
