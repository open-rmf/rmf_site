use crate::mapf::LinearTrajectorySE2;
use crate::*;

/// The distance at which a robot will hold at if it is blocked by an obstacle.
const ROBOT_OBSTACLE_HOLD_DISTANCE: f32 = 1.0;
/// The duration taken by a door to move between fully closed and fully opened states.
const DOOR_TRANSITION_DURATION: Duration = Duration::from_secs(2);

/// A marker component for entities extracted into the simulation world.
#[derive(Component, Clone, Copy)]
pub struct SimulationMarker;

/// The state of a task.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Active,
    Complete,
}

/// A robot and its positional goal for a task.
#[derive(Component, Clone, Copy, Debug)]
pub struct RobotGoalAssignment {
    pub robot: Entity,
    pub goal: Vec2,
}

/// A trajectory for a robot, in simulation time.
#[derive(Component, Clone, Debug)]
pub struct RobotTrajectory {
    pub task: Entity,
    pub waypoints: LinearTrajectorySE2,
    /// The time at which a robot had stopped along its trajectory, if any.
    pub on_hold_at: Option<SimulationTime>,
}

impl RobotTrajectory {
    pub fn new(task: Entity, waypoints: LinearTrajectorySE2) -> Self {
        Self {
            task,
            waypoints,
            on_hold_at: None,
        }
    }

    /// Create a version of this trajectory that is on hold at the given time.
    pub fn hold(&self, at: SimulationTime) -> Self {
        Self {
            on_hold_at: Some(at),
            ..self.clone()
        }
    }

    /// Create a version of this trajectory that is resumed at the given time.
    pub fn resume(&self, now: SimulationTime) -> Self {
        let waited = now.elapsed() - self.on_hold_at.unwrap_or(now).elapsed();
        Self {
            task: self.task,
            waypoints: crate::mapf::postponed(&self.waypoints, waited),
            on_hold_at: None,
        }
    }

    pub fn time_at(&self, now: SimulationTime) -> SimulationTime {
        self.on_hold_at.unwrap_or(now)
    }

    pub fn pose_at(&self, now: SimulationTime, z: f32) -> Pose {
        crate::mapf::pose_at(&self.waypoints, self.time_at(now), z)
    }
}

/// A convenience command for assigning trajectories to all robots in a single [`rmf_site_sim::event::DiscreteEvent`].
#[derive(Clone, Debug)]
pub struct AssignRobotTrajectory {
    pub trajectories: Vec<(Entity, RobotTrajectory)>,
}

impl Command for AssignRobotTrajectory {
    fn apply(self, world: &mut World) {
        for (robot, trajectory) in self.trajectories {
            world.entity_mut(robot).insert(trajectory);
        }
    }
}

/// The state that a door is requested to move into.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoorCommand {
    Open,
    Close,
}

/// The current state of a door.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum DoorState {
    Closed,
    Opening { since: SimulationTime },
    Open,
    Closing { since: SimulationTime },
}

impl DoorState {
    pub fn is_open(&self) -> bool {
        matches!(self, DoorState::Open)
    }

    /// The position of the door between being fully closed (0.0) and fully open (1.0).
    pub fn position(&self, now: SimulationTime) -> f32 {
        let progress = |since: SimulationTime| {
            let elapsed = now.elapsed().saturating_sub(since.elapsed());
            (elapsed.as_secs_f32() / DOOR_TRANSITION_DURATION.as_secs_f32()).clamp(0.0, 1.0)
        };

        match self {
            DoorState::Closed => 0.0,
            DoorState::Opening { since } => progress(*since),
            DoorState::Open => 1.0,
            DoorState::Closing { since } => 1.0 - progress(*since),
        }
    }
}

/// The cells occupied by a dynamic obstacle across its entire range of motion.
/// The door is the only dynamic obstacle in this simulation.
#[derive(Component, Clone, Debug)]
pub struct DynamicObstacleOccupancy(pub HashSet<Cell>);

/// An occupancy grid for the current level accounting for all non-simulation entities.
#[derive(Resource, Clone, Debug)]
pub struct Occupancy {
    pub cells: HashSet<Cell>,
    pub cell_size: f32,
}

