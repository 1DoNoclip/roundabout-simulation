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

fn draw_window(
    mut contexts: EguiContexts,
    mut map_settings: ResMut<MapSettings>,
    mut simulation_settings: ResMut<SimulationSettings>,
) -> Result {
    egui::Window::new("Map").show(contexts.ctx_mut()?, |ui| {
        // Number of lanes.
        ui.horizontal(|ui| {
            ui.label("Number of lanes:");
            ui.add(egui::Slider::new(&mut map_settings.number_of_lanes, 1..=3));
        });

        // Speed limit.
        ui.horizontal(|ui| {
            ui.label("Speed limit:");
            let (mut display_value, range) = match map_settings.current_ui_speed_unit {
                SpeedUnit::MeterPerSecond => (
                    map_settings.speed_limit.get::<meter_per_second>(),
                    (0.0..=27.8),
                ),
                SpeedUnit::MilePerHour => (
                    map_settings.speed_limit.get::<mile_per_hour>(),
                    (0.0..=62.1),
                ),
            };
            if ui
                .add(
                    egui::DragValue::new(&mut display_value)
                        .speed(0.1)
                        .range(range),
                )
                .changed()
            {
                map_settings.speed_limit = match map_settings.current_ui_speed_unit {
                    SpeedUnit::MeterPerSecond => Velocity::new::<meter_per_second>(display_value),
                    SpeedUnit::MilePerHour => Velocity::new::<mile_per_hour>(display_value),
                };
            }
            ui.selectable_value(
                &mut map_settings.current_ui_speed_unit,
                SpeedUnit::MeterPerSecond,
                "m/s",
            );
            ui.selectable_value(
                &mut map_settings.current_ui_speed_unit,
                SpeedUnit::MilePerHour,
                "mph",
            );
        });

        ui.horizontal(|ui| {
            ui.label("Radius:");
            let mut radius_meter = map_settings.radius.get::<meter>();
            if ui
                .add(egui::Slider::new(&mut radius_meter, 10.0..=80.0).suffix("m"))
                .changed()
            {
                map_settings.radius = Length::new::<meter>(radius_meter);
            }
        });

        ui.horizontal(|ui| {
            ui.label("Deflection radius:");
            let mut deflection_radius_meter = map_settings.deflection_radius.get::<meter>();
            // Cap the max deflection radius to the radius dynamically.
            let max_value_meter = map_settings.radius.get::<meter>();
            if ui
                .add(
                    egui::Slider::new(&mut deflection_radius_meter, 5.0..=max_value_meter)
                        .suffix("m"),
                )
                .changed()
            {
                map_settings.deflection_radius = Length::new::<meter>(deflection_radius_meter);
            }
        });

        // let mut arm_to_remove = None;
        ui.label("Arms:");
        ui.indent("arms_indent", |ui| {
            for (index, arm) in map_settings.arms.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label("Arm angle:");
                    let mut angle_degree = arm.angle.as_degrees();
                    if ui
                        .add(
                            egui::DragValue::new(&mut angle_degree)
                                .speed(1.0)
                                .suffix("°"),
                        )
                        .changed()
                    {
                        arm.angle = Rot2::degrees(angle_degree);
                    }
                });
            }
        });
    });

    egui::Window::new("Simulation").show(contexts.ctx_mut()?, |ui| {
        ui.horizontal(|ui| {
            let (label, button_label) = if simulation_settings.paused {
                ("Paused:", "Play")
            } else {
                ("Playing:", "Pause")
            };
            ui.label(label);
            ui.toggle_value(&mut simulation_settings.paused, button_label);
        });

        ui.label("Time speed factor:");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut simulation_settings.time_speed_factor, 0.25, "x0.25");
            ui.selectable_value(&mut simulation_settings.time_speed_factor, 0.5, "x0.5");
            ui.selectable_value(&mut simulation_settings.time_speed_factor, 1.0, "Real time");
        });
        ui.horizontal(|ui| {
            ui.selectable_value(&mut simulation_settings.time_speed_factor, 2.0, "x2");
            ui.selectable_value(&mut simulation_settings.time_speed_factor, 4.0, "x4");
            ui.selectable_value(&mut simulation_settings.time_speed_factor, 8.0, "x8");
            ui.selectable_value(&mut simulation_settings.time_speed_factor, 16.0, "x16");
        });
    });

    egui::Window::new("Statistics").show(contexts.ctx_mut()?, |ui| {});

    Ok(())
}

#[derive(PartialEq)]
enum SpeedUnit {
    MeterPerSecond,
    MilePerHour,
}

#[derive(Resource)]
struct MapSettings {
    number_of_lanes: usize,
    speed_limit: Velocity,
    current_ui_speed_unit: SpeedUnit,
    radius: Length,
    deflection_radius: Length,
    arms: Vec<ArmSettings>,
}

impl Default for MapSettings {
    fn default() -> Self {
        MapSettings {
            number_of_lanes: 2,
            speed_limit: Velocity::new::<mile_per_hour>(30.0),
            current_ui_speed_unit: SpeedUnit::MilePerHour,
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
pub(crate) struct SimulationSettings {
    paused: bool,
    time_speed_factor: f32,
}

impl SimulationSettings {
    pub const fn paused(&self) -> bool {
        self.paused
    }

    pub const fn time_speed_factor(&self) -> f32 {
        self.time_speed_factor
    }
}

impl Default for SimulationSettings {
    fn default() -> Self {
        SimulationSettings {
            paused: false,
            time_speed_factor: 1.0,
        }
    }
}

struct ArmSettings {
    angle: Rot2,
    vehicles_per_hour: i32,
    speed_limit_override: Option<Speed>,
}

impl ArmSettings {
    const fn new(angle: Rot2, vehicles_per_hour: i32, speed_limit_override: Option<Speed>) -> Self {
        ArmSettings {
            angle,
            vehicles_per_hour,
            speed_limit_override,
        }
    }
}
