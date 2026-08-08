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
