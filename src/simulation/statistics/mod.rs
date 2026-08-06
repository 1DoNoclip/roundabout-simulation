use crate::*;

pub(super) struct StatisticsPlugin;

impl Plugin for StatisticsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Statistics>();
    }
}

/// A collection of statistics to be used later when analysing the results.
#[derive(Default, Reflect, Resource)]
#[reflect(Resource)]
pub(crate) struct Statistics {
    /// The number of vehicles which have fully traversed the junction and have now despawned.
    total_vehicles_passed: u32,
}

impl Statistics {
    pub fn increment_total_vehicles_passed(&mut self) {
        self.total_vehicles_passed += 1;
    }
}
