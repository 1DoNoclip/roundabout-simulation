use crate::*;
use rand::{RngExt, SeedableRng, rng, rngs::StdRng};

pub(crate) fn spawn_vehicles(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner_rng: Local<SpawnerRng>,
    mut entry_lines: Query<(&Segment, &EntryLine, &mut SpawnTimer)>,
) {
    for (segment, entry_line, mut spawn_timer) in entry_lines {
        spawn_timer.0.tick(time.delta());

        if spawn_timer.0.is_finished() {
            set_next_spawn_time(
                &mut spawner_rng,
                &mut spawn_timer,
                entry_line.total_flow_rate(),
            );
        }
    }
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
}
