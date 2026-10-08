use crate::*;
use rand::{RngExt, SeedableRng, rng, rngs::StdRng};

pub(crate) fn spawn_vehicles(
    mut commands: Commands,
    time: Res<Time>,
    mut statistics: ResMut<Statistics>,
    mut spawner_rng: Local<SpawnerRng>,
    mut entry_lines: Query<(&Segment, &EntryLine, &mut SpawnTimer)>,
) {
    for (segment, entry_line, mut spawn_timer) in entry_lines {
        spawn_timer.0.tick(time.delta());

        if spawn_timer.0.is_finished() {
            let total_flow_rate = entry_line.total_flow_rate();

            set_next_spawn_time(&mut spawner_rng, &mut spawn_timer, total_flow_rate);
            if let Some(destination_arm_id) = select_destination_arm_id(
                &mut spawner_rng,
                entry_line.flow_rates(),
                total_flow_rate,
            ) {} else {
                warn!("Failed to select destination arm ID.");
            };
        }
    }
}

fn select_destination_arm_id(
    spawner_rng: &mut SpawnerRng,
    flow_rates: &FlowRates,
    total_flow_rate: Frequency,
) -> Option<Entity> {
    // Using a `WeightedIndex` would be better but would require storing it as part of the `FlowRates`.

    let total_rate = total_flow_rate.get::<per_hour>();
    if total_rate <= 0.0 {
        return None;
    }

    // Pick a uniform random float in 0.0..total_flow_rate.
    let mut sample = spawner_rng.0.random_range(0.0..total_rate);

    for (&destination_arm_id, flow_rate) in flow_rates {
        let rate = flow_rate.get::<per_hour>();
        if sample < rate {
            return Some(destination_arm_id);
        }
        sample -= rate;
    }

    // Fallback in case of floating-point rounding precision edge cases.
    warn!("Precision error.");
    flow_rates.keys().next().copied()
}

/// Calculates the next spawn time so that the `total_flow_rate` is maintained.
fn set_next_spawn_time(
    spawner_rng: &mut SpawnerRng,
    spawn_timer: &mut SpawnTimer,
    total_flow_rate: Frequency,
) {
    // The extra time that has elapsed.
    // Will be a very small time, but important to maintain the total flow rate.
    // Ensures that flow rate is frame-rate indepdendent.
    let overshoot_time = spawn_timer
        .0
        .elapsed()
        .saturating_sub(spawn_timer.0.duration());
    // `f32::EPSILON` is the step from 1.0 to the next larger representable f32 number.
    // Clamping `u` to this value prevents ln(0) which would be negative infinity.
    // This would cause a panic if converted to a Duration.
    let u = spawner_rng.0.random::<f32>().max(f32::EPSILON);
    let next_spawn_time = -f32::ln(u) / total_flow_rate;
    spawn_timer.reset_and_set(
        Duration::from_secs_f32(next_spawn_time.get::<second>()),
        overshoot_time,
    );
}

fn spawn_vehicle(mut commands: Commands, segments: Query<&Segment>, route: Vec<Entity>) {
    commands.spawn(
        VehicleBundle::try_new(
            &segments,
            Speed::ZERO,
            Speed::try_new(Velocity::new::<mile_per_hour>(60.0)).expect("failed to create"),
            Acceleration::new::<meter_per_second_squared>(3.5),
            Acceleration::new::<meter_per_second_squared>(-8.0),
            route,
        )
        .expect("failed to spawn VehicleBundle"),
    );
}

/// The spawn timer for a singular lane.
#[derive(Resource)]
pub(crate) struct SpawnTimer(pub Timer);

impl SpawnTimer {
    /// Resets and sets the time to `duration`.
    /// Ticks the timer by `overshoot_time` after resetting.
    fn reset_and_set(&mut self, duration: Duration, overshoot_time: Duration) {
        self.0.set_duration(duration);
        self.0.reset();
        self.0.tick(overshoot_time);
    }
}

impl Default for SpawnTimer {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0.0, TimerMode::Once);
        // Ticks to make `.is_finished()` true.
        timer.tick(Duration::ZERO);
        SpawnTimer(timer)
    }
}

/// Used in spawn_vehicles.
#[derive(Deref, DerefMut)]
pub(crate) struct SpawnerRng(StdRng);

