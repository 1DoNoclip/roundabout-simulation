use crate::*;

pub(super) struct StatisticsPlugin;

impl Plugin for StatisticsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Statistics>();
    }
}

/// A collection of statistics to be used later when analysing the results.
#[derive(Default, Reflect, Resource)]
pub struct Statistics {
    /// The number of vehicles which have fully traversed the junction and have now despawned.
    pub total_vehicles_passed: usize,
}
