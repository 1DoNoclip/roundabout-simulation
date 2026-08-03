use crate::*;
use rand::{
    distr::{Distribution, weighted::WeightedIndex},
    rngs::StdRng,
};

pub fn calculate_route() {

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
