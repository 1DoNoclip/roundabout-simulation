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
        app.add_systems(Startup, (setup_world, setup_roundabout_layout));
    }
}

fn setup_world(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.5,
            ..OrthographicProjection::default_2d()
        }),
    ));
    commands.insert_resource(Statistics::default());
}

fn setup_roundabout_layout(mut commands: Commands) {
    commands.insert_resource(IntersectionBlueprint {
        number_of_lanes: 1,
        deflection_radius: 15.0,
        speed_limit: Speed::from_miles_per_hour(30.0).expect("failed to create Speed"),
        arms: vec![
            ArmBlueprint {
                angle: Rot2::degrees(0.0),
            },
            ArmBlueprint {
                angle: Rot2::degrees(120.0),
            },
            ArmBlueprint {
                angle: Rot2::degrees(225.0),
            },
        ],
    });

    commands.insert_resource(RoundaboutCircleBlueprint { radius: 25.0 });
}
