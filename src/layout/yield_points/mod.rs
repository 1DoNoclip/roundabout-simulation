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
        roundabout_blueprint: Res<RoundaboutBlueprint>,
        entry_deflection_segments: Query<(Entity, &Segment), With<segment_type::EntryDeflection>>,
        entry_line_segments: Query<&Segment, With<segment_type::EntryLine>>,
    ) {
        let number_of_lanes = roundabout_blueprint.number_of_lanes();

        let mut yield_points: HashMap<YieldPointIndex, YieldPoint> = HashMap::new();
        for (entry_deflection_segment_id, entry_deflection_segment) in entry_deflection_segments {
            // Ensure that this segment merges (which they all should do anyway).
            let Connection::Merge { .. } = entry_deflection_segment.connection() else {
                warn!(
                    "Entry deflection segment ({entry_deflection_segment_id:?}) does not have a merge connection."
                );
                continue;
            };

            let Some(conflict_point) = get_first_conflict_point(
                &conflict_points,
                number_of_lanes,
                entry_deflection_segment,
            ) else {
                warn!(
                    "Failed to get a conflict point for segment {entry_deflection_segment_id:?}."
                );
                continue;
            };

            // Attempt to get the yield point as a point on the deflection curve.
            // If this progress is negative, then the yield point will be on the
            // entry line instead of the entry deflection curve.
            let deflection_yield_point_progress = conflict_point.entry_deflection_progress
                - (YieldPoint::YIELD_DISTANCE_FROM_CONFLICT_METRES
                    / entry_deflection_segment.length_metres());
            // This yield point is valid.
            if deflection_yield_point_progress >= 0.0 {
                let yield_location =
                    entry_deflection_segment.sample_clamped(deflection_yield_point_progress);
                yield_points.insert(
                    YieldPointIndex {
                        arm_index: entry_deflection_segment.arm_index(),
                        lane_index: entry_deflection_segment.lane_index(),
                    },
                    YieldPoint {
                        location: yield_location,
                        progress: deflection_yield_point_progress,
                        segment_id: entry_deflection_segment_id,
                    },
                );
            }
            // This yield point is invalid, so use the entry line instead.
            else {
                todo!();
            }
        }

        *resource = RoundaboutYieldPoints {
            points: yield_points,
        };
    }

    pub fn get(&self, index: YieldPointIndex) -> Option<YieldPoint> {
        self.points.get(&index).copied()
    }

    pub const fn points(&self) -> &HashMap<YieldPointIndex, YieldPoint> {
        &self.points
    }
}

/// Gets the first conflict point that a vehicle will reach.
///
/// Decides where the yield line is.
fn get_first_conflict_point(
    conflict_points: &RoundaboutConflictPoints,
    // Used since maximum lane index = `number_of_lanes` - 1.
    number_of_lanes: usize,
    entry_deflection_segment: &Segment,
) -> Option<ConflictPoint> {
    let (index, _) = ConflictPointIndex::try_new(
        entry_deflection_segment.arm_index(),
        entry_deflection_segment.lane_index(),
        number_of_lanes - 1,
    )?;
    conflict_points.get(index)
}

/// Defines a point where entering vehicles must yield to circulating traffic due to conflict points.
#[derive(Clone, Copy, Debug, Reflect)]
pub(crate) struct YieldPoint {
    /// Used in graphics for displaying yield points.
    /// No use in the simulation.
    pub location: Vec3,
    /// The location of the yield point as a progress of the segment.
    progress: f32,
    segment_id: Entity,
}

impl YieldPoint {
    pub const YIELD_DISTANCE_FROM_CONFLICT_METRES: f32 = 5.0;

    pub const fn progress(&self) -> f32 {
        self.progress
    }

    pub const fn segment_id(&self) -> Entity {
        self.segment_id
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Reflect)]
pub(crate) struct YieldPointIndex {
    arm_index: usize,
    /// The index of the entry (yielding) lane.
    lane_index: usize,
}

impl YieldPointIndex {
    pub const fn new(arm_index: usize, lane_index: usize) -> Self {
        YieldPointIndex {
            arm_index,
            lane_index,
        }
    }
}
