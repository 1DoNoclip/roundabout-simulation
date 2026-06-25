use bevy::prelude::*;

#[derive(Default, Reflect, Resource)]
pub struct Statistics {
    /// The number of vehicles which have fully traversed the junction and have now despawned
    pub total_vehicles_passed: usize,
}
