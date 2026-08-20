//! Defines conflict points on the roundabout.

use crate::*;
use bevy::platform::collections::HashMap;

/// The points where lane `Segment`s cross over, but do not connect with a `Connection`.
#[derive(Resource, Default)]
pub(crate) struct RoundaboutConflictPoints {
    points: HashMap<ConflictPointIndex, ConflictPoint>,
}

impl RoundaboutConflictPoints {
    pub(crate) const fn new(points: HashMap<ConflictPointIndex, ConflictPoint>) -> Self {
        RoundaboutConflictPoints { points }
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
    pub arm_index: usize,
    /// The lane index of the entry lane.
    pub entry_lane_index: usize,
    /// The lane index of the circulating lane.
    pub circulating_lane_index: usize,
}
