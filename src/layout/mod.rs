use crate::*;

pub(crate) mod assembly;
pub(crate) mod components;
pub(crate) mod conflict_points;
pub(crate) mod curve;
pub(crate) mod geometry;
pub(crate) mod yield_points;

use assembly::*;
pub(crate) use components::*;
pub(crate) use conflict_points::*;
pub(crate) use curve::*;
pub(crate) use geometry::*;
pub(crate) use yield_points::*;

pub(crate) struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RegenerateLayout>()
            .add_plugins((
                AssemblyPlugin,
                ComponentsPlugin,
                ConflictPointsPlugin,
                CurvePlugin,
                GeometryPlugin,
                YieldPointsPlugin,
            ))
            .add_systems(Startup, initialize_generation)
            .add_systems(Update, regenerate_layout);
    }
}

#[derive(Message)]
pub(crate) struct RegenerateLayout;

/// Creates a `RegenerateLayout` message.
fn initialize_generation(mut writer: MessageWriter<RegenerateLayout>) {
    writer.write(RegenerateLayout);
}

fn regenerate_layout(mut commands: Commands, mut reader: MessageReader<RegenerateLayout>) {
    // If one or more RegenerateLayout messages have been created.
    // If more than 1 messages have been created, only regenerate once.
    if reader.is_empty().not() {
        reader.clear();
        commands.queue(move |world: &mut World| {
            info!("RoundaboutBlueprint was created/changed. Running layout generation pipeline.");
            world.run_system_cached(assemble_roundabout).unwrap();
            world.flush();
            world
                .run_system_cached(RoundaboutConflictPoints::generate)
                .unwrap();
            world
                .run_system_cached(RoundaboutYieldPoints::generate)
                .unwrap();
            info!("Roundabout layout and conflict points successfully updated.");
        });
    }
}
