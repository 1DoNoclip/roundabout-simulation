use crate::*;
use bevy_egui::prelude::*;

pub(super) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(MapSettings::default())
            .insert_resource(SimulationSettings::default());
    }
}

#[derive(Resource)]
struct MapSettings {
    number_of_lanes: usize,
    speed_limit_metres_per_second: MetresPerSecond,
    radius_metres: f32,
    deflection_radius_metres: f32,
    arms: Vec<ArmSettings>,
}

impl Default for MapSettings {
    fn default() -> Self {
        MapSettings {
            number_of_lanes: 2,
            speed_limit_metres_per_second:
        }
    }
}

#[derive(Resource)]
struct SimulationSettings {
    time_speed: f32,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        SimulationSettings {
            time_speed: 1.0,
        }
    }
}

struct ArmSettings {
    arm_angle_degrees: f32,
    vehicles_per_hour: f32,
    speed_limit_metres_per_second_override: Option<f32>,
}
