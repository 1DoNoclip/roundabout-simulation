use crate::*;
use bevy::platform::collections::{HashMap as BevyHashMap, HashSet as BevyHashSet};
use rand::{
    distr::{Distribution, weighted::WeightedIndex},
    rngs::StdRng,
};
use std::collections::VecDeque;

/// Computes the route for a vehicle to follow.
///
/// Assumes that the `lane_index` is always constant throughout the route.
///
/// Returns `Some(Vec<Entity>)`, a vector of `Segment` entities if a route is found.
/// Returns `None` if a route is not found.
pub fn calculate_route(
    segments: &Query<&Segment>,
    arms: &Query<(Entity, &Arm)>,
    lane_index: usize,
    start_arm_index: usize,
    end_arm_index: usize,
) -> Option<Vec<Entity>> {
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
