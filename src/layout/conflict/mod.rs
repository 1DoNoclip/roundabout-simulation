//! Defines conflict points on the roundabout.

use crate::*;
use bevy::platform::collections::HashMap;

/// The points where lane `Segment`s cross over, but do not connect with a `Connection`.
///
/// This resource is essentially a cache of points to massively reduce the work required to get conflict points.
#[derive(Resource, Default)]
pub(crate) struct RoundaboutConflictPoints {
    points: HashMap<ConflictPointIndex, ConflictPoint>,
}

impl RoundaboutConflictPoints {
    pub(crate) const fn find(segments: Query<(Entity, &Segment)>) -> Self {

    }

    pub(crate) fn get(&self, conflict_point_index: ConflictPointIndex) -> Option<ConflictPoint> {
        self.points.get(&conflict_point_index).copied()
    }
}

/// Defines a conflict point (where vehicles have to cross) when merging onto the roundabout.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ConflictPoint {
    /// The distance from deflection start to the conflict.
    pub entry_distance_to_point: Distance,
    /// The distance from the inter arm sector (between Arm N-1 and Arm N) to the conflict.
    pub circulating_distance_to_point: Distance,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(crate) struct ConflictPointIndex {
    arm_index: usize,
    /// The lane index of the entry lane.
    entry_lane_index: usize,
    /// The lane index of the circulating lane.
    circulating_lane_index: usize,
}

impl ConflictPointIndex {
    /// Attempts to create a new `Self`.
    ///
    /// Returns `Some(Self)` if `circulating_lane_index` > `entry_lane_index`.
    ///
    /// Returns `None` if there is not a valid conflict point.
    pub const fn try_new(
        arm_index: usize,
        entry_lane_index: usize,
        circulating_lane_index: usize,
    ) -> Option<Self> {
        if circulating_lane_index > entry_lane_index {
            Some(ConflictPointIndex {
                arm_index,
                entry_lane_index,
                circulating_lane_index,
            })
        } else {
            None
        }
    }
}
