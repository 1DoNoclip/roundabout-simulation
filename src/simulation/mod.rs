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

    /// Pushes an end_arm_id onto the queue.
    pub fn push_back(&mut self, end_arm_id: Entity) {
        self.pending_destinations.push_back(end_arm_id);
    }

    pub fn drain(&mut self, count: usize) {
        self.pending_destinations.drain(0..count);
    }

    pub const fn pending_destinations(&self) -> &VecDeque<Entity> {
        &self.pending_destinations
    }
}
