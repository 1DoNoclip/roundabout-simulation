use crate::*;
use bevy_inspector_egui::bevy_egui::prelude::*;

pub(super) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MapSettings::default())
            .insert_resource(SimulationSettings::default())
            .add_systems(EguiPrimaryContextPass, draw_window);
    }
}

fn draw_window(mut contexts: EguiContexts) -> Result {
    egui::Window::new("Control Panel").show(contexts.ctx_mut()?, |ui| {
        if ui.button("Click me").clicked() {
            println!("clicked");
        }
    });
    Ok(())
}

#[derive(Resource)]
struct MapSettings {
    number_of_lanes: usize,
    speed_limit: Speed,
    radius: Length,
    deflection_radius: Length,
    arms: Vec<ArmSettings>,
}

impl Default for MapSettings {
    fn default() -> Self {
        MapSettings {
            number_of_lanes: 2,
            speed_limit: Speed::try_new(Velocity::new::<mile_per_hour>(30.0)).unwrap(),
            radius: Length::new::<meter>(30.0),
            deflection_radius: Length::new::<meter>(12.5),
            arms: vec![
                ArmSettings::new(Rot2::degrees(0.0), 1_000, None),
                ArmSettings::new(Rot2::degrees(-90.0), 1_000, None),
                ArmSettings::new(Rot2::degrees(-180.0), 1_000, None),
                ArmSettings::new(Rot2::degrees(-270.0), 1_000, None),
            ],
        }
    }
}

#[derive(Resource)]
struct SimulationSettings {
    time_speed_factor: f32,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        SimulationSettings {
            time_speed_factor: 1.0,
        }
    }
}

struct ArmSettings {
    arm_angle: Rot2,
    vehicles_per_hour: i32,
    speed_limit_override: Option<Speed>,
}

impl ArmSettings {
    const fn new(
        arm_angle: Rot2,
        vehicles_per_hour: i32,
        speed_limit_override: Option<Speed>,
    ) -> Self {
        ArmSettings {
            arm_angle,
            vehicles_per_hour,
            speed_limit_override,
        }
    }
}
