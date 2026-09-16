use crate::*;
use ::mapf::motion::{Duration as MapfDuration, Motion, TimePoint};
use ::mapf::negotiation::{Agent, Scenario as MapfScenario, negotiate};

pub use ::mapf::negotiation::LinearTrajectorySE2;

/// The maximum size that the search queue size can reach in mapf.
const NEGOTIATION_QUEUE_LENGTH_LIMIT: usize = 1_000_000;

/// Negotiate trajectories for agents.
pub fn negotiate_trajectories(
    agents: HashMap<Entity, Agent>,
    names: &Query<&NameInSite>,
    occupancy: &Occupancy,
    start_time: SimulationTime,
) -> Option<Vec<(Entity, LinearTrajectorySE2)>> {
    let mut robots = HashMap::new();
    for robot in agents.keys() {
        let name = name_of(*robot, names);
        if let Some(duplicate) = robots.insert(name.clone(), *robot) {
            panic!("Two robots share the name {name}: {duplicate} and {robot}");
        }
    }

    let scenario = MapfScenario {
        agents: robots
            .iter()
            .map(|(name, robot)| (name.clone(), agents[robot]))
            .collect(),
        obstacles: Vec::new(),
        occupancy: occupancy_map(occupancy),
        cell_size: f64::from(occupancy.cell_size),
        camera_bounds: None,
    };

    let (solution, _, name_map) = negotiate(&scenario, Some(NEGOTIATION_QUEUE_LENGTH_LIMIT))
        .inspect_err(|err| error!("[planner] Negotiation failed: {err}"))
        .ok()?;

    Some(
        solution
            .proposals
            .iter()
            .filter_map(|(id, proposal)| {
                let robot = name_map
                    .get(id)
                    .and_then(|name| robots.get(name))
                    .copied()?;
                let mut trajectory = proposal.meta.trajectory.clone();
                trajectory.adjust_times(MapfDuration::from_std_duration(start_time.elapsed()));
                Some((
                    robot,
                    trajectory
                        .with_indefinite_initial_time(true)
                        .with_indefinite_finish_time(true),
                ))
            })
            .collect(),
    )
}

/// Create a new agent.
pub fn agent(
    pose: &Pose,
    goal: Vec2,
    drive: &DifferentialDrive,
    collision: &CircleCollision,
    cell_size: f32,
) -> Agent {
    let cell = |point: Vec2| Cell::from_point(point, cell_size).to_xy();
    Agent {
        start: cell(Vec2::new(pose.trans[0], pose.trans[1])),
        yaw: f64::from(pose.rot.yaw().radians()),
        goal: cell(goal),
        radius: f64::from(collision.radius),
        speed: f64::from(drive.translational_speed),
        spin: f64::from(drive.rotational_speed),
    }
}

/// The pose at `time` along `trajectory`.
pub fn pose_at(trajectory: &LinearTrajectorySE2, time: SimulationTime, z: f32) -> Pose {
    let position = trajectory
        .motion()
        .compute_position(&time_point(time))
        .unwrap_or_else(|_| trajectory.initial_motion().position);

    Pose {
        trans: [
            position.translation.x as f32,
            position.translation.y as f32,
            z,
        ],
        rot: Rotation::Yaw(Angle::Rad(position.rotation.angle() as f32)),
    }
}

/// The finish time of `trajectory`.
pub fn finish_time(trajectory: &LinearTrajectorySE2) -> SimulationTime {
    SimulationTime::new(
        trajectory
            .finish_motion_time()
            .duration_from_zero()
            .into_std_duration(),
    )
}

/// Shift a trajectory forward in time by `postponement`.
pub fn postponed(trajectory: &LinearTrajectorySE2, postponement: Duration) -> LinearTrajectorySE2 {
    let mut postponed = trajectory.clone();
    postponed.adjust_times(MapfDuration::from_std_duration(postponement));
    postponed
}

/// Intersection intervals in which a trajectory is within `distance` of `cells`.
pub fn intersection_intervals(
    trajectory: &LinearTrajectorySE2,
    cells: &HashSet<Cell>,
    cell_size: f32,
    distance: f32,
) -> Option<(SimulationTime, SimulationTime)> {
    let waypoints = waypoints(trajectory);
    let near = |index: &usize| {
        let point = waypoints[*index].0;
        cells
            .iter()
            .any(|cell| cell.to_center_point(cell_size).distance(point) <= distance)
    };

    let approach = waypoints[(0..waypoints.len()).find(near)?.saturating_sub(1)].1;
    let clear = waypoints[((0..waypoints.len()).rfind(near)? + 1).min(waypoints.len() - 1)].1;
    Some((approach, clear))
}

pub fn waypoint_positions(trajectory: &LinearTrajectorySE2) -> Vec<Vec2> {
    waypoints(trajectory)
        .into_iter()
        .map(|(position, _)| position)
        .collect()
}

/// The position and time of each waypoint of a trajectory.
fn waypoints(trajectory: &LinearTrajectorySE2) -> Vec<(Vec2, SimulationTime)> {
    trajectory
        .iter()
        .map(|waypoint| {
            (
                Vec2::new(
                    waypoint.position.translation.x as f32,
                    waypoint.position.translation.y as f32,
                ),
                SimulationTime::new(waypoint.time.duration_from_zero().into_std_duration()),
            )
        })
        .collect()
}

fn occupancy_map(occupancy: &Occupancy) -> HashMap<i64, Vec<i64>> {
    let mut map = HashMap::<i64, Vec<i64>>::new();
    for cell in occupancy.cells.iter() {
        map.entry(cell.y).or_default().push(cell.x);
    }
    for column in map.values_mut() {
        column.sort();
    }
    map
}

fn time_point(time: SimulationTime) -> TimePoint {
    TimePoint::zero() + MapfDuration::from_std_duration(time.elapsed())
}
