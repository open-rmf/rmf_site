//! In this example, we configure, compute, and animate a simple discrete event
//! simulation for a fleet of robots navigating to randomly assigned goals. Each
//! type of entity is is represented as a Bevy system.
//!
//! ```text
//! ┌───────────────────┐     ┌─────────┐     ┌───────┐
//! │ request_generator │     │ planner │     │ robot │
//! └─────────┬─────────┘     └────┬────┘     └───┬───┘
//!           │                    │              │
//!           │ SubmitRequest      │              │
//!           ├───────────────────►│              │
//!           │                    │              │
//!           │                    │ Trajectory   │
//!           │                    ├─────────────►│
//!           │                    │              │
//!           │                    │              │ Waypoint
//!           │                    │              ├──┐
//!           │                    │              │  │
//!           │                    │              │◄─┘
//!           │                    │              │
//!           │                    │              │ CompleteRequest
//!           │                    │              ├──┐
//!           │                    │              │  │
//!           │                    │              │◄─┘
//!           │                    │              │
//!
//! ```
//!
//! [`request_generator`] creates [`SubmitRequest`] commands assigning robots to
//! goal waypoints at random. [`planner`] generates a [`Trajectory`] from each of
//! these requests. From the generated trajectories, [`robot`] then predicts
//! [`Waypoint`] visits and [`CompleteRequest`] completions.

use bevy::color::palettes::basic;
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;
use rand::Rng;
use rmf_site_sim::event::{CandidateComponentEventWriter, CandidateEventWriter};
use rmf_site_sim::interaction::keyboard::SimulationPlaybackKeyboardPlugin;
use rmf_site_sim::playback::{
    SimulationPlaybackCommand, SimulationPlaybackPlugin, SimulationReplayBehaviour,
};
use rmf_site_sim::time::SimulationClock;
use rmf_site_sim::time::SimulationTime;
use rmf_site_sim::{SimulationBuilder, SimulationPlugin};
use std::time::Duration;

/// Number of robots and goal waypoints to generate.
const NUM_ROBOTS_WAYPOINTS: usize = 5;
/// The duration to stagger the arrival of consecutive requests in seconds.
const REQUEST_ARRIVAL_STAGGER_SECONDS: u64 = 2;
/// The half size of the square simulation area in meters.
const SIMULATION_AREA_HALF_SIZE: f32 = 250.0;
/// Number of steps to interpolate within a trajectory.
const TRAJECTORY_INTERPOLATION_STEPS: usize = 4;
/// The time taken for each robot to travel to
const ROBOT_TRAVEL_DURATION: Duration = Duration::from_secs(10);
/// The speed multiplier used for simulation playback.
const PLAYBACK_SPEED: f32 = 5.0;
/// The duration to pause for before replaying the simulation.
const PAUSE_BEFORE_REPLAY: Duration = Duration::from_secs(1);

/// A pending request for a robot.
#[derive(Component, Clone)]
struct Request {
    goal: Entity,
}

/// Robots and waypoints for which requests have already been submitted.
#[derive(Resource, Clone, Default)]
struct ActiveRequestEntities {
    robots: EntityHashSet,
    waypoints: EntityHashSet,
}

/// A marker component for entities extracted into the simulation world.
#[derive(Component, Clone)]
struct Simulated;

/// A marker component for a robot.
#[derive(Component, Clone, Debug)]
struct Robot;

/// A pose with translation and rotation.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct Pose {
    translation: Vec3,
    rotation: Quat,
}

/// A point on a trajectory.
#[derive(Clone, Debug)]
struct TrajectoryPoint {
    time: SimulationTime,
    pose: Pose,
}

/// A trajectory for a robot to follow.
#[derive(Component, Clone, Debug)]
struct Trajectory {
    points: Vec<TrajectoryPoint>,
}

/// A waypoint, which can be visited or unvisited.
#[derive(Component, Clone, Debug, Eq, PartialEq)]
enum Waypoint {
    Visited,
    Unvisited,
}