/// Spawns a simulation from the current scenario, that is asynchronously computed.
pub fn spawn_simulation(world: &mut World, tasks: &[Entity], name: String) -> Entity {
    let mut setup_state: SystemState<SimulationSetup> = SystemState::new(world);
    let setup = setup_state.get(world);

    let simulated_entities = setup.simulated_entities();
    let robot_goal_assignments: Vec<_> = tasks
        .iter()
        .filter_map(|task| Some((*task, setup.create_assignment(*task)?)))
        .collect();
    let occupancy = setup.create_occupancy();
    let door_occupancy = setup.create_door_occupancies(occupancy.cell_size);
    world.insert_resource(occupancy);

    for entity in simulated_entities {
        world.entity_mut(entity).insert(SimulationMarker);
    }
    for task in tasks {
        world
            .entity_mut(*task)
            .insert((SimulationMarker, TaskState::Pending));
    }
    for (task, assignment) in robot_goal_assignments {
        world.entity_mut(task).insert(assignment);
    }
    for (door, occupancy) in door_occupancy {
        world.entity_mut(door).insert((
            occupancy,
            // default to closed
            DoorState::Closed,
            DoorCommand::Close,
        ));
    }

    let simulation = SimulationBuilder::<SimulationMarker>::new()
        .register_component::<NameInSite>()
        .register_component::<TaskParams>()
        .register_component::<TaskState>()
        .register_component::<RobotGoalAssignment>()
        .register_component::<Pose>()
        .register_component::<RobotTrajectory>()
        .register_component::<Affiliation<Entity>>()
        .register_component::<DifferentialDrive>()
        .register_component::<CircleCollision>()
        .register_component::<DoorCommand>()
        .register_component::<DoorState>()
        .register_component::<DynamicObstacleOccupancy>()
        .register_component::<DoorType>()
        .register_resource::<Occupancy>()
        .add_prediction_systems((request_generator, robot, door, planner).chain())
        .add_visualization_systems((animate_robots, animate_doors, draw_robot_paths).chain())
        .build(world);

    world.spawn((simulation, NameInSite(name))).id()
}

/// Predicts the activation of tasks at their request time.
pub fn request_generator(
    tasks: Query<(
        Entity,
        &TaskState,
        &RobotGoalAssignment,
        Option<&TaskParams>,
    )>,
    names: Query<&NameInSite>,
    mut states: CandidateComponentEventWriter<TaskState>,
) {
    for (task, state, assignment, params) in tasks.iter() {
        if *state != TaskState::Pending {
            continue;
        }

        let request_time_millis = params
            .and_then(|params| params.request_time())
            .unwrap_or(0)
            .max(0) as u64;
        let time = SimulationTime::new(Duration::from_millis(request_time_millis));
        info!(
            "[request_generator] Predicted the request for {} at {time:?}",
            name_of(assignment.robot, &names)
        );
        states.predict(time, task, TaskState::Active);
    }
}

