use bevy::{
    ecs::entity::EntityHashMap,
    math::cubic_splines::LinearSpline,
    prelude::*,
    render::{
        RenderPlugin,
        settings::{Backends, RenderCreation, WgpuSettings},
    },
};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use clap::Parser;
use std::time::Duration;

mod blueprint;
mod graphics;
mod layout;
mod simulation;

use blueprint::*;
use graphics::*;
use layout::*;
use simulation::*;

/// Sets up the roundabout simulation.
///
/// Add this plugin to the application to use the simulation.
pub struct AppSetupPlugin;

impl Plugin for AppSetupPlugin {
    fn build(&self, app: &mut App) {
        let cli_args = CliArgs::parse();

        if cli_args.no_render {
            app.add_plugins(
                DefaultPlugins
                    .set(RenderPlugin {
                        render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                            backends: Some(Backends::empty()), // Destroys active GPU driver attachments
                            ..default()
                        })),
                        ..default()
                    })
                    // Keep the OS canvas hidden from desktop presentation views.
                    .set(WindowPlugin {
                        primary_window: Some(Window {
                            title: "Simulation (Headless Keyboard Focus)".into(),
                            visible: false,
                            ..default()
                        }),
                        ..default()
                    }),
            );
        } else {
            // Normal visual display loop mode.
            app.add_plugins((
                DefaultPlugins,
                GraphicsPlugin,
                EguiPlugin::default(),
                WorldInspectorPlugin::default(),
            ));
        }

        // Register simulation code.
        app.add_plugins((BlueprintPlugin, LayoutPlugin, SimulationPlugin))
            .add_systems(
                Startup,
                (setup_roundabout_layout, setup_world, setup_simulation_time),
            )
            .add_systems(
                Update,
                (
                    handle_delayed_start.run_if(resource_exists::<StartupDelayTimer>),
                    set_time_speed,
                ),
            )
            .insert_resource(cli_args);
    }
}

#[derive(Parser, Debug, Resource)]
#[command(author, version, about)]
struct CliArgs {
    // Can use -p or --paused.
    // Automatically parses into false.
    /// Start the simulation paused.
    #[arg(short, long, default_value_t = false)]
    paused: bool,

    // Use --run-after=<SECONDS>.
    // Automatically parses into None if omitted.
    /// Initially pauses and delays playing the simulation by N real-world seconds.
    #[arg(long, value_name = "SECONDS")]
    run_after: Option<f32>,

    // Use --no-render.
    // Automatically parses into false if omitted.
    /// Run the simulation in headless mode, preventing rendering.
    #[arg(long, default_value_t = false)]
    no_render: bool,
}

#[derive(Resource)]
struct StartupDelayTimer(Timer);

fn setup_simulation_time(
    mut commands: Commands,
    args: Res<CliArgs>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    if let Some(delay_seconds) = args.run_after {
        virtual_time.pause();
        let timer = Timer::from_seconds(delay_seconds, TimerMode::Once);
        info!(
            "Simulation paused. Will start automatically after {} seconds.",
            timer.duration().as_secs_f32()
        );
        commands.insert_resource(StartupDelayTimer(timer));
    } else if args.paused {
        virtual_time.pause();
        info!("Simulation started in paused state.");
    }
}

fn handle_delayed_start(
    mut commands: Commands,
    real_time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut delay_timer: ResMut<StartupDelayTimer>,
) {
    delay_timer.0.tick(real_time.delta());

    if delay_timer.0.just_finished() {
        virtual_time.unpause();
        info!("Delayed start complete. Simulation unpaused.");
        commands.remove_resource::<StartupDelayTimer>();
    }
}

/// Use the number keys to set time speed.
///
/// 0 => paused, 1 => 0.25, 4 => 1.0, 9 => 50.0.
fn set_time_speed(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    use KeyCode::*;
    if keyboard_input.just_pressed(Digit0) {
        virtual_time.pause();
        return;
    }
    let digits = [
        Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    ];
    for (index, digit) in digits.into_iter().enumerate() {
        if keyboard_input.just_pressed(digit) {
            let key_number = index + 1;
            let speed = if key_number <= 4 {
                key_number as f32 * 0.25
            } else {
                (key_number - 4) as f32 * 10.0
            };
            virtual_time.set_relative_speed(speed);
            virtual_time.unpause();
            info!("Set speed to {speed:.2}x");
            break;
        }
    }
}

fn setup_world(mut commands: Commands) {
    commands.spawn((
        Name::new("Camera"),
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.5,
            ..OrthographicProjection::default_2d()
        }),
    ));
    commands.insert_resource(Statistics::default());
}

fn setup_roundabout_layout(mut commands: Commands) {
    let arm_blueprints = vec![
        ArmBlueprint::from_degrees(0.0, None, 1.0),
        ArmBlueprint::from_degrees(-90.0, None, 0.15),
        ArmBlueprint::from_degrees(-180.0, None, 1.0),
        ArmBlueprint::from_degrees(-270.0, None, 0.15),
    ];
    let circle_blueprint = CircleBlueprint::try_new(30.0, 12.5).expect("failed to create");
    commands.insert_resource(
        RoundaboutBlueprint::try_new(
            arm_blueprints,
            circle_blueprint,
            2,
            Speed::try_miles_per_hour(30.0).expect("failed to create"),
        )
        .expect("failed to create"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_roundabout_spawns_correct_topology() {
        use crate::assembly::assemble_roundabout;

        let mut app = App::new();

        let arm_blueprints = vec![
            ArmBlueprint::from_degrees(0.0, None, 0.5),
            ArmBlueprint::from_degrees(120.0, None, 0.5),
            ArmBlueprint::from_degrees(240.0, None, 0.5),
        ];
        let circle_blueprint = CircleBlueprint::try_new(20.0, 15.0).expect("failed to create");

        app.insert_resource(
            RoundaboutBlueprint::try_new(
                arm_blueprints,
                circle_blueprint,
                2,
                Speed::try_miles_per_hour(30.0).expect("failed to create"),
            )
            .expect("failed to create"),
        );

        app.add_systems(Update, assemble_roundabout);

        // First update enqueues the commands.
        app.update();
        // Flush forces Bevy to apply all queued commands to the World immediately.
        app.world_mut().flush();

        let mut query = app.world_mut().query::<&Segment>();
        assert!(query.iter(app.world()).count() > 0);
    }
}
