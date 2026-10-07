use crate::*;
use rand::{SeedableRng, rng, rngs::StdRng};

pub(crate) fn spawn_vehicles(
    mut commands: Commands,
    mut spawner_rng: Local<SpawnerRng>,
    mut entry_lines: Query<(&Segment, &EntryLine, &SpawnTimer)>,
) {
    for (segment, entry_line, spawn_timer) in entry_lines {
        println!("{}", entry_line.total_flow_rate().get::<per_hour>());
    }
}

fn spawn_vehicle(
    In((spawn_point_id, end_arm_id)): In<(Entity, Entity)>,
    mut commands: Commands,
    arms: Query<&Arm>,
    segments: Query<&Segment>,
    spawn_points: Query<&SpawnPoint>,
    end_points: Query<(Entity, &EndPoint)>,
) {
    let spawn_point = spawn_points
        .get(spawn_point_id)
        .expect("expected to get SpawnPoint from entity");
    let end_arm = arms
        .get(end_arm_id)
        .expect("expected to get end Arm from entity");

    let route = calculate_route(&arms, &end_points, &segments, spawn_point, end_arm.index())
        .expect("failed to pathfind from SpawnPoint to EndPoint");

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
