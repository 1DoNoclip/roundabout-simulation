use crate::*;

pub mod assembly;
pub mod components;
pub mod curve;
pub mod geometry;

pub use assembly::*;
pub use components::*;
pub use curve::*;
pub use geometry::*;

pub struct LayoutPlugin;

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
            // Only runs if the blueprint resources have changed since last frame.
            assemble_roundabout.run_if(
                // Uses `.or_eager` instead of `.or_else` so both change ticks are checked
                // and cleared on frame 1. Lazy evaluation (`.or_else`) short-circuits,
                // leaving the second blueprint's tick 'unread' and triggering a redundant
                // second assembly run on frame 2.
                // The simulation should be able to handle redundant roundabout rebuilds without issues,
                // but redundant rebuilds do result in unneccessary work to be carried out.
                resource_changed::<IntersectionBlueprint>
                    .or_eager(resource_changed::<RoundaboutCircleBlueprint>),
            ),
        );
    }
}

pub struct RoundaboutLayout {
    pub arms: Vec<ArmLayout>,
    pub central_island: CentralIslandLayout,
}

pub struct ArmLayout {
    pub angle_degrees: f32,
    pub entries: Vec<LaneGeometry>,
    pub exits: Vec<LaneGeometry>,
}

pub struct CentralIslandLayout {
    pub radius: f32,
}
