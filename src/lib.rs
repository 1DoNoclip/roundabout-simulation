pub use bevy::{ecs::entity::EntityHashMap, math::cubic_splines::LinearSpline, prelude::*};
pub use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

pub mod blueprint;
pub mod graphics;
pub mod layout;
pub mod simulation;

pub use blueprint::*;
pub use graphics::*;
pub use layout::*;
pub use simulation::*;

pub struct AppSetupPlugin;

impl Plugin for AppSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_world, setup_roundabout_layout))
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
    commands.insert_resource(
        IntersectionBlueprint::try_new(
            vec![
                ArmBlueprint::from_degrees(60.0, None, 0.25),
                ArmBlueprint::from_degrees(0.0, None, 0.5),
                ArmBlueprint::from_degrees(-60.0, None, 1.0),
                ArmBlueprint::from_degrees(-120.0, None, 2.0),
                ArmBlueprint::from_degrees(-180.0, None, 4.0),
                ArmBlueprint::from_degrees(-240.0, None, 16.0),
            ],
            2,
            Speed::from_miles_per_hour(30.0).expect("failed to create"),
            12.5,
        )
        .expect("failed to create"),
    );

    commands.insert_resource(RoundaboutCircleBlueprint::try_new(25.0).expect("failed to create"));
}