// Local requires Default to initialize the struct.
impl Default for SpawnerRng {
    fn default() -> Self {
        Self(StdRng::from_rng(&mut rng()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_rates_match_mean() {
        const SAMPLE_SIZE: u32 = 10_000_000;

        let mut spawner_rng = SpawnerRng::default();
        let mut spawn_timer = SpawnTimer::default();

        let total_flow_rate = Frequency::new::<per_hour>(1_200.0);
        let expected_mean_seconds = 1. / total_flow_rate.get::<per_second>();

        let mut total_spawn_time_seconds = 0.0;
        for _ in 0..SAMPLE_SIZE {
            set_next_spawn_time(&mut spawner_rng, &mut spawn_timer, total_flow_rate);
            total_spawn_time_seconds += spawn_timer.0.duration().as_secs_f32();
        }

        let sample_mean_seconds = total_spawn_time_seconds / SAMPLE_SIZE as f32;
        // Must be within 2% of the expected mean.
        let tolerance = expected_mean_seconds * 0.02;

        assert!(
            (sample_mean_seconds - expected_mean_seconds).abs() < tolerance,
            "Expected mean near {}, but got {}",
            expected_mean_seconds,
            sample_mean_seconds
        );
    }

    /// Tests for `select_destination_arm_id`.
    mod test_select_destination_arm_id {
        use super::*;

        #[test]
        fn zero_or_negative_flow_rate_returns_none() {
            let mut spawner_rng = SpawnerRng::default();
            let mut flow_rates = FlowRates::default();
            let entity = Entity::from_raw_u32(1).unwrap();
            flow_rates.insert(entity, Frequency::new::<per_hour>(0.0));

            // Test with 0.0 flow rate.
            let result = select_destination_arm_id(
                &mut spawner_rng,
                &flow_rates,
                Frequency::new::<per_hour>(0.0),
            );
            assert_eq!(result, None);

            // Test with negative flow rate.
            let result_neg = select_destination_arm_id(
                &mut spawner_rng,
                &flow_rates,
                Frequency::new::<per_hour>(-100.0),
            );
            assert_eq!(result_neg, None);
        }

        #[test]
        fn single_destination_always_selected() {
            let mut spawner_rng = SpawnerRng::default();
            let mut flow_rates = FlowRates::default();
            let target_entity = Entity::from_raw_u32(1).unwrap();
            let rate = Frequency::new::<per_hour>(500.0);
            flow_rates.insert(target_entity, rate);

            for _ in 0..100 {
                let result = select_destination_arm_id(&mut spawner_rng, &flow_rates, rate);
                assert_eq!(result, Some(target_entity));
            }
        }

        #[test]
        fn weighted_distribution_proportions() {
            let mut spawner_rng = SpawnerRng::default();
            let mut flow_rates = FlowRates::default();

            let arm_a = Entity::from_raw_u32(1).unwrap();
            let arm_b = Entity::from_raw_u32(2).unwrap();

            // 70% to Arm A, 30% to Arm B.
            let rate_a = Frequency::new::<per_hour>(700.0);
            let rate_b = Frequency::new::<per_hour>(300.0);
            let total_rate = Frequency::new::<per_hour>(1000.0);

            flow_rates.insert(arm_a, rate_a);
            flow_rates.insert(arm_b, rate_b);

            const SAMPLE_COUNT: u32 = 10_000;
            let mut count_a = 0;
            let mut count_b = 0;

            for _ in 0..SAMPLE_COUNT {
                let selected = select_destination_arm_id(&mut spawner_rng, &flow_rates, total_rate);
                match selected {
                    Some(id) if id == arm_a => count_a += 1,
                    Some(id) if id == arm_b => count_b += 1,
                    _ => panic!("Unexpected entity selected"),
                }
            }

            let ratio_a = count_a as f32 / SAMPLE_COUNT as f32;
            let ratio_b = count_b as f32 / SAMPLE_COUNT as f32;

            // Verify sampling is within a ±2% margin of error for 10,000 samples.
            assert!(
                (ratio_a - 0.70).abs() < 0.02,
                "Expected ~0.70, got {ratio_a}"
            );
            assert!(
                (ratio_b - 0.30).abs() < 0.02,
                "Expected ~0.30, got {ratio_b}"
            );
        }

        #[test]
        fn fallback_returns_valid_entity() {
            let mut spawner_rng = SpawnerRng::default();
            let mut flow_rates = FlowRates::default();

            let entity_b = Entity::from_raw_u32(2).unwrap();
            let entity_a = Entity::from_raw_u32(1).unwrap();

            flow_rates.insert(entity_a, Frequency::new::<per_hour>(100.0));
            flow_rates.insert(entity_b, Frequency::new::<per_hour>(200.0));

            // Pass total_flow_rate slightly larger than actual sum to force fallback path iteration.
            let inflated_total = Frequency::new::<per_hour>(300.001);

            let result = select_destination_arm_id(&mut spawner_rng, &flow_rates, inflated_total);
            assert!(
                result == Some(entity_a) || result == Some(entity_b),
                "Fallback should return one of the valid destination entities"
            );
        }
    }
}