/// Predicts robot behaviour, including its pose updates, task completion, and issued door commands.
pub fn robot(
    tasks: Query<(Entity, &TaskState, &RobotGoalAssignment)>,
    trajectories: Query<&RobotTrajectory>,
    poses_now: Query<&Pose>,
    doors: Query<(Entity, &DoorState, &DoorCommand, &DynamicObstacleOccupancy)>,
    names: Query<&NameInSite>,
    occupancy: Res<Occupancy>,
    clock: Res<SimulationClock>,
    mut states: CandidateComponentEventWriter<TaskState>,
    mut poses: CandidateComponentEventWriter<Pose>,
    mut paths: CandidateComponentEventWriter<RobotTrajectory>,
    mut commands: CandidateComponentEventWriter<DoorCommand>,
) {
    let now = clock.now();
    let mut doors_to_open = HashSet::new();

    for (task, state, assignment) in tasks.iter() {
        let Some(trajectory) = active_trajectory(task, state, assignment, &trajectories) else {
            continue;
        };
        let Ok(pose) = poses_now.get(assignment.robot) else {
            continue;
        };

        // The current time of a robot along its trajectory.
        // If a robot went on hold for an obstacle, this is the time at which it stopped, otherwise it is the current simulation time.
        let trajectory_time = trajectory.time_at(now);

        // An instantaneous update to the robot's position along its trajectory, if not already in position.
        let expected = trajectory.pose_at(now, pose.trans[2]);
        if *pose != expected {
            info!(
                "[robot] Predicted {} to move into position at {now:?}",
                name_of(assignment.robot, &names)
            );
            poses.predict_now(assignment.robot, expected);
        }

        let door_intersections =
            door_intersections(trajectory, trajectory_time, &doors, occupancy.cell_size);
        doors_to_open.extend(
            door_intersections
                .iter()
                .filter(|intersection| intersection.reached(trajectory_time))
                .map(|intersection| intersection.entity),
        );

        let is_trajectory_on_hold = trajectory.on_hold_at.is_some();
        let is_blocked = door_intersections
            .iter()
            .any(|intersection| !intersection.open && intersection.reached(trajectory_time));
        match (is_trajectory_on_hold, is_blocked) {
            (false, true) => {
                info!(
                    "[robot] Predicted {} to wait for a door at {now:?}",
                    name_of(assignment.robot, &names)
                );
                paths.predict_now(assignment.robot, trajectory.hold(trajectory_time));
            }
            (true, false) => {
                info!(
                    "[robot] Predicted {} to resume at {now:?}",
                    name_of(assignment.robot, &names)
                );
                paths.predict_now(assignment.robot, trajectory.resume(now));
            }
            (false, false) => {
                let arrival = crate::mapf::finish_time(&trajectory.waypoints);
                if let Some(next_pose_update) = door_intersections
                    .iter()
                    .flat_map(|intersection| [intersection.begin, intersection.end])
                    .chain([arrival])
                    .filter(|time| *time > trajectory_time)
                    .min()
                {
                    let moved = crate::mapf::pose_at(
                        &trajectory.waypoints,
                        next_pose_update,
                        pose.trans[2],
                    );
                    info!(
                        "[robot] Predicted {} to move into position at {next_pose_update:?}",
                        name_of(assignment.robot, &names)
                    );
                    poses.predict(next_pose_update, assignment.robot, moved);
                }
                info!(
                    "[robot] Predicted {} to complete its task at {arrival:?}",
                    name_of(assignment.robot, &names)
                );
                states.predict(arrival, task, TaskState::Complete);
            }
            _ => {}
        }
    }

    command_doors(&doors, &doors_to_open, now, &names, &mut commands);
}

/// The interval over which a robot's trajectory comes within
/// [`ROBOT_OBSTACLE_HOLD_DISTANCE`] of an obstacle.
struct ObstacleIntersection {
    entity: Entity,
    open: bool,
    begin: SimulationTime,
    end: SimulationTime,
}

impl ObstacleIntersection {
    fn reached(&self, time: SimulationTime) -> bool {
        self.begin <= time
    }
}

fn door_intersections(
    trajectory: &RobotTrajectory,
    at: SimulationTime,
    doors: &Query<(Entity, &DoorState, &DoorCommand, &DynamicObstacleOccupancy)>,
    cell_size: f32,
) -> Vec<ObstacleIntersection> {
    doors
        .iter()
        .filter_map(|(entity, state, _, cells)| {
            let (begin, end) = crate::mapf::intersection_intervals(
                &trajectory.waypoints,
                &cells.0,
                cell_size,
                ROBOT_OBSTACLE_HOLD_DISTANCE,
            )?;
            (end > at).then_some(ObstacleIntersection {
                entity,
                open: state.is_open(),
                begin,
                end,
            })
        })
        .collect()
}

fn command_doors(
    doors: &Query<(Entity, &DoorState, &DoorCommand, &DynamicObstacleOccupancy)>,
    open: &HashSet<Entity>,
    now: SimulationTime,
    names: &Query<&NameInSite>,
    commands: &mut CandidateComponentEventWriter<DoorCommand>,
) {
    for (door, _, commanded, _) in doors.iter() {
        let command = if open.contains(&door) {
            DoorCommand::Open
        } else {
            DoorCommand::Close
        };
        if *commanded != command {
            info!(
                "[robot] Predicted {} to be commanded to {command:?} at {now:?}",
                name_of(door, names)
            );
            commands.predict_now(door, command);
        }
    }
}

