use std::collections::VecDeque;

use crate::*;

pub(crate) mod statistics;
pub(crate) mod vehicle;

pub(crate) use statistics::*;
pub(crate) use vehicle::*;

pub(crate) struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((StatisticsPlugin, VehiclePlugin))
            .add_systems(FixedUpdate, (spawn_vehicles, move_vehicles));
    }
}

#[derive(Component)]
pub(crate) struct VehicleSpawnQueue {
    /// Holds destination arms of vehicles waiting to spawn.
    pending_destinations: VecDeque<Entity>,
}

impl VehicleSpawnQueue {
    pub const fn new() -> Self {
        VehicleSpawnQueue {
            pending_destinations: VecDeque::new(),
        }
    }

    pub const fn pending_destinations(&self) -> &VecDeque<Entity> {
        &self.pending_destinations
    }
}
