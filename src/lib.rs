use bevy::{
    ecs::entity::EntityHashMap, math::cubic_splines::LinearSpline, prelude::*,
    window::PrimaryWindow,
};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use clap::Parser;

mod blueprint;
mod graphics;
mod layout;
mod simulation;
mod ui;
mod utils;

use blueprint::*;
use graphics::*;
use layout::*;
use simulation::*;
use ui::*;
use utils::*;

/// Sets up the roundabout simulation.
///
/// Add this plugin to the application to use the simulation.
pub struct AppSetupPlugin;

impl Plugin for AppSetupPlugin {
    fn build(&self, app: &mut App) {
        let cli_args = CliArgs::parse();

        if cli_args.no_render {
            app.add_systems(Startup, setup_no_render_overlay);
        } else {
            app.add_plugins(GraphicsPlugin);
        }

        // Core Bevy & third-party plugins.
        app.insert_resource(cli_args)
            .add_plugins(DefaultPlugins)
            // Register simulation domain plugins.
            .add_plugins((BlueprintPlugin, LayoutPlugin, SimulationPlugin, UtilsPlugin))
            .add_systems(
                Startup,
                (maximise_window, setup_world, setup_simulation_time),
            )
            .add_systems(
                Update,
                (
                    handle_delayed_start.run_if(resource_exists::<StartupDelayTimer>),
                    set_time_speed.run_if(resource_changed::<SimulationSettings>),
                ),
            );

        if cli_args.enable_inspector || !cli_args.no_control_panel {
            app.add_plugins(EguiPlugin::default());
        }
        if cli_args.enable_inspector {
            app.add_plugins(WorldInspectorPlugin::default());
        }
        if !cli_args.no_control_panel {
            app.add_plugins(UiPlugin);
        }
    }
}

#[derive(Copy, Clone, Debug, Parser, Resource)]
#[command(author, version, about)]
struct CliArgs {
    // Can use `-p` or `--paused`.
    // Automatically parses into false if omitted.
    /// Start the simulation paused.
    #[arg(short, long, default_value_t = false)]
    paused: bool,

    // Use `--run-after=<SECONDS>`.
    // Automatically parses into None if omitted.
    /// Initially pauses and delays playing the simulation by N real-world seconds.
    #[arg(long, value_name = "SECONDS")]
    run_after: Option<f32>,

    /// Enables the `bevy_inspector_egui`.
    #[arg(long, alias = "ei", default_value_t = false)]
    enable_inspector: bool,

    /// Disables the control panel.
    #[arg(long, alias = "nc", default_value_t = false)]
    no_control_panel: bool,

    // Use `--nr` or `--no-render`.
    // Automatically parses into false if omitted.
    /// Run the simulation without rendering graphics.
    ///
    /// A blank window will still open to enable user input.
    #[arg(long, alias = "nr", default_value_t = false)]
    no_render: bool,
}

fn maximise_window(mut window_query: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = window_query.single_mut() {
        window.set_maximized(true);
    }
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
    mut virtual_time: ResMut<Time<Virtual>>,
    simulation_settings: Res<SimulationSettings>,
) {
    if simulation_settings.paused() {
        info!("Time has paused.");
        virtual_time.pause();
    } else {
        info!("Time has resumed.");
        virtual_time.unpause();
    }
    info!("Speed set to x{}", simulation_settings.time_speed_factor());
    virtual_time.set_relative_speed(simulation_settings.time_speed_factor());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_roundabout_spawns_correct_topology() {
        use crate::assembly::assemble_roundabout;

        let mut app = App::new();

        let arm_blueprints = vec![
            ArmBlueprint::new_degrees(0.0, None, 0.5),
            ArmBlueprint::new_degrees(120.0, None, 0.5),
            ArmBlueprint::new_degrees(240.0, None, 0.5),
        ];
        let circle_blueprint =
            CircleBlueprint::try_new(Length::new::<meter>(20.0), Length::new::<meter>(15.0))
                .expect("failed to create");

        app.insert_resource(
            RoundaboutBlueprint::try_new(
                arm_blueprints,
                circle_blueprint,
                2,
                Speed::try_new(Velocity::new::<mile_per_hour>(30.0)).expect("failed to create"),
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
