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
            CurvePlugin,
            GeometryPlugin,
        ))
        .add_systems(
            Update,
            // Only runs if the blueprint resource has changed since last frame.
            assemble_roundabout.run_if(resource_changed::<RoundaboutBlueprint>),
        );
    }
}
