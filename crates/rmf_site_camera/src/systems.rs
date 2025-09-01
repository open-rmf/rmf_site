use crate::*;
use bevy_time::Time;
use bevy_transform::components::GlobalTransform;
use tracing::warn;

/// init cameras for project
pub fn init_cameras(
    camera_control_mesh: Res<PickMarkerMesh>,
    mut ambient_light: ResMut<AmbientLight>,
    mut commands: Commands,
) {
    let selection_mesh = camera_control_mesh.0.clone();
    commands.spawn((
        Mesh3d(selection_mesh),
        Visibility::Visible,
        Transform::default(),
        MeshMaterial3d::<StandardMaterial>::default(),
        CameraPickMarker,
        Name::new("selection_marker"),
    ));

    let perspective_headlight = commands
        .spawn((
            DirectionalLight {
                shadows_enabled: false,
                illuminance: 50.,
                ..default()
            },
            PerspectiveHeadlightTarget,
            Name::new("perspective_headlight"),
        ))
        .insert(main_view_render_layers())
        .id();

    let perspective_child_cameras = [
        (1, SELECTED_OUTLINE_LAYER),
        (2, HOVERED_OUTLINE_LAYER),
        (3, XRAY_RENDER_LAYER),
    ]
    .map(|(order, layer)| {
        commands
            .spawn(Camera3d::default())
            .insert((
                Projection::Perspective(Default::default()),
                Camera {
                    order,
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                Tonemapping::ReinhardLuminance,
                Exposure {
                    ev100: DEFAULT_CAMERA_EV100,
                },
            ))
            .insert(Visibility::Inherited)
            .insert(RenderLayers::layer(layer))
            .id()
    });

    commands
        .spawn(Camera3d::default())
        .insert((
            Transform::from_xyz(-10., -10., 10.).looking_at(Vec3::ZERO, Vec3::Z),
            PerspectiveCameraRoot,
            Projection::Perspective(Default::default()),
            Exposure {
                ev100: DEFAULT_CAMERA_EV100,
            },
            Tonemapping::ReinhardLuminance,
            Name::new("perspective_base_camera"),
        ))
        .insert(Visibility::Inherited)
        .insert(RenderLayers::from_layers(&[
            GENERAL_RENDER_LAYER,
            VISUAL_CUE_RENDER_LAYER,
        ]))
        .add_children(&[perspective_headlight])
        .add_children(&perspective_child_cameras);

    let orthographic_headlight = commands
        .spawn((
            DirectionalLight {
                shadows_enabled: false,
                illuminance: 50.,
                ..default()
            },
            OrthographicHeadlightTarget,
            Transform::from_rotation(Quat::from_axis_angle(
                Vec3::new(1., 1., 0.).normalize(),
                35_f32.to_radians(),
            )),
        ))
        .insert(main_view_render_layers())
        .id();

    let ortho_projection = OrthographicProjection {
        viewport_origin: Vec2::new(0.5, 0.5),
        scaling_mode: ScalingMode::FixedVertical {
            viewport_height: 1.0,
        },
        scale: 10.0,
        ..OrthographicProjection::default_3d()
    };

    let orthographic_child_cameras = [
        (1, SELECTED_OUTLINE_LAYER),
        (2, HOVERED_OUTLINE_LAYER),
        (3, XRAY_RENDER_LAYER),
    ]
    .map(|(order, layer)| {
        commands
            .spawn(Camera3d::default())
            .insert((
                Camera {
                    is_active: false,
                    order,
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                Projection::Orthographic(ortho_projection.clone()),
                Exposure {
                    ev100: DEFAULT_CAMERA_EV100,
                },
                Tonemapping::ReinhardLuminance,
            ))
            .insert(Visibility::Inherited)
            .insert(RenderLayers::layer(layer))
            .id()
    });

    commands
        .spawn(Camera3d::default())
        .insert((
            Camera {
                is_active: false,
                ..default()
            },
            OrthographicCameraRoot,
            Transform::from_xyz(0., 0., 20.).looking_at(Vec3::ZERO, Vec3::Y),
            Projection::Orthographic(ortho_projection),
            Exposure {
                ev100: DEFAULT_CAMERA_EV100,
            },
            Tonemapping::ReinhardLuminance,
            Name::new("orthographic_base_camera"),
        ))
        .insert(Visibility::Inherited)
        .insert(RenderLayers::from_layers(&[
            GENERAL_RENDER_LAYER,
            VISUAL_CUE_RENDER_LAYER,
        ]))
        .add_children(&[orthographic_headlight])
        .add_children(&orthographic_child_cameras);

    ambient_light.brightness = 2.0;
}

pub fn change_projection_mode(
    projection_mode: Res<ProjectionMode>,
    mut cameras: Query<(&mut Camera, &mut Visibility)>,
    ortho_cams: Query<(Entity, &Children), With<OrthographicCameraRoot>>,
    persp_cams: Query<(Entity, &Children), With<PerspectiveCameraRoot>>,
) {
    let projection_mode = *projection_mode;

    match projection_mode {
        ProjectionMode::Perspective => {
            for (ortho_cam, children) in ortho_cams {
                let Ok((mut camera, mut visibility)) = cameras.get_mut(ortho_cam) else {
                    continue;
                };
                camera.is_active = false;
                *visibility = Visibility::Hidden;
                for child in children {
                    let Ok((mut child_camera, _)) = cameras.get_mut(*child) else {
                        continue;
                    };
                    child_camera.is_active = false;
                }
            }
            for (persp_cam, children) in persp_cams {
                let Ok((mut camera, mut visibility)) = cameras.get_mut(persp_cam) else {
                    continue;
                };
                camera.is_active = true;
                *visibility = Visibility::Inherited;
                for child in children {
                    let Ok((mut child_camera, _)) = cameras.get_mut(*child) else {
                        continue;
                    };
                    child_camera.is_active = true;
                }
            }
        }
        ProjectionMode::Orthographic => {
            for (ortho_cam, children) in ortho_cams {
                let Ok((mut camera, mut visibility)) = cameras.get_mut(ortho_cam) else {
                    continue;
                };
                camera.is_active = true;
                *visibility = Visibility::Inherited;
                for child in children {
                    let Ok((mut child_camera, _)) = cameras.get_mut(*child) else {
                        continue;
                    };
                    child_camera.is_active = true;
                }
            }
            for (persp_cam, children) in persp_cams {
                let Ok((mut camera, mut visibility)) = cameras.get_mut(persp_cam) else {
                    continue;
                };
                camera.is_active = false;
                *visibility = Visibility::Hidden;
                for child in children {
                    let Ok((mut child_camera, _)) = cameras.get_mut(*child) else {
                        continue;
                    };
                    child_camera.is_active = false;
                }
            }
        }
    }
}

pub fn toggle_headlights(
    headlight_toggle: Res<HeadlightToggle>,
    projection_mode: Res<ProjectionMode>,
    mut visibility: Query<&mut Visibility>,
    mut perspective_headlights: Query<Entity, With<PerspectiveHeadlightTarget>>,
    mut orthographic_headlights: Query<Entity, With<PerspectiveHeadlightTarget>>,
) {
    let targets = match *projection_mode {
        ProjectionMode::Perspective => perspective_headlights.iter_mut().collect::<Vec<_>>(),
        ProjectionMode::Orthographic => orthographic_headlights.iter_mut().collect::<Vec<_>>(),
    };
    for target in targets {
        if let Ok(mut visibility) = visibility.get_mut(target) {
            if headlight_toggle.0 {
                *visibility = Visibility::Inherited
            } else {
                *visibility = Visibility::Hidden
            }
        }
    }
}

pub fn camera_config(
    mut pan_to: ResMut<PanToElement>,
    mut cursor_command: ResMut<CursorCommand>,
    mut keyboard_command: ResMut<KeyboardCommand>,
    mut cameras: Query<(&mut Projection, &mut Transform)>,
    projection_mode: Res<ProjectionMode>,
    camera_blocked: Res<CameraControlBlocked>,
    perspective_cam_root: Query<(Entity, &Children), With<PerspectiveCameraRoot>>,
    orthographic_cam_root: Query<(Entity, &Children), With<OrthographicCameraRoot>>,
) {
    // don't run camera controls if something else has taken priority over it.
    if camera_blocked.blocked() {
        return;
    }

    let translation_delta: Vec3;
    let rotation_delta: Quat;
    let fov_delta: f32;
    let scale_delta: f32;
    if cursor_command.command_type != CameraCommandType::Inactive {
        translation_delta = cursor_command.take_translation_delta();
        rotation_delta = cursor_command.take_rotation_delta();
        fov_delta = cursor_command.take_fov_delta();
        scale_delta = cursor_command.take_scale_delta();
    } else {
        translation_delta = keyboard_command.take_translation_delta();
        rotation_delta = keyboard_command.take_rotation_delta();
        fov_delta = keyboard_command.take_fov_delta();
        scale_delta = keyboard_command.take_scale_delta();
    }

    // stop camera panning if there exists user input that moves the camera
    if pan_to.interruptible
        && pan_to.target.is_some()
        && (translation_delta != Vec3::ZERO
            || rotation_delta != Quat::IDENTITY
            || fov_delta != 0.
            || scale_delta != 0.)
    {
        pan_to.target = None;
    }

    match *projection_mode {
        ProjectionMode::Perspective => {
            let Ok((e, children)) = perspective_cam_root
                .single()
                .inspect_err(|err| warn!("could not get perspective cam entity due to: {:#}", err))
            else {
                return;
            };

            let new_child_proj = {
                let Ok((mut persp_proj, mut persp_transform)) =
                    cameras.get_mut(e).inspect_err(|err| {
                        warn!("could not get perspective cam components due to {:#}", err)
                    })
                else {
                    return;
                };
                if let Projection::Perspective(persp_proj) = persp_proj.as_mut() {
                    persp_transform.translation += translation_delta;
                    persp_transform.rotation *= rotation_delta;
                    persp_proj.fov += fov_delta;
                    persp_proj.fov = persp_proj
                        .fov
                        .clamp(MIN_FOV.to_radians(), MAX_FOV.to_radians());

                    // Ensure upright
                    let forward = persp_transform.forward();
                    persp_transform.look_to(*forward, Vec3::Z);
                }
                persp_proj.clone()
            };

            let valid_children = children
                .iter()
                .filter(|n| cameras.contains(*n))
                .collect::<Vec<_>>();

            for child in &valid_children {
                if let Ok((mut child_proj, _)) = cameras.get_mut(*child) {
                    *child_proj = new_child_proj.clone();
                }
            }
        }
        ProjectionMode::Orthographic => {
            let Ok((e, children)) = orthographic_cam_root
                .single()
                .inspect_err(|err| warn!("could not get perspective cam entity due to: {:#}", err))
            else {
                return;
            };

            let new_child_proj = {
                let Ok((mut ortho_proj, mut ortho_transform)) =
                    cameras.get_mut(e).inspect_err(|err| {
                        warn!("could not get perspective cam components due to {:#}", err)
                    })
                else {
                    return;
                };

                if let Projection::Orthographic(ortho_proj) = ortho_proj.as_mut() {
                    ortho_transform.translation += translation_delta;
                    ortho_transform.rotation *= rotation_delta;
                    ortho_proj.scale += scale_delta;
                }

                ortho_proj.clone()
            };

            let valid_children = children
                .iter()
                .filter(|n| cameras.contains(*n))
                .collect::<Vec<_>>();

            for child in &valid_children {
                if let Ok((mut child_proj, _)) = cameras.get_mut(*child) {
                    *child_proj = new_child_proj.clone();
                }
            }
        }
    }
}
pub fn update_orbit_center_marker(
    //camera: Single<(&CameraConfig, &ProjectionMode)>,
    controls: Res<CameraConfig>,
    projection_mode: Res<ProjectionMode>,
    keyboard_command: Res<KeyboardCommand>,
    cursor_command: Res<CursorCommand>,
    camera_orbit_mat: Res<OrbitMarkerMaterial>,
    mut gizmo: Gizmos,
    mut marker_query: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
            &CameraPickMarker,
        ),
        Without<Projection>,
    >,
) {
    //let (controls, mode) = *camera_config;
    let Ok((mut marker_transform, mut marker_visibility, mut marker_material, _)) = marker_query
        .single_mut()
        .inspect_err(|err| warn!("could not update orbit marker due to: {:#}", err))
    else {
        return;
    };

    // Orbiting
    if (cursor_command.command_type == CameraCommandType::Orbit
        || keyboard_command.command_type == CameraCommandType::Orbit)
        && *projection_mode == ProjectionMode::Perspective
    {
        if let Some(orbit_center) = controls.orbit_center {
            *marker_visibility = Visibility::Visible;
            *marker_material = MeshMaterial3d(camera_orbit_mat.0.clone());
            marker_transform.translation = orbit_center;
            gizmo.sphere(Isometry3d::new(orbit_center, Quat::IDENTITY), 0.1, LIME);
        }
    // Panning
    } else if cursor_command.command_type == CameraCommandType::Pan {
        if let Some(cursor_selection) = cursor_command.cursor_selection {
            *marker_visibility = Visibility::Visible;
            *marker_material = MeshMaterial3d(camera_orbit_mat.0.clone());
            marker_transform.translation = cursor_selection;
            gizmo.sphere(
                Isometry3d::new(cursor_selection, Quat::IDENTITY),
                0.1,
                WHITE,
            );
        }
    } else {
        *marker_visibility = Visibility::Hidden;
    }
}

pub fn focus_camera_on_target(
    mut pan_to: ResMut<PanToElement>,
    time: Res<Time>,
    active_camera: ActiveCameraQuery,
    global_transforms: Query<&GlobalTransform>,
    mut transforms: Query<&mut Transform>,
    camera_targets: Query<&CameraTarget>,
) {
    let Some(target_entity) = pan_to.target else {
        return;
    };

    let Ok(active_camera_entity) = active_camera_maybe(&active_camera) else {
        return;
    };
    let Ok(mut camera_transform) = transforms.get_mut(active_camera_entity) else {
        return;
    };

    let target_position;

    if let Ok(target) = camera_targets.get(target_entity) {
        target_position = target.point;
    } else if let Ok(global_transform) = global_transforms.get(target_entity) {
        target_position = global_transform.translation();
    } else {
        warn!("cannot find camera pan target");
        return;
    };

    let rotation_speed = 2.0;
    let camera_motion = camera_transform.looking_at(target_position, Vec3::Z);

    let current_direction: Vec3 = camera_transform.forward().into();
    let target_direction: Vec3 = camera_motion.forward().into();
    let rotation_difference = current_direction - target_direction;

    if rotation_difference.length() > 0.05 {
        camera_transform.rotation = camera_transform
            .rotation
            .slerp(camera_motion.rotation, rotation_speed * time.delta_secs());
    } else if !pan_to.persistent {
        pan_to.target = None;
    }
}
