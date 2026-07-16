use crate::*;

pub mod components;
pub mod curve;
pub mod geometry;

pub use components::*;
pub use curve::*;
pub use geometry::*;

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
