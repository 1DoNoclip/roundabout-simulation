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
    pub(crate) const fn find(
        entry_deflection_segments: Query<(Entity, &Segment), With<segment_type::EntryDeflection>>,
        intra_arm_sector_segments: Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
    ) -> Self {
    }

    pub(crate) fn get(&self, conflict_point_index: ConflictPointIndex) -> Option<ConflictPoint> {
        self.points.get(&conflict_point_index).copied()
    }
}

/// Returns `Some((f32, f32))` where `.0` is `entry_deflection`'s progress and `.1` is `intra_arm`'s progress.
fn get_entry_deflection_intra_arm_conflict_point(
    entry_deflection: &Segment,
    intra_arm_sector: &Segment,
) -> Option<(f32, f32)> {
    /// The maximum distance between the entry deflection and
    /// inter arm accepted to be considered to be overlapping.
    const MAX_ACCEPTED_DISTANCE: f32 = 2.5;
    const MAX_ACCEPTED_DISTANCE_SQUARED: f32 = MAX_ACCEPTED_DISTANCE * MAX_ACCEPTED_DISTANCE;
    const COARSE_STEPS: usize = 50;
    const REFINE_STEPS: usize = 10;

    // .0 is the progress and .1 is the associated position.
    // Using 2D here allows easier conversion of the project
    // to 3D as distance here does not need height differences.
    let entry_deflection_data: [(f32, Vec2); COARSE_STEPS] = std::array::from_fn(|index| {
        let progress = index as f32 / (COARSE_STEPS - 1) as f32;
        let position_3d = entry_deflection.sample_clamped(progress);
        let position_2d = Vec2::new(position_3d.x, position_3d.y);
        (progress, position_2d)
    });
    let sector_data: [(f32, Vec2); COARSE_STEPS] = std::array::from_fn(|index| {
        let progress = index as f32 / (COARSE_STEPS - 1) as f32;
        let position_3d = intra_arm_sector.sample_clamped(progress);
        let position_2d = Vec2::new(position_3d.x, position_3d.y);
        (progress, position_2d)
    });

    let mut best_entry_progress = 0.0;
    let mut best_sector_progress = 0.0;
    let mut min_distance_squared = f32::MAX;

    // Do a coarse grid search to find the closest pairs of progresses.
    for (entry_progress, entry_position) in entry_deflection_data {
        for (sector_progress, sector_position) in sector_data {
            let distance_squared = entry_position.distance_squared(sector_position);

            if distance_squared < min_distance_squared {
                min_distance_squared = distance_squared;
                best_entry_progress = entry_progress;
                best_sector_progress = sector_progress;
            }
        }
    }

    if min_distance_squared > MAX_ACCEPTED_DISTANCE_SQUARED {
        return None;
    }

    let mut entry_step = 0.5 / (COARSE_STEPS - 1) as f32;
    // Copy entry_step.
    let mut sector_step = entry_step;

    // Do a refined grid search to get more accurate results.
    for _ in 0..REFINE_STEPS {
        let offsets = [-1.0, 0.0, 1.0];
        let mut local_best_entry_progress = best_entry_progress;
        let mut local_best_sector_progress = best_sector_progress;

        for entry_offset in offsets {
            let test_entry_progress =
                (best_entry_progress + entry_offset * entry_step).clamp(0.0, 1.0);
            let entry_position_3d = entry_deflection.sample_clamped(test_entry_progress);
            let entry_position_2d = Vec2::new(entry_position_3d.x, entry_position_3d.y);

            for sector_offset in offsets {
                let sector_test_progress =
                    (best_sector_progress + sector_offset * sector_step).clamp(0.0, 1.0);
                let sector_position_3d = intra_arm_sector.sample_clamped(sector_test_progress);
                let sector_position_2d = Vec2::new(sector_position_3d.x, sector_position_3d.y);

                let distance_squared = entry_position_2d.distance_squared(sector_position_2d);

                if distance_squared < min_distance_squared {
                    min_distance_squared = distance_squared;
                    local_best_entry_progress = test_entry_progress;
                    local_best_sector_progress = sector_test_progress;
                }
            }
        }

        best_entry_progress = local_best_entry_progress;
        best_sector_progress = local_best_sector_progress;
        entry_step *= 0.5;
        sector_step *= 0.5;
    }

    Some((best_entry_progress, best_sector_progress))
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
