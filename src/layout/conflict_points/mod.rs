//! Defines conflict points on the roundabout.

use crate::*;
use bevy::platform::collections::HashMap;

pub(super) struct ConflictPointsPlugin;

impl Plugin for ConflictPointsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RoundaboutConflictPoints>()
            .register_type::<ConflictPoint>()
            .register_type::<ConflictPointIndex>()
            .insert_resource(RoundaboutConflictPoints::default());
    }
}

/// The points where lane `Segment`s cross over, but do not connect with a `Connection`.
///
/// This resource is essentially a cache of points to massively reduce the work required to get conflict points.
#[derive(Default, Reflect, Resource)]
pub(crate) struct RoundaboutConflictPoints {
    points: HashMap<ConflictPointIndex, ConflictPoint>,
}

impl RoundaboutConflictPoints {
    pub fn generate(
        mut resource: ResMut<Self>,
        arms: Query<(Entity, &Arm)>,
        entry_deflection_segments: Query<&Segment, With<segment_type::EntryDeflection>>,
        intra_arm_sector_segments: Query<(Entity, &Segment), With<segment_type::IntraArmSector>>,
    ) {
        let mut conflict_points: HashMap<ConflictPointIndex, ConflictPoint> = HashMap::new();

        let sectors_by_arm: EntityHashMap<Vec<(Entity, &Segment)>> = intra_arm_sector_segments
            .iter()
            .fold(EntityHashMap::new(), |mut map, (id, segment)| {
                map.entry(segment.arm_id()).or_default().push((id, segment));
                map
            });

        for entry_deflection_segment in entry_deflection_segments {
            let (arm_id, arm) = arms
                .get(entry_deflection_segment.arm_id())
                .expect("expected Segment to point to a valid Arm entity");
            let arm_index = arm.index();

            // All sectors that are on the same arm as entry_deflection_segment.
            let same_arm_sectors = sectors_by_arm
                .get(&arm_id)
                .expect("expected to find matching sector Segments on this Arm");

            for &(id, sector_segment) in same_arm_sectors {
                let Some((conflict_point_index, is_merge)) = ConflictPointIndex::try_new(
                    arm_index,
                    entry_deflection_segment.lane_index(),
                    sector_segment.lane_index(),
                ) else {
                    continue;
                };
                if let Some(conflict_point) =
                    ConflictPoint::try_new(entry_deflection_segment, sector_segment, id, is_merge)
                {
                    conflict_points.insert(conflict_point_index, conflict_point);
                }
            }
        }

        *resource = RoundaboutConflictPoints {
            points: conflict_points,
        };
    }

    pub fn get(&self, index: ConflictPointIndex) -> Option<ConflictPoint> {
        self.points.get(&index).copied()
    }

    pub const fn points(&self) -> &HashMap<ConflictPointIndex, ConflictPoint> {
        &self.points
    }
}

/// Defines a conflict point (where vehicles have to cross) when merging onto the roundabout.
#[derive(Clone, Copy, Debug, Reflect)]
pub(crate) struct ConflictPoint {
    /// Used in graphics for displaying conflict points.
    /// No use in the simulation.
    pub location: Vec3,
    /// The conflict point as a progress along the entry deflection.
    ///
    /// Used to determine locations of `YieldPoint`s.
    pub entry_deflection_progress: f32,
    /// The conflict point as a progress along the intra arm sector.
    pub intra_arm_sector_progress: f32,
    pub intra_arm_sector_id: Entity,
}

impl ConflictPoint {
    fn try_new(
        entry_deflection: &Segment,
        intra_arm_sector: &Segment,
        intra_arm_sector_id: Entity,
        is_merge: bool,
    ) -> Option<Self> {
        let (conflict_location, entry_deflection_progress, intra_arm_sector_progress) = if is_merge
        {
            (intra_arm_sector.position_at(1.0), 1.0, 1.0)
        } else {
            ConflictPoint::get_entry_deflection_intra_arm_conflict_progresses(
                entry_deflection,
                intra_arm_sector,
            )?
        };
        Some(ConflictPoint {
            location: conflict_location,
            entry_deflection_progress,
            intra_arm_sector_progress,
            intra_arm_sector_id,
        })
    }