/// Assigns a robot to a goal waypoint.
#[derive(Clone, Debug)]
struct SubmitRequest {
    robot: Entity,
    goal: Entity,
}

impl Command for SubmitRequest {
    fn apply(self, world: &mut World) {
        world
            .entity_mut(self.robot)
            .insert(Request { goal: self.goal });
        let mut active_requests = world.resource_mut::<ActiveRequestEntities>();
        active_requests.robots.insert(self.robot);
        active_requests.waypoints.insert(self.goal);
    }
}

/// Places a robot at the end of its trajectory and clears its request.
#[derive(Clone, Debug)]
struct CompleteRequest {
    robot: Entity,
    pose: Pose,
}

impl Command for CompleteRequest {
    fn apply(self, world: &mut World) {
        world
            .entity_mut(self.robot)
            .insert(self.pose)
            .remove::<(Request, Trajectory)>();
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            SimulationPlugin,
            SimulationPlaybackPlugin,
            SimulationPlaybackKeyboardPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

/// Setup the world, builds and spawns a simulation.
fn setup(world: &mut World) {
    let position_bounds =
        Rect::from_center_half_size(Vec2::ZERO, Vec2::splat(SIMULATION_AREA_HALF_SIZE));

    world.spawn((
        Camera2d,
        Transform::from_translation(position_bounds.center().extend(0.0)),
    ));

    spawn_robots_waypoints(world, NUM_ROBOTS_WAYPOINTS, position_bounds);
    world.insert_resource(ActiveRequestEntities::default());

    let simulation = SimulationBuilder::<Simulated>::new()
        // Components and resources extracted into the simulation world,
        // for entities with the marker component.
        .register_component::<Robot>()
        .register_component::<Name>()
        .register_component::<Pose>()
        .register_component::<Waypoint>()
        .register_resource::<ActiveRequestEntities>()
        // Systems representing models used to compute the simulation.
        .add_prediction_systems((request_generator, planner, robot))
        // Systems to visualize this simulation's synchronized state in the main world while it
        // is the active playback simulation.
        .add_visualization_systems(
            (
                animate_robots,
                (draw_robots, draw_waypoints, draw_trajectory),
            )
                .chain(),
        )
        .build(world);

    let simulation_entity = world.spawn(simulation).id();

    // Select the simulation to play back, replay it on a loop, and start playing at
    // `PLAYBACK_SPEED` instead of the default 1.0x speed.
    world.send_event(SimulationPlaybackCommand::SetActiveSimulation(Some(
        simulation_entity,
    )));
    world.send_event(SimulationPlaybackCommand::SetReplayBehaviour(
        SimulationReplayBehaviour(Some(PAUSE_BEFORE_REPLAY)),
    ));
    world.send_event(SimulationPlaybackCommand::SetSpeed(PLAYBACK_SPEED));
    world.send_event(SimulationPlaybackCommand::Play);
}

/// Spawns a specified number of robots and waypoints of random pose within
/// bounds.
fn spawn_robots_waypoints(world: &mut World, n: usize, position_bounds: Rect) {
    let mut rng = rand::thread_rng();

    for i in 0..n {
        world.spawn((
            Simulated,
            Robot,
            Name::new(format!("Robot{i}")),
            random_axis_aligned_pose(&mut rng, position_bounds),
        ));
        world.spawn((
            Simulated,
            Waypoint::Unvisited,
            Name::new(format!("Waypoint{i}")),
            random_axis_aligned_pose(&mut rng, position_bounds),
        ));
    }
}

/// Generates requests assigning robots to waypoints arbitrarily, exactly once
/// per robot and waypoint.
fn request_generator(
    robots: Query<(Entity, &Name), With<Robot>>,
    waypoints: Query<(Entity, &Name), With<Waypoint>>,
    active_request_entities: Res<ActiveRequestEntities>,
    mut changes: CandidateEventWriter,
) {
    let idle_waypoints = waypoints
        .iter()
        .filter(|(waypoint, _)| !active_request_entities.waypoints.contains(waypoint));
    let idle_robots = robots
        .iter()
        .filter(|(robot, _)| !active_request_entities.robots.contains(robot));

    for (i, ((robot, robot_name), (goal, waypoint_name))) in
        idle_robots.zip(idle_waypoints).enumerate()
    {
        let schedule_number = (active_request_entities.robots.len() + i) as u64;
        let time = SimulationTime::new(Duration::from_secs(
            REQUEST_ARRIVAL_STAGGER_SECONDS * schedule_number,
        ));
        changes.predict(time, SubmitRequest { robot, goal });
        info!(
            "[request_generator] Scheduled request for {} to {} at {:?}",
            robot_name, waypoint_name, time
        );
    }
}

/// Plans trajectories for robots using cubic interpolation.
fn planner(
    robots: Query<(Entity, &Pose, &Name, &Request), (With<Robot>, Without<Trajectory>)>,
    goals: Query<&Pose, With<Waypoint>>,
    clock: Res<SimulationClock>,
    mut trajectories: CandidateComponentEventWriter<Trajectory>,
) {
    let time_now = clock.now();
    for (robot, start_pose, robot_name, request) in robots.iter() {
        let target_pose = goals.get(request.goal).unwrap();
        let trajectory = plan_trajectory(start_pose, target_pose, time_now);
        trajectories.predict_now(robot, trajectory);
        info!(
            "[planner] Planned trajectory for {} at {:?}",
            robot_name, time_now
        );
    }
}

fn plan_trajectory(
    start_pose: &Pose,
    target_pose: &Pose,
    start_time: SimulationTime,
) -> Trajectory {
    let tangent_scale = start_pose.translation.distance(target_pose.translation);
    let curve = CubicHermite::new(
        [start_pose.translation, target_pose.translation],
        [
            start_pose.rotation * Vec3::X * tangent_scale,
            target_pose.rotation * Vec3::X * tangent_scale,
        ],
    )
    .to_curve()
    .unwrap();

    let points = (0..=TRAJECTORY_INTERPOLATION_STEPS)
        .map(|current_step| {
            let progress = current_step as f32 / TRAJECTORY_INTERPOLATION_STEPS as f32;
            let velocity = curve.velocity(progress);
            TrajectoryPoint {
                time: SimulationTime::new(
                    start_time.elapsed() + ROBOT_TRAVEL_DURATION.mul_f32(progress),
                ),
                pose: Pose {
                    translation: curve.position(progress),
                    rotation: Quat::from_rotation_z(velocity.y.atan2(velocity.x)),
                },
            }
        })
        .collect();

    Trajectory { points }
}

/// A robot controller that marks the goal waypoint as visited when a robot's
/// trajectory ends, then completes the request by placing the robot at its
/// goal.
fn robot(
    robots: Query<(Entity, &Request, &Trajectory, &Name), With<Robot>>,
    target_waypoints: Query<&Waypoint>,
    mut waypoints: CandidateComponentEventWriter<Waypoint>,
    mut changes: CandidateEventWriter,
    clock: Res<SimulationClock>,
) {
    let now = clock.now();

    for (robot, request, trajectory, robot_name) in robots.iter() {
        let end = trajectory.points.last().unwrap();
        if end.time < now {
            continue;
        }

        match *target_waypoints.get(request.goal).unwrap() {
            Waypoint::Unvisited => {
                waypoints.predict(end.time, request.goal, Waypoint::Visited);
                info!(
                    "[robot] Scheduled waypoint visit for {} at {:?}",
                    robot_name, end.time
                );
            }
            _ => {
                changes.predict(
                    end.time,
                    CompleteRequest {
                        robot,
                        pose: end.pose,
                    },
                );
                info!(
                    "[robot] Scheduled request completion for {} at {:?}",
                    robot_name, end.time
                );
            }
        }
    }
}

/// Interpolates robot poses along their trajectories at the current playback
/// time, which the [`SimulationClock`] tracks while this simulation is being
/// played back.
fn animate_robots(
    clock: Res<SimulationClock>,
    mut robots: Query<(&mut Pose, &Trajectory), With<Robot>>,
) {
    let time = clock.now();

    for (mut pose, trajectory) in &mut robots {
        if let Some(interpolated) = get_pose_at(trajectory, time) {
            *pose = interpolated;
        }
    }
}

fn get_pose_at(trajectory: &Trajectory, time: SimulationTime) -> Option<Pose> {
    let first = trajectory.points.first()?;
    let last = trajectory.points.last()?;

    if time <= first.time {
        return Some(first.pose);
    }
    if time >= last.time {
        return Some(last.pose);
    }

    let next_idx = trajectory
        .points
        .partition_point(|point| point.time <= time);
    let prev_point = &trajectory.points[next_idx - 1];
    let next_point = &trajectory.points[next_idx];
    let point_delta = next_point.time.elapsed() - prev_point.time.elapsed();
    let progress = if point_delta.is_zero() {
        0.0
    } else {
        (time.elapsed() - prev_point.time.elapsed()).as_secs_f32() / point_delta.as_secs_f32()
    };

    Some(Pose {
        translation: prev_point
            .pose
            .translation
            .lerp(next_point.pose.translation, progress),
        rotation: prev_point
            .pose
            .rotation
            .slerp(next_point.pose.rotation, progress),
    })
}

/// Draws robots using a circle and arrow with gizmos.
fn draw_robots(robots: Query<&Pose, With<Robot>>, mut gizmos: Gizmos) {
    let robot_radius = 40.0;

    for pose in &robots {
        let position = pose.translation.truncate();
        gizmos.circle_2d(position, robot_radius / 2.0, Color::default());
        draw_direction_arrow(&mut gizmos, pose, robot_radius, Color::default());
    }
}

/// Draws waypoints using a circle and arrow with gizmos.
fn draw_waypoints(waypoints: Query<(&Pose, &Waypoint)>, mut gizmos: Gizmos) {
    let waypoint_size = 50.0;

    for (pose, waypoint) in &waypoints {
        let color = match waypoint {
            Waypoint::Visited => Color::from(basic::GREEN),
            Waypoint::Unvisited => Color::from(basic::GRAY),
        };
        gizmos.circle_2d(pose.translation.truncate(), waypoint_size / 2.0, color);
        draw_direction_arrow(&mut gizmos, pose, waypoint_size, color);
    }
}

/// Draws the trajectories of robots with gizmos.
fn draw_trajectory(trajectories: Query<&Trajectory, With<Robot>>, mut gizmos: Gizmos) {
    for trajectory in &trajectories {
        gizmos.linestrip_2d(
            trajectory
                .points
                .iter()
                .map(|point| point.pose.translation.truncate()),
            Color::from(basic::BLUE).with_alpha(0.2),
        );
        for point in &trajectory.points {
            draw_direction_arrow(&mut gizmos, &point.pose, 10.0, Color::from(basic::BLUE));
        }
    }
}

/// Draws a direction arrow with gizmos.
fn draw_direction_arrow(gizmos: &mut Gizmos, pose: &Pose, length: f32, color: Color) {
    let position = pose.translation.truncate();
    let direction = (pose.rotation * Vec3::X).truncate();
    gizmos.arrow_2d(position, position + direction * length, color);
}

/// Generates a random axis-aligned pose within the given position bounds,
fn random_axis_aligned_pose(rng: &mut impl Rng, position_bounds: Rect) -> Pose {
    Pose {
        translation: Vec3::new(
            rng.gen_range(position_bounds.min.x..=position_bounds.max.x),
            rng.gen_range(position_bounds.min.y..=position_bounds.max.y),
            0.0,
        ),
        rotation: Quat::from_rotation_z(rng.gen_range(0..4) as f32 * std::f32::consts::FRAC_PI_2),
    }
}
