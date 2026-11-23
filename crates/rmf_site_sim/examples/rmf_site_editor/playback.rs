use crate::*;

/// A marker component for a mesh drawing part of a robot's trajectory.
#[derive(Component)]
pub struct SimulationPathVisual;

pub fn animate_robots(
    tasks: Query<(Entity, &TaskState, &RobotGoalAssignment)>,
    trajectories: Query<&RobotTrajectory>,
    mut poses: Query<&mut Pose>,
    clock: Res<SimulationClock>,
) {
    let now = clock.now();

    for (task, state, assignment) in tasks.iter() {
        let Some(trajectory) = active_trajectory(task, state, assignment, &trajectories) else {
            continue;
        };
        let Ok(mut pose) = poses.get_mut(assignment.robot) else {
            continue;
        };

        *pose = trajectory.pose_at(now, pose.trans[2]);
    }
}

pub fn animate_doors(
    doors: Query<(Entity, &DoorState)>,
    mut kinds: Query<&mut DoorType>,
    clock: Res<SimulationClock>,
) {
    let now = clock.now();

    for (door, state) in doors.iter() {
        let Ok(mut kind) = kinds.get_mut(door) else {
            continue;
        };

        let mut moved = kind.clone();
        moved.set_positions(state.position(now));
        kind.set_if_neq(moved);
    }
}

pub fn draw_robot_paths(
    tasks: Query<(Entity, &TaskState, &RobotGoalAssignment)>,
    trajectories: Query<&RobotTrajectory>,
    collisions: Query<&CircleCollision>,
    affiliations: Query<&Affiliation<Entity>>,
    site_assets: Res<SiteAssets>,
    current_level: Res<CurrentLevel>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut visuals: Query<PathVisualMeshes, With<SimulationPathVisual>>,
    mut pool: Local<Vec<Entity>>,
    mut robot_materials: Local<HashMap<Entity, Handle<StandardMaterial>>>,
    mut commands: Commands,
) {
    let Some(level) = current_level.0 else {
        return;
    };

    let mut meshes = Vec::new();
    for (task, state, assignment) in tasks.iter() {
        let Some(trajectory) = active_trajectory(task, state, assignment, &trajectories) else {
            continue;
        };

        let radius = affiliations
            .get(assignment.robot)
            .ok()
            .and_then(|affiliation| affiliation.0)
            .and_then(|description| collisions.get(description).ok())
            .map(|collision| collision.radius)
            .expect("Robots should have a collision radius");
        let material = robot_materials
            .entry(assignment.robot)
            .or_insert_with(|| materials.add(path_material()))
            .clone();

        let positions = crate::mapf::waypoint_positions(&trajectory.waypoints);
        for pair in positions.windows(2) {
            let start = pair[0].extend(ZLayer::RobotPath.to_z());
            let end = pair[1].extend(ZLayer::RobotPath.to_z());
            let scale = Vec3::new(radius, radius, 1.0);
            meshes.extend([
                (
                    site_assets.robot_path_rectangle_mesh.clone(),
                    line_stroke_transform(&start, &end, radius * 2.0),
                    material.clone(),
                ),
                (
                    site_assets.robot_path_circle_mesh.clone(),
                    Transform::from_translation(start).with_scale(scale),
                    material.clone(),
                ),
                (
                    site_assets.robot_path_circle_mesh.clone(),
                    Transform::from_translation(end).with_scale(scale),
                    material.clone(),
                ),
            ]);
        }
    }

    let drawn = meshes.len();
    for (index, (mesh, transform, material)) in meshes.into_iter().enumerate() {
        let Some(mut visual) = pool.get(index).and_then(|e| visuals.get_mut(*e).ok()) else {
            let entity = commands
                .spawn((
                    SimulationPathVisual,
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    transform,
                    ChildOf(level),
                ))
                .id();
            match pool.get_mut(index) {
                Some(pooled) => *pooled = entity,
                None => pool.push(entity),
            }
            continue;
        };

        *visual.transform = transform;
        *visual.visibility = Visibility::Inherited;
        visual.mesh.0 = mesh;
        visual.material.0 = material;
    }

    for entity in pool.iter().skip(drawn) {
        if let Ok(mut visual) = visuals.get_mut(*entity) {
            *visual.visibility = Visibility::Hidden;
        }
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct PathVisualMeshes {
    transform: &'static mut Transform,
    visibility: &'static mut Visibility,
    mesh: &'static mut Mesh3d,
    material: &'static mut MeshMaterial3d<StandardMaterial>,
}

fn path_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb_from_array(ColorPicker::get_color()),
        unlit: true,
        ..Default::default()
    }
}
