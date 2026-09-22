use crate::*;
use bevy_inspector_egui::bevy_egui::prelude::*;

pub(super) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ApplyUiSettings>()
            .insert_resource(MapSettings::default())
            .insert_resource(SimulationSettings::default())
            .add_systems(Update, ui_settings_changed)
            .add_systems(EguiPrimaryContextPass, draw_window);
    }
}

#[derive(Message)]
pub(crate) enum ApplyUiSettings {
    Map,
    SpeedLimit,
    SpeedLimitOverride,
    SimulationPlayPause,
    SimulationSpeed,
}

fn ui_settings_changed(mut commands: Commands, mut reader: MessageReader<ApplyUiSettings>) {
    // Expected reader.read().len() to be a max of 1 as the user should
    // struggle to change map and simulation settings in the same frame.
    if reader.read().len() > 1 {
        warn!(
            "Expected reader.read().len() to be a max of 1 as the user should
            struggle to change map and simulation settings in the same frame."
        );
    }
    for apply_ui_settings in reader.read() {
        match apply_ui_settings {
            ApplyUiSettings::Map => commands.run_system_cached(replace_roundabout_blueprint),
            ApplyUiSettings::SpeedLimit => commands.run_system_cached(update_speed_limit),
            ApplyUiSettings::SpeedLimitOverride => {
                commands.run_system_cached(update_speed_limit_overrides)
            }
            ApplyUiSettings::SimulationPlayPause => commands.run_system_cached(play_pause_time),
            ApplyUiSettings::SimulationSpeed => commands.run_system_cached(set_time_speed),
        }
    }
}

fn draw_window(
    mut contexts: EguiContexts,
    mut apply_writer: MessageWriter<ApplyUiSettings>,
    mut map_settings: ResMut<MapSettings>,
    mut simulation_settings: ResMut<SimulationSettings>,
) -> Result {
    egui::Window::new("Map Settings").show(contexts.ctx_mut()?, |ui| {
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
                        apply_writer.write(ApplyUiSettings::SpeedLimit);
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
            for (index, arm_settings) in map_settings.arms.iter_mut().enumerate() {
                egui::Grid::new(("arm_settings_grid_", index))
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Arm angle:");
                        let mut angle_degree = arm_settings.angle.as_degrees();
                        if ui
                            .add(
                                egui::DragValue::new(&mut angle_degree)
                                    .speed(1.0)
                                    .suffix("°"),
                            )
                            .changed()
                        {
                            arm_settings.angle = Rot2::degrees(angle_degree);
                        }
                        ui.end_row();

                        ui.label("Vehicles per hour:");
                        // ui.horizontal(|ui| {

                        // });
                        ui.end_row();

                        ui.label("Speed limit override:");
                        ui.horizontal(|ui| {
                            let mut is_overridden = arm_settings.speed_limit_override().is_some();
                            if ui.checkbox(&mut is_overridden, "").changed() {
                                if is_overridden {
                                    arm_settings.speed_limit_override = Some(Speed::default());
                                } else {
                                    arm_settings.speed_limit_override = None;
                                }
                                apply_writer.write(ApplyUiSettings::SpeedLimitOverride);
                            }
                        });
                        ui.end_row();
                    });
                ui.add_space(8.0);
            }
        });

        if ui.button("Apply changes").clicked() {
            apply_writer.write(ApplyUiSettings::Map);
        }
    });

    egui::Window::new("Simulation Settings").show(contexts.ctx_mut()?, |ui| {
        ui.horizontal(|ui| {
            let (label, button_label) = if simulation_settings.paused {
                ("Paused:", "Play")
            } else {
                ("Playing:", "Pause")
            };
            ui.label(label);
            if ui
                .toggle_value(&mut simulation_settings.paused, button_label)
                .changed()
            {
                apply_writer.write(ApplyUiSettings::SimulationPlayPause);
            }
        });

        let mut simulation_speed_changed = false;
        ui.label("Time speed factor:");
        ui.horizontal(|ui| {
            simulation_speed_changed |= ui
                .selectable_value(&mut simulation_settings.time_speed_factor, 0.25, "x0.25")
                .changed();
            simulation_speed_changed |= ui
                .selectable_value(&mut simulation_settings.time_speed_factor, 0.5, "x0.5")
                .changed();
            simulation_speed_changed |= ui
                .selectable_value(&mut simulation_settings.time_speed_factor, 1.0, "Real time")
                .changed();
        });
        ui.horizontal(|ui| {
            simulation_speed_changed |= ui
                .selectable_value(&mut simulation_settings.time_speed_factor, 2.0, "x2")
                .changed();
            simulation_speed_changed |= ui
                .selectable_value(&mut simulation_settings.time_speed_factor, 4.0, "x4")
                .changed();
            simulation_speed_changed |= ui
                .selectable_value(&mut simulation_settings.time_speed_factor, 8.0, "x8")
                .changed();
            simulation_speed_changed |= ui
                .selectable_value(&mut simulation_settings.time_speed_factor, 16.0, "x16")
                .changed();
        });
        if simulation_speed_changed {
            apply_writer.write(ApplyUiSettings::SimulationSpeed);
        }
    });

    egui::Window::new("Statistics").show(contexts.ctx_mut()?, |_ui| {});

    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
enum SpeedUnit {
    MeterPerSecond,
    MilePerHour,
}

#[derive(PartialEq, Resource)]
pub(crate) struct MapSettings {
    number_of_lanes: usize,
    speed_limit: Velocity,
    current_ui_speed_unit: SpeedUnit,
    radius: Length,
    deflection_radius: Length,
    arms: Vec<ArmSettings>,
}

impl MapSettings {
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

#[derive(Clone, Debug, PartialEq)]
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
