use crate::*;
use bevy_inspector_egui::bevy_egui::prelude::*;

pub(super) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, ui_settings_changed)
            .add_systems(EguiPrimaryContextPass, draw_window);
    }
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
            ApplyUiSettings::UpdateFlowRates => commands.run_system_cached(update_flow_rates),
            ApplyUiSettings::ApplyFlowRates => warn!("Not implemented."),
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
    statistics: Res<Statistics>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let screen_height = ctx.content_rect().size().y - 60.0;
    egui::Window::new("Interface")
        .fixed_size([350.0, screen_height])
        .show(contexts.ctx_mut()?, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::CollapsingHeader::new("Map Settings")
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::Grid::new("map_settings_grid")
                                .num_columns(2)
                                .show(ui, |ui| {
                                    // Number of lanes.
                                    ui.label("Number of lanes:");
                                    if ui.add(egui::Slider::new(
                                        &mut map_settings.number_of_lanes,
                                        1..=3,
                                    )).changed() {
                                        // Reason: The number of lanes affects where each lane goes to.
                                        apply_writer.write(ApplyUiSettings::UpdateFlowRates);
                                    }
                                    ui.end_row();

                                    // Speed limit.
                                    ui.label("Speed limit:");
                                    ui.horizontal(|ui| {
                                        let (mut display_value, range) = match map_settings
                                            .current_ui_speed_unit
                                        {
                                            SpeedUnit::MeterPerSecond => (
                                                map_settings
                                                    .speed_limit()
                                                    .get::<meter_per_second>(),
                                                (0.0..=27.8),
                                            ),
                                            SpeedUnit::MilePerHour => (
                                                map_settings.speed_limit().get::<mile_per_hour>(),
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
                                            map_settings.speed_limit = match map_settings
                                                .current_ui_speed_unit
                                            {
                                                SpeedUnit::MeterPerSecond => {
                                                    Velocity::new::<meter_per_second>(display_value)
                                                }
                                                SpeedUnit::MilePerHour => {
                                                    Velocity::new::<mile_per_hour>(display_value)
                                                }
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
                                    let mut radius_meter = map_settings.radius().get::<meter>();
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut radius_meter, 10.0..=80.0)
                                                .suffix("m")
                                                .max_decimals(1)
                                                .step_by(0.1)
                                                .drag_value_speed(0.025),
                                        )
                                        .changed()
                                    {
                                        map_settings.radius = Length::new::<meter>(radius_meter);
                                        // Prevent deflection radius from exceeding radius.
                                        map_settings.deflection_radius = map_settings
                                            .deflection_radius()
                                            .min(map_settings.radius());
                                    }
                                    ui.end_row();

                                    // Deflection radius.
                                    ui.label("Deflection radius:");
                                    let mut deflection_radius_meter =
                                        map_settings.deflection_radius().get::<meter>();
                                    // Cap the max deflection radius to the radius to prevent panicking.
                                    let max_value_meter = map_settings.radius().get::<meter>();
                                    if ui
                                        .add(
                                            egui::Slider::new(
                                                &mut deflection_radius_meter,
                                                5.0..=max_value_meter,
                                            )
                                            .suffix("m")
                                            .max_decimals(1)
                                            .step_by(0.1)
                                            .drag_value_speed(0.025),
                                        )
                                        .changed()
                                    {
                                        map_settings.deflection_radius =
                                            Length::new::<meter>(deflection_radius_meter);
                                    }
                                    ui.end_row();
                                });

                            // Arms.
                            egui::CollapsingHeader::new("Arms")
                                .default_open(true)
                                .show(ui, |ui| {
                                    let number_of_lanes = map_settings.number_of_lanes();
                                    // let arm_settings_view = map_settings.clone().arms;
                                    for (arm_index, arm_settings) in
                                        map_settings.arms.iter_mut().enumerate()
                                    {
                                        egui::Grid::new(("arm_settings_grid_", arm_index))
                                            .num_columns(2)
                                            .show(ui, |ui| {
                                                ui.label(format!("Arm {arm_index}"));
                                                ui.end_row();

                                                ui.label("Arm angle:");
                                                let mut angle_degree =
                                                    arm_settings.angle().as_degrees();
                                                if ui
                                                    .add(
                                                        egui::DragValue::new(&mut angle_degree)
                                                            .speed(1.0)
                                                            .suffix("°"),
                                                    )
                                                    .changed()
                                                {
                                                    arm_settings.angle =
                                                        Rot2::degrees(angle_degree);
                                                    // Reason: The arm angle may have affected what order the
                                                    // arms are, requiring updating flow rate mappings.
                                                    apply_writer.write(ApplyUiSettings::UpdateFlowRates);
                                                }
                                                ui.end_row();

                                                egui::CollapsingHeader::new("Flow rates")
                                                    .id_salt(("flow_rates_header_", arm_index))
                                                    .show(ui, |ui| {
                                                        for lane_index in 0..number_of_lanes {
                                                            egui::Grid::new((
                                                                "flow_rates_grid_",
                                                                lane_index,
                                                            ))
                                                            .num_columns(2)
                                                            .show(ui, |ui| {
                                                                ui.label(format!(
                                                                    "Lane {lane_index}"
                                                                ));
                                                                let flow_rates = &mut arm_settings
                                                                    .arm_flow_rates[lane_index];
                                                                egui::Grid::new(
                                                                    ("flow_rates_inner_grid_", lane_index)
                                                                ).num_columns(2)
                                                                .show(ui, |ui| {
                                                                for (exit_arm_index, flow_rate) in
                                                                    flow_rates
                                                                {
                                                                    ui.label(format!("To arm {exit_arm_index}"));
                                                                    let mut flow_per_hour =
                                                                        flow_rate.get::<per_hour>()
                                                                            as u32;
                                                                    if ui
                                                                        .add(
                                                                            egui::DragValue::new(
                                                                                &mut flow_per_hour
                                                                            )
                                                                            .range(0..=3600)
                                                                            .suffix(" per hour"),
                                                                        )
                                                                        .changed()
                                                                    {
                                                                        *flow_rate = Frequency::new::<per_hour>(flow_per_hour as f32);
                                                                        apply_writer.write(ApplyUiSettings::ApplyFlowRates);
                                                                    }
                                                                    ui.end_row();
                                                                }
                                                                // Add some space between each arm's flow rate controls.
                                                                ui.allocate_space(egui::vec2(0.0, 20.0));
                                                                });
                                                            });
                                                        }
                                                    });
                                                ui.end_row();

                                                ui.label("Speed limit override:");
                                                ui.horizontal(|ui| {
                                                    let mut is_overridden = arm_settings
                                                        .speed_limit_override()
                                                        .is_some();
                                                    if ui.checkbox(&mut is_overridden, "").changed()
                                                    {
                                                        if is_overridden {
                                                            arm_settings.speed_limit_override =
                                                                Some(Speed::default());
                                                        } else {
                                                            arm_settings.speed_limit_override =
                                                                None;
                                                        }
                                                    }
                                                });
                                                ui.end_row();
                                            });
                                        ui.add_space(8.0);
                                    }
                                });

                            if ui.button("Apply changes and reset statistics").clicked() {
                                apply_writer.write(ApplyUiSettings::Map);
                            }
                        });

                    ui.add_space(15.0);

                    egui::CollapsingHeader::new("Simulation Settings")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (label, button_label) = if simulation_settings.paused() {
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
                                    .selectable_value(
                                        &mut simulation_settings.time_speed_factor,
                                        0.25,
                                        "x0.25",
                                    )
                                    .changed();
                                simulation_speed_changed |= ui
                                    .selectable_value(
                                        &mut simulation_settings.time_speed_factor,
                                        0.5,
                                        "x0.5",
                                    )
                                    .changed();
                                simulation_speed_changed |= ui
                                    .selectable_value(
                                        &mut simulation_settings.time_speed_factor,
                                        1.0,
                                        "Real time",
                                    )
                                    .changed();
                            });
                            ui.horizontal(|ui| {
                                simulation_speed_changed |= ui
                                    .selectable_value(
                                        &mut simulation_settings.time_speed_factor,
                                        2.0,
                                        "x2",
                                    )
                                    .changed();
                                simulation_speed_changed |= ui
                                    .selectable_value(
                                        &mut simulation_settings.time_speed_factor,
                                        4.0,
                                        "x4",
                                    )
                                    .changed();
                                simulation_speed_changed |= ui
                                    .selectable_value(
                                        &mut simulation_settings.time_speed_factor,
                                        8.0,
                                        "x8",
                                    )
                                    .changed();
                                simulation_speed_changed |= ui
                                    .selectable_value(
                                        &mut simulation_settings.time_speed_factor,
                                        16.0,
                                        "x16",
                                    )
                                    .changed();
                            });
                            if simulation_speed_changed {
                                apply_writer.write(ApplyUiSettings::SimulationSpeed);
                            }
                        });

                    ui.add_space(15.0);

                    egui::CollapsingHeader::new("Statistics")
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::Grid::new("statistics_grid")
                                .num_columns(2)
                                .show(ui, |ui| {
                                    ui.label("Total vehicles passed");
                                    ui.label(statistics.total_vehicles_passed().to_string());

                                    ui.end_row();

                                    ui.label("Minimum time to collision (TTC)");

                                    ui.end_row();

                                    ui.label("Max change in velocity (Δv)");

                                    ui.end_row();

                                    ui.label("Mean maximum deceleration");

                                    ui.end_row();

                                    ui.label("Average queue length");

                                    ui.end_row();

                                    ui.label("Maximum queue length");

                                    ui.end_row();

                                    ui.label("Mean delay");

                                    ui.end_row();

                                    ui.label("Maximum junction capacity");

                                    ui.end_row();
                                });
                        });
                });
        });

    Ok(())
}
