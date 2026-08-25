use crate::*;

pub(crate) mod assembly;
pub(crate) mod components;
pub(crate) mod conflict;
pub(crate) mod curve;
pub(crate) mod geometry;

use assembly::*;
pub(crate) use components::*;
pub(crate) use conflict::*;
pub(crate) use curve::*;
pub(crate) use geometry::*;

pub(crate) struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            AssemblyPlugin,
            ComponentsPlugin,
            ConflictPlugin,
            CurvePlugin,
            GeometryPlugin,
        ))
        .add_systems(Update, roundabout_blueprint_changed);
    }
}

fn roundabout_blueprint_changed(mut commands: Commands, blueprint: Res<RoundaboutBlueprint>) {
    // Only runs if the blueprint resource has changed.
    if !blueprint.is_changed() {
        return;
    }
    commands.queue(move |world: &mut World| {
        info!("RoundaboutBlueprint has changed. Running layout generation pipeline.");
        world.run_system_cached(assemble_roundabout).unwrap();
        world.flush();
        world
            .run_system_cached(RoundaboutConflictPoints::generate)
            .unwrap();
        info!("Roundabout layout and conflict points successfully updated.");
    });
}
