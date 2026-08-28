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
        app.add_plugins((
            AssemblyPlugin,
            ComponentsPlugin,
            ConflictPointsPlugin,
            CurvePlugin,
            GeometryPlugin,
            YieldPointsPlugin,
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
        world
            .run_system_cached(RoundaboutYieldPoints::generate)
            .unwrap();
        info!("Roundabout layout and conflict points successfully updated.");
    });
}