/// Predicts the state changes of a door in response to its [`DoorCommand`] request.
pub fn door(
    doors: Query<(Entity, &DoorState, &DoorCommand)>,
    names: Query<&NameInSite>,
    clock: Res<SimulationClock>,
    mut states: CandidateComponentEventWriter<DoorState>,
) {
    let now = clock.now();

    for (door, state, command) in doors.iter() {
        let (time, next) = match (state, command) {
            // A door in motion always comes to rest before it reverses.
            (DoorState::Opening { since }, _) => {
                (*since + DOOR_TRANSITION_DURATION, DoorState::Open)
            }
            (DoorState::Closing { since }, _) => {
                (*since + DOOR_TRANSITION_DURATION, DoorState::Closed)
            }
            (DoorState::Closed, DoorCommand::Open) => (now, DoorState::Opening { since: now }),
            (DoorState::Open, DoorCommand::Close) => (now, DoorState::Closing { since: now }),
            _ => continue,
        };

        info!(
            "[door] Predicted {} to be {next:?} at {time:?}",
            name_of(door, &names)
        );
        states.predict(time, door, next);
    }
}

/// Predicts the instantaneous generation of trajectories for robots.
pub fn planner(
    tasks: Query<(Entity, &TaskState, &RobotGoalAssignment)>,
    robots: Query<(&Pose, &Affiliation<Entity>, Option<&RobotTrajectory>)>,
    descriptions: Query<(&DifferentialDrive, &CircleCollision)>,
    names: Query<&NameInSite>,
    occupancy: Res<Occupancy>,
    clock: Res<SimulationClock>,
    mut changes: CandidateEventWriter,
) {
    let mut agents = HashMap::new();
    let mut assigned_tasks = HashMap::new();
    let mut trajectory_required = false;

    for (task, state, assignment) in tasks.iter() {
        if *state != TaskState::Active {
            continue;
        }

        let (pose, affiliation, trajectory) = robots
            .get(assignment.robot)
            .expect("An active robot has no pose, affiliation, or trajectory.");
        let (drive, collision) = affiliation
            .0
            .and_then(|description| descriptions.get(description).ok())
            .expect(
                "An agent has insufficient collision or drive information for trajectory planning.",
            );

        trajectory_required |= trajectory.is_none_or(|trajectory| trajectory.task != task);
        agents.insert(
            assignment.robot,
            crate::mapf::agent(pose, assignment.goal, drive, collision, occupancy.cell_size),
        );
        assigned_tasks.insert(assignment.robot, task);
    }

    if !trajectory_required || agents.is_empty() {
        return;
    }

    let now = clock.now();
    let trajectory_count = agents.len();
    let Some(negotiated) = crate::mapf::negotiate_trajectories(agents, &names, &occupancy, now)
    else {
        return;
    };
    let trajectories: Vec<_> = negotiated
        .into_iter()
        .filter_map(|(robot, waypoints)| {
            let task = *assigned_tasks.get(&robot)?;
            Some((robot, RobotTrajectory::new(task, waypoints)))
        })
        .collect();

    info!("[planner] Predicted {trajectory_count} negotiated trajectories at {now:?}");
    changes.predict_now(AssignRobotTrajectory { trajectories });
}

/// A helper [`SystemParam`] for setting up the simulation.
#[derive(SystemParam)]
pub struct SimulationSetup<'w, 's> {
    tasks: Query<'w, 's, (&'static Task<Entity>, &'static GoToPlace<Entity>)>,
    robots: Query<'w, 's, (Entity, Option<&'static Affiliation<Entity>>), With<Robot>>,
    doors: Query<
        'w,
        's,
        (
            Entity,
            &'static Edge<Entity>,
            &'static DoorType,
            &'static Bottom,
            &'static Top,
        ),
        With<DoorMarker>,
    >,
    locations: Query<'w, 's, &'static Point<Entity>, With<LocationTags>>,
    anchors: Query<'w, 's, &'static GlobalTransform>,
    grids: Query<'w, 's, (&'static Grid, &'static ChildOf)>,
    level_height: LevelHeightParam<'w, 's>,
    current_level: Res<'w, CurrentLevel>,
}

impl SimulationSetup<'_, '_> {
    fn simulated_entities(&self) -> Vec<Entity> {
        let mut entities = Vec::new();

        entities.extend(self.robots.iter().map(|(robot, ..)| robot));
        entities.extend(
            self.robots
                .iter()
                .filter_map(|(_, affiliation)| affiliation.and_then(|a| a.0)),
        );
        entities.extend(self.doors.iter().map(|(door, ..)| door));
        entities
    }