    /// Returns `Some((Vec3, f32, f32))` where:
    /// * `.0` is the location of the conflict point.
    /// * `.1` is `entry_deflection`'s progress.
    /// * `.2` is `intra_arm`'s progress.
    ///
    /// Returns `None` if a conflict point cannot be found.
    fn get_entry_deflection_intra_arm_conflict_progresses(
        entry_deflection: &Segment,
        intra_arm_sector: &Segment,
    ) -> Option<(Vec3, f32, f32)> {
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
            let position_3d = entry_deflection.position_at(progress);
            let position_2d = Vec2::new(position_3d.x, position_3d.y);
            (progress, position_2d)
        });
        let sector_data: [(f32, Vec2); COARSE_STEPS] = std::array::from_fn(|index| {
            let progress = index as f32 / (COARSE_STEPS - 1) as f32;
            let position_3d = intra_arm_sector.position_at(progress);
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
                    best_entry_progress = entry_progress;
                    best_sector_progress = sector_progress;
                    min_distance_squared = distance_squared;
                }
            }
        }

        if min_distance_squared > MAX_ACCEPTED_DISTANCE_SQUARED {
            return None;
        }

        let mut entry_step = 0.5 / (COARSE_STEPS - 1) as f32;
        // Copy entry_step.
        let mut sector_step = entry_step;

        let mut best_position = Vec3::ZERO;

        // Do a refined grid search to get more accurate results.
        for _ in 0..REFINE_STEPS {
            let offsets = [-1.0, 0.0, 1.0];
            let mut local_best_entry_progress = best_entry_progress;
            let mut local_best_sector_progress = best_sector_progress;

            for entry_offset in offsets {
                let test_entry_progress =
                    (best_entry_progress + entry_offset * entry_step).clamp(0.0, 1.0);
                let entry_position_3d = entry_deflection.position_at(test_entry_progress);
                let entry_position_2d = Vec2::new(entry_position_3d.x, entry_position_3d.y);

                for sector_offset in offsets {
                    let sector_test_progress =
                        (best_sector_progress + sector_offset * sector_step).clamp(0.0, 1.0);
                    let sector_position_3d = intra_arm_sector.position_at(sector_test_progress);
                    let sector_position_2d = Vec2::new(sector_position_3d.x, sector_position_3d.y);

                    let distance_squared = entry_position_2d.distance_squared(sector_position_2d);

                    if distance_squared < min_distance_squared {
                        best_position = entry_position_3d;
                        local_best_entry_progress = test_entry_progress;
                        local_best_sector_progress = sector_test_progress;
                        min_distance_squared = distance_squared;
                    }
                }
            }

            best_entry_progress = local_best_entry_progress;
            best_sector_progress = local_best_sector_progress;
            entry_step *= 0.5;
            sector_step *= 0.5;
        }

        Some((best_position, best_entry_progress, best_sector_progress))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Reflect)]
pub(crate) struct ConflictPointIndex {
    arm_index: usize,
    /// The index of the entry lane.
    entry_lane_index: usize,
    /// The index of the circulating lane.
    circulating_lane_index: usize,
}

impl ConflictPointIndex {
    /// Attempts to create a new `Self`.
    ///
    /// Returns `Some(Self)` if `circulating_lane_index` >= `entry_lane_index`.
    ///
    /// Returns `None` if there is not a valid conflict point.
    pub const fn try_new(
        arm_index: usize,
        entry_lane_index: usize,
        circulating_lane_index: usize,
    ) -> Option<(Self, bool)> {
        let is_merge = circulating_lane_index == entry_lane_index;
        if circulating_lane_index >= entry_lane_index {
            Some((
                ConflictPointIndex {
                    arm_index,
                    entry_lane_index,
                    circulating_lane_index,
                },
                is_merge,
            ))
        } else {
            None
        }
    }
}
