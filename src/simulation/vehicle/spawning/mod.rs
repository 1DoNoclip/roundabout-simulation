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
            let overshoot_time = spawn_timer
                .0
                .elapsed()
                .saturating_sub(spawn_timer.0.duration());

            let u = spawner_rng.0.random::<f32>().max(f32::EPSILON);
            let next_spawn_time = -f32::ln(u) / entry_line.total_flow_rate().get::<per_second>();
            spawn_timer.reset_and_set(Duration::from_secs_f32(next_spawn_time), overshoot_time);
        }
    }
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
        SpawnTimer(Timer::from_seconds(0.0, TimerMode::Once))
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
