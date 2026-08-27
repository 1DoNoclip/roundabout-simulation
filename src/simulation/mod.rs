use crate::*;
use std::collections::VecDeque;

pub(crate) mod statistics;
pub(crate) mod vehicle;

pub(crate) use statistics::*;
pub(crate) use vehicle::*;

pub(crate) struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((StatisticsPlugin, VehiclePlugin))
            .add_systems(
                FixedUpdate,
                (
                    spawn_vehicles,
                    (
                        calculate_accelerations,
                        update_vehicle_accelerations,
                        apply_accelerations,
                        move_vehicles,
                    )
                        .chain(),
                ),
            );
    }
}

#[derive(Component, Deref, DerefMut)]
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
}
