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
        app.add_plugins((CurvePlugin, GeometryPlugin));
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
