//! Define conflict points on the roundabout.

use crate::*;
use bevy::platform::collections::HashMap;

#[derive(Resource, Default)]
pub(crate) struct IntersectionConflictPoints {
    points: HashMap<ConflictPointIndex, ConflictPoint>,
}

/// Defines a conflict point (where vehicles have to cross) when merging onto the roundabout.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ConflictPoint {
    /// The distance from deflection start to the conflict.
    entry_distance_to_point: Distance,
    /// The distance from the inter arm sector (between Arm N-1 and Arm N) to the conflict.
    circulating_distance_to_point: Distance,
}

#[derive(Clone, Copy)]
pub(crate) struct ConflictPointIndex {
    arm_index: usize,
    /// The lane index of the entry lane.
    deflection_lane_index: usize,
    /// The lane index of the circulating lane.
    circulating_lane_index: usize,
}
