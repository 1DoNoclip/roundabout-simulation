use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Statistics {
    /// The number of vehicles which have fully traversed the junction and have now despawned
    pub total_vehicles_passed: usize,
}
