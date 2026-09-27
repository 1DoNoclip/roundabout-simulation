use crate::*;
use uom::ConstZero;

pub(crate) mod components;
pub(crate) mod kinematics;
mod pathfinding;
pub(crate) mod spawning;

pub(crate) use components::*;
pub(crate) use kinematics::*;
use pathfinding::*;
pub(crate) use spawning::*;

pub(super) struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((ComponentsPlugin, KinematicsPlugin, PathfindingPlugin));
    }
}

#[derive(Bundle)]
struct VehicleBundle {
    name: Name,
    vehicle: Vehicle,
    idm_driver: IdmDriver,
    kinematics: Kinematics,
    navigator: Navigator,
    current_speed: Speed,
    current_acceleration: AccelerationComponent,
    next_acceleration: NextAcceleration,
    transform: Transform,
}

impl VehicleBundle {
    fn try_new(
        segments: &Query<&Segment>,
        current_speed: Speed,
        target_speed: Speed,
        max_acceleration: Acceleration,
        max_deceleration: Acceleration,
        route: Vec<Entity>,
    ) -> Result<Self, &'static str> {
        let navigator = Navigator::try_new(route)?;
        let start_segment = segments
            .get(navigator.current_segment_id())
            .expect("expected to find a Segment component");
        let current_acceleration = AccelerationComponent::new(Acceleration::ZERO);
        Ok(VehicleBundle {
            name: Name::new("Vehicle"),
            vehicle: Vehicle,
            idm_driver: IdmDriver::default(),
            kinematics: Kinematics::new(target_speed, max_acceleration, max_deceleration),
            navigator,
            current_speed,
            current_acceleration,
            next_acceleration: NextAcceleration::from(*current_acceleration),
            transform: Transform::from_translation(start_segment.position_at(0.0)),
        })
    }
}

pub(super) fn spawn_vehicles() {}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Generates a fake Bevy entity for testing.
    fn make_test_entity(id: u32) -> Entity {
        Entity::from_raw_u32(id).expect("failed to create Entity from an ID")
    }

    #[test]
    #[should_panic(expected = "Cannot select a destination arm from an empty destination_weights")]
    fn empty_weights_panics() {
        let mut rng = StdRng::seed_from_u64(42);
        let destination_weights = DestinationWeights::default();

        select_destination_arm(&mut rng, &destination_weights);
    }

    #[test]
    fn single_choice_guaranteed() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut destination_weights = DestinationWeights::default();

        let target_arm = make_test_entity(1);
        // Anything above a value of 0 is included in the distribution.
        destination_weights.insert(target_arm, 1);

        // With only one option, it must return that option 100% of the time.
        for _ in 0..100 {
            let selected = select_destination_arm(&mut rng, &destination_weights);
            assert_eq!(selected, target_arm);
        }
    }

    #[test]
    fn zero_weight_ignored() {
        let mut rng = StdRng::seed_from_u64(12345);
        let mut destination_weights = DestinationWeights::default();

        let lucky_arm = make_test_entity(1);
        let unlucky_arm = make_test_entity(2);

        destination_weights.insert(lucky_arm, 10);
        destination_weights.insert(unlucky_arm, 0); // 0% chance of selection.

        // The 0-weight option should never be picked.
        for _ in 0..100 {
            let selected = select_destination_arm(&mut rng, &destination_weights);
            assert_eq!(selected, lucky_arm);
            assert_ne!(selected, unlucky_arm);
        }
    }

    #[test]
    fn statistical_distribution() {
        // Use a fixed seed so the test outcome is completely deterministic.
        // The seed number has no value. Using a fixed seed just ensures that
        // the test never fails due to the seed. The Law of Large Numbers and
        // the tolerance used in the assertions ensures that this test should
        // pass for most seeds.
        let mut rng = StdRng::seed_from_u64(987654321);
        let mut weights = DestinationWeights::default();

        let arm_a = make_test_entity(1);
        let arm_b = make_test_entity(2);

        weights.insert(arm_a, 25); // Should get ~25% of rolls.
        weights.insert(arm_b, 75); // Should get ~75% of rolls.

        let mut count_a = 0;
        let mut count_b = 0;
        let iterations = 10_000;

        for _ in 0..iterations {
            let selected = select_destination_arm(&mut rng, &weights);
            if selected == arm_a {
                count_a += 1;
            } else if selected == arm_b {
                count_b += 1;
            }
        }

        const PERCENTAGE_TOLERANCE: f64 = 2.0;
        // Calculate actual percentages.
        let percentage_a = (count_a as f64 / iterations as f64) * 100.0;
        let percentage_b = (count_b as f64 / iterations as f64) * 100.0;

        // Allow a small statistical tolerance variance (margin of error) of ±2%.
        assert!(
            (percentage_a - 25.0).abs() < PERCENTAGE_TOLERANCE,
            "Arm A variance too high above {PERCENTAGE_TOLERANCE}%: {}%",
            percentage_a
        );
        assert!(
            (percentage_b - 75.0).abs() < PERCENTAGE_TOLERANCE,
            "Arm B variance too high above {PERCENTAGE_TOLERANCE}%: {}%",
            percentage_b
        );
    }
}
