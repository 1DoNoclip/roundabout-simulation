use crate::*;
use bevy::platform::collections::{HashMap as BevyHashMap, HashSet as BevyHashSet};
use rand::{
    distr::{Distribution, weighted::WeightedIndex},
    rngs::StdRng,
};
use std::collections::VecDeque;

/// Computes the route for a vehicle to follow.
///
/// Uses Breadth-First Search (BFS).
pub fn calculate_route(
    start_segment_id: Entity,
    target_end_point_id: Entity,
    segments: &Query<&Segment>,
) -> Option<Vec<Entity>> {
    // Edge case.
    if start_segment_id == target_end_point_id {
        return Some(Vec::new());
    }

    // Check if the start segment immediately connects to the target end point.
    if let Ok(start_segment) = segments.get(start_segment_id) {
        if let Connection::EndPoint { end_point_id: end_point } = &start_segment.connection {
            if *end_point == target_end_point_id {
                return Some(vec![start_segment_id]);
            }
        }
    }

    // Queue used in BFS traversal: stores the current segment entity being explored.
    let mut queue = VecDeque::new();
    queue.push_back(start_segment_id);
    // Stores visited segments to prevent infinite looping around.
    let mut visited = BevyHashSet::new();
    visited.insert(start_segment_id);
    // Maps a child segment to its parent segment to reconstruct the path later.
    let mut came_from: BevyHashMap<Entity, Entity> = BevyHashMap::new();
    // Stores the final segment that connects to the end point.
    let mut final_segment: Option<Entity> = None;

    // BFS search loop.
    'search: while let Some(current_segment_id) = queue.pop_front() {
        if let Ok(segment) = segments.get(current_segment_id) {
            match &segment.connection {
                Connection::NextSegments { next_segment_ids, .. } => {
                    for next_segment_id in next_segment_ids {
                        if !visited.contains(next_segment_id) {
                            visited.insert(*next_segment_id);
                            came_from.insert(*next_segment_id, current_segment_id);

                            if let Ok(next_segment) = segments.get(*next_segment_id) {
                                if let Connection::EndPoint { end_point_id } = next_segment.connection {
                                    if end_point_id == target_end_point_id {
                                        final_segment = Some(*next_segment_id);
                                        break 'search;
                                    }
                                }
                            }

                            queue.push_back(*next_segment_id);
                        }
                    }
                }
                Connection::EndPoint { end_point_id } => {
                    if *end_point_id == target_end_point_id {
                        final_segment = Some(current_segment_id);
                        break 'search;
                    }
                }
            }
        }
    }

    // Reconstruct path from end back to start.
    if let Some(end_segment) = final_segment {
        let mut current_segment = end_segment;
        let mut path = vec![current_segment];
        while let Some(&parent) = came_from.get(&current_segment) {
            path.push(parent);
            current_segment = parent;
        }

        path.reverse();
        Some(path)
    } else {
        None
    }
}

pub fn select_destination_arm(
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