    fn create_door_occupancies(&self, cell_size: f32) -> Vec<(Entity, DynamicObstacleOccupancy)> {
        self.doors
            .iter()
            .filter_map(|(door, edge, kind, bottom, top)| {
                let left = self.anchors.get(edge.left()).ok()?.translation().truncate();
                let right = self
                    .anchors
                    .get(edge.right())
                    .ok()?
                    .translation()
                    .truncate();
                let level_height = self.level_height.get_level_height(door);
                let bottom = bottom.for_level_height(level_height);
                let top = top.for_level_height(level_height);
                Some((
                    door,
                    create_door_occupancy(left, right, kind, bottom, top, cell_size),
                ))
            })
            .collect()
    }

    fn create_assignment(&self, task: Entity) -> Option<RobotGoalAssignment> {
        let (request, go_to_place) = self.tasks.get(task).ok()?;
        let (robot, _) = self.robots.get(request.robot().0?).ok()?;
        let Point(anchor) = self.locations.get(go_to_place.location.0?).ok()?;
        let goal = self.anchors.get(*anchor).ok()?.translation().truncate();

        Some(RobotGoalAssignment { robot, goal })
    }

    fn create_occupancy(&self) -> Occupancy {
        let level = self.current_level.0.expect("No current level set");
        let (grid, _) = self
            .grids
            .iter()
            .find(|(_, child_of)| child_of.parent() == level)
            .expect("No occupancy grid found for the current level");

        Occupancy {
            cells: grid.occupied.clone(),
            cell_size: grid.cell_size,
        }
    }
}

/// The occupancy that a door occupies across every position of its motion.
fn create_door_occupancy(
    left: Vec2,
    right: Vec2,
    door: &DoorType,
    bottom: f32,
    top: f32,
    cell_size: f32,
) -> DynamicObstacleOccupancy {
    let steps = ((left.distance(right) * std::f32::consts::PI) / (cell_size / 2.0)).ceil();
    let steps = (steps as usize).max(1);

    let mut cells = HashSet::new();
    let mut door_swept = door.clone();

    for step in 0..=steps {
        door_swept.set_positions(step as f32 / steps as f32);
        for (start, end) in door_panel_endpoints(left, right, &door_swept, bottom, top) {
            cells.extend(get_cells_along(&[start, end], cell_size));
        }
    }

    DynamicObstacleOccupancy(cells)
}

fn door_panel_endpoints(
    left: Vec2,
    right: Vec2,
    kind: &DoorType,
    bottom: f32,
    top: f32,
) -> Vec<(Vec2, Vec2)> {
    let length = left.distance(right);
    let offset = match kind {
        DoorType::DoubleSliding(door) => door.compute_offset(length),
        DoorType::DoubleSwing(door) => door.compute_offset(length),
        _ => 0.0,
    };

    let origin = (left + right) / 2.0;
    let y_axis = (left - right).normalize_or_zero();
    let x_axis = Vec2::new(y_axis.y, -y_axis.x);
    let in_site = |point: Vec2| origin + x_axis * point.x + y_axis * point.y;

    find_door_position_tfs(kind, bottom, top, length, offset)
        .into_iter()
        .map(|panel| {
            let center = panel.translation.truncate();
            let half_span = (panel.rotation * Vec3::Y).truncate() * panel.scale.y / 2.0;
            (in_site(center - half_span), in_site(center + half_span))
        })
        .collect()
}

pub fn get_cells_along(points: &[Vec2], cell_size: f32) -> HashSet<Cell> {
    let mut cells: HashSet<Cell> = points
        .iter()
        .map(|point| Cell::from_point(*point, cell_size))
        .collect();

    for pair in points.windows(2) {
        let steps = (pair[0].distance(pair[1]) / (cell_size / 2.0)).ceil() as usize;
        for step in 1..steps {
            let point = pair[0].lerp(pair[1], step as f32 / steps as f32);
            cells.insert(Cell::from_point(point, cell_size));
        }
    }
    cells
}

pub fn active_trajectory<'a>(
    task: Entity,
    state: &TaskState,
    assignment: &RobotGoalAssignment,
    trajectories: &'a Query<&RobotTrajectory>,
) -> Option<&'a RobotTrajectory> {
    if *state != TaskState::Active {
        return None;
    }
    let trajectory = trajectories.get(assignment.robot).ok()?;
    (trajectory.task == task).then_some(trajectory)
}

pub fn name_of(entity: Entity, names: &Query<&NameInSite>) -> String {
    names
        .get(entity)
        .map(|name| name.0.clone())
        .unwrap_or_else(|_| entity.to_string())
}
