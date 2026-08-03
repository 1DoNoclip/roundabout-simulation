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
    start_segment: Entity,
    target_end_point: Entity,
    segments: &Query<&Segment>,
) -> Option<Vec<Entity>> {
    // Edge case.
    if start_segment == target_end_point {
        return Some(Vec::new());
    }

    // Queue used in BFS traversal: stores the current segment entity being explored.
    let mut queue = VecDeque::new();
    queue.push_back(start_segment);
    // Stores visited segments to prevent infinite looping around.
    let mut visited = BevyHashSet::new();
    visited.insert(start_segment);
    // Maps a child segment to its parent segment to reconstruct the path later.
    let mut came_from = BevyHashMap::new();
    // Stores the final segment that connects to the end point.
    let mut final_segment = None;

    // BFS search loop.
    while let Some(current_segment) = queue.pop_front() {
        if let Ok(segment) = segments.get(current_segment) {
            match &segment.connection {
                Connection::NextSegments { next_segments, .. } => {
                    for next_segment in next_segments {
                        if !visited.contains(next_segment) {
                            visited.insert(*next_segment);
                            came_from.insert(next_segment, current_segment);
                            queue.push_back(*next_segment);
                        }
                    }
                }
                Connection::EndPoint { end_point } => {
                    if *end_point == target_end_point {
                        final_segment = Some(current_segment);
                        break;
                    }
                }
            }
        }
    }

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
