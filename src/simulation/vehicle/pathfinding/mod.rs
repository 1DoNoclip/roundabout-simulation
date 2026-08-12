use crate::*;
use rand::{
    distr::{Distribution, weighted::WeightedIndex},
    rngs::StdRng,
};

pub(super) struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Computes the route for a vehicle to follow.
///
/// Assumes that the `lane_index` is always constant throughout the route.
///
/// Returns `Some(Vec<Entity>)`, a vector of `Segment` entities if a route is found from start to end.
/// Returns `None` if a route is not found.
pub(super) fn calculate_route(
    arms: &Query<&Arm>,
    end_points: &Query<(Entity, &EndPoint)>,
    segments: &Query<&Segment>,
    spawn_point: &SpawnPoint,
    end_arm_index: usize,
) -> Result<Vec<Entity>, String> {
    let start_segment_id = spawn_point.segment();

    let mut route = vec![start_segment_id];
    loop {
        let route_segment_length = route.len();
        let number_of_segments = segments.iter().len();
        if route_segment_length > number_of_segments {
            return Err(format!(
                "number of segments in route ({route_segment_length}) exceeds number of segments {number_of_segments}"
            ));
        }
        let current_segment_id = *route
            .last()
            .ok_or_else(|| "next_segment should not be empty".to_owned())?;
        let current_segment = segments
            .get(current_segment_id)
            .map_err(|error| format!("failed to get Segment from Segment entity: {error}"))?;
        match *current_segment.connection() {
            Connection::Direct { next_segment_id } => {
                route.push(next_segment_id);
            }
            Connection::Diverge {
                exit_arm_index,
                exit_segment_id,
                circulating_segment_id,
            } => {
                if exit_arm_index == end_arm_index {
                    route.push(exit_segment_id);
                } else {
                    route.push(circulating_segment_id);
                }
            }
            Connection::EndPoint { end_point_id } => {
                let (_, end_point) = end_points.get(end_point_id).map_err(|error| {
                    format!("failed to get EndPoint from EndPoint entity: {error}")
                })?;
                let arm = arms
                    .get(end_point.arm())
                    .map_err(|error| format!("failed to get Arm from Arm entity: {error}"))?;
                if arm.index() == end_arm_index {
                    break;
                } else {
                    return Err("EndPoint did not match the wanted EndPoint".to_owned());
                }
            }
            Connection::Merge { next_segment_id } => {
                route.push(next_segment_id);
            }
        }
    }

    Ok(route)
}

pub(super) fn select_destination_arm(
    mut spawner_rng: &mut StdRng,
    destination_weights: &DestinationWeights,
) -> Entity {
    if destination_weights.is_empty() {
        panic!("Cannot select a destination arm from an empty destination_weights");
    }

    let arms = destination_weights.keys().cloned().collect::<Vec<_>>();
    let weights = destination_weights.values().cloned().collect::<Vec<_>>();

    let distribution = WeightedIndex::new(&weights)
        .expect("failed to create WeightedIndex, ensure that not every weight is zero");
    let selected_index = distribution.sample(&mut spawner_rng);
    arms[selected_index]
}

pub(super) fn select_lane_index(
    entry_arm: &Arm,
    exit_arm: &Arm,
    number_of_arms: usize,
    number_of_lanes: usize,
) -> usize {
    // Single-lane roundabouts always use lane 0.
    if number_of_lanes <= 1 {
        return 0;
    }

    let exit_rank = get_exit_rank(entry_arm, exit_arm, number_of_arms);
    let max_rank = number_of_arms - 1;

    // Clamp exit_rank so U-turns share highest rank with final exit.
    let rank = if exit_rank == 0 || exit_rank > max_rank {
        max_rank
    } else {
        exit_rank
    };

    let raw_progress = (rank - 1) as f32 / (max_rank - 1) as f32;
    // Adds quadratic bias (which delays using more inner lanes until later ranks).
    let biased_progress = raw_progress.powf(2.0);

    let inner_offset = (biased_progress * (number_of_lanes - 1) as f32).round() as usize;

    (number_of_lanes - 1) - inner_offset
}

/// Returns a 1-based exit rank for a vehicle travelling from `entry_arm` to `exit_arm`.
const fn get_exit_rank(entry_arm: &Arm, exit_arm: &Arm, number_of_arms: usize) -> usize {
    (exit_arm.index() + number_of_arms - entry_arm.index()) % number_of_arms
}
