use bevy::{ecs::entity::EntityHashMap, math::cubic_splines::LinearSpline, prelude::*};

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
        app.add_plugins((
            BlueprintPlugin,
            GraphicsPlugin,
            LayoutPlugin,
            SimulationPlugin,
        ))
        .add_systems(Startup, (setup_world, setup_roundabout_layout))
        .add_systems(Update, play_pause_time);
    }
}

// Temporary play/pause functionality before adding proper user input and UI.
fn play_pause_time(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        if virtual_time.is_paused() {
            virtual_time.unpause();
        } else {
            virtual_time.pause();
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
        // ArmBlueprint::from_degrees(60.0, None, 0.25),
        ArmBlueprint::from_degrees(0.0, None, 1.5),
        // ArmBlueprint::from_degrees(-60.0, None, 1.0),
        ArmBlueprint::from_degrees(-120.0, None, 2.0),
        // ArmBlueprint::from_degrees(-180.0, None, 4.0),
        ArmBlueprint::from_degrees(-240.0, None, 2.5),
    ];
    let circle_blueprint = CircleBlueprint::try_new(25.0, 12.5).expect("failed to create");
    commands.insert_resource(
        RoundaboutBlueprint::try_new(
            arm_blueprints,
            circle_blueprint,
            2,
            Speed::from_miles_per_hour(30.0).expect("failed to create"),
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
                Speed::from_miles_per_hour(30.0).expect("failed to create"),
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
