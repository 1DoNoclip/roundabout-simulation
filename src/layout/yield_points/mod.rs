//! Defines yield points for entering vehicles.

use crate::*;
use bevy::platform::collections::HashMap;

pub(super) struct YieldPointsPlugin;

impl Plugin for YieldPointsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RoundaboutYieldPoints>()
            .register_type::<YieldPoint>()
            .register_type::<YieldPointIndex>()
            .insert_resource(RoundaboutYieldPoints::default());
    }
}

#[derive(Default, Reflect, Resource)]
pub(crate) struct RoundaboutYieldPoints {
    points: HashMap<YieldPointIndex, YieldPoint>,
}

impl RoundaboutYieldPoints {
    pub fn generate(
        mut resource: ResMut<Self>,
        conflict_points: Res<RoundaboutConflictPoints>,
        entry_deflection_segments: Query<&Segment, With<segment_type::EntryDeflection>>,
        entry_line_segments: Query<&Segment, With<segment_type::EntryLine>>,
    ) {

    }
}

/// Defines a point where entering vehicles must yield to circulating traffic due to conflict points.
#[derive(Clone, Copy, Debug, Reflect)]
pub(crate) struct YieldPoint {
    /// Used in graphics for displaying yield points.
    /// No use in the simulation.
    yield_location: Vec3,
    /// The location of the yield point as a progress of the segment.
    progress: f32,
    segment_id: Entity,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Reflect)]
pub(crate) struct YieldPointIndex {
    arm_index: usize,
    /// The index of the entry (yielding) lane.
    lane_index: usize,
}
