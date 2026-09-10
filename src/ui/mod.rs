use crate::*;
use bevy_inspector_egui::bevy_egui::prelude::*;
use std::marker::PhantomData;

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
    mut map_settings: ResMut<MapSettings<InProgress>>,
    mut simulation_settings: ResMut<SimulationSettings>,
) -> Result {
    let map_settings = map_settings.bypass_change_detection();

    egui::Window::new("Map").show(contexts.ctx_mut()?, |ui| {
        egui::Grid::new("map_settings_grid")
            .num_columns(2)
            .show(ui, |ui| {
                // Number of lanes.
                ui.label("Number of lanes:");
                ui.add(egui::Slider::new(&mut map_settings.number_of_lanes, 1..=3));
                ui.end_row();

                // Speed limit.
                ui.label("Speed limit:");
                ui.horizontal(|ui| {
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
                            SpeedUnit::MeterPerSecond => {
                                Velocity::new::<meter_per_second>(display_value)
                            }
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
                ui.end_row();

                // Radius.
                ui.label("Radius:");
                let mut radius_meter = map_settings.radius.get::<meter>();
                if ui
                    .add(egui::Slider::new(&mut radius_meter, 10.0..=80.0).suffix("m"))
                    .changed()
                {
                    map_settings.radius = Length::new::<meter>(radius_meter);
                }
                ui.end_row();

                // Deflection radius.
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
                ui.end_row();
            });

        // Arms.
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
                ui.horizontal(|ui| {
                    ui.label("Vehicles per hour:");
                });
                ui.add_space(8.0);
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

pub(crate) struct Applied;
struct InProgress;

#[derive(PartialEq, Resource)]
pub(crate) struct MapSettings<S> {
    state: PhantomData<S>,
    number_of_lanes: usize,
    speed_limit: Velocity,
    current_ui_speed_unit: SpeedUnit,
    radius: Length,
    deflection_radius: Length,
    arms: Vec<ArmSettings>,
}

impl MapSettings<InProgress> {
    fn apply_settings(in_progress: Res<Self>, applied: ResMut<MapSettings<Applied>>) {

    }
}

impl MapSettings<Applied> {
    pub const fn number_of_lanes(&self) -> usize {
        self.number_of_lanes
    }

    pub const fn speed_limit(&self) -> Velocity {
        self.speed_limit
    }

    pub const fn radius(&self) -> Length {
        self.radius
    }

    pub const fn deflection_radius(&self) -> Length {
        self.deflection_radius
    }

    pub fn arms(&self) -> &[ArmSettings] {
        &self.arms
    }
}

impl Default for MapSettings<InProgress> {
    fn default() -> Self {
        MapSettings {
            state: PhantomData,
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

#[derive(PartialEq)]
pub(crate) struct ArmSettings {
    angle: Rot2,
    vehicles_per_hour: u32,
    speed_limit_override: Option<Speed>,
}

impl ArmSettings {
    const fn new(angle: Rot2, vehicles_per_hour: u32, speed_limit_override: Option<Speed>) -> Self {
        ArmSettings {
            angle,
            vehicles_per_hour,
            speed_limit_override,
        }
    }

    pub const fn angle(&self) -> Rot2 {
        self.angle
    }

    pub const fn vehicles_per_hour(&self) -> u32 {
        self.vehicles_per_hour
    }

    pub const fn speed_limit_override(&self) -> Option<Speed> {
        self.speed_limit_override
    }
}
