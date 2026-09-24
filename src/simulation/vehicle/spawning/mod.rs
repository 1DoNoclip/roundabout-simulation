use crate::*;
use rand::{SeedableRng, rng, rngs::StdRng};

/// The spawn timer for a singular lane.
#[derive(Resource)]
pub(crate) enum SpawnTimer {
    /// The countdown timer until the next spawn.
    InterSpawn(Timer),
    /// The current
    DelayedSpawn,
}

impl Default for SpawnTimer {
    fn default() -> Self {
        SpawnTimer::InterSpawn(Timer::from_seconds(0.0, TimerMode::Once))
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
