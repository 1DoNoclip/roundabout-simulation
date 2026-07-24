use crate::*;

pub mod statistics;
pub mod vehicle;

pub use statistics::*;
pub use vehicle::*;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((StatisticsPlugin, VehiclePlugin))
            .add_systems(FixedUpdate, (spawn_vehicles, vehicle_movement));
    }
}
