use crate::{components::*, resources::*, *};
use tracing::warn;
use std::any::{type_name, TypeId};
use std::ops::Deref;

fn unqualified_type_name<'a, T>() -> Option<&'a str> {
    type_name::<T>().split("::").last()
}

/// checks if a camera blocking [T] is currently enabled, and block camera if it is.
pub fn update_blocker_registry<T: Resource + Deref<Target = bool>>(
    mut blocker_registry: ResMut<CameraBlockerRegistry>,
    camera_blocker: Res<T>
) {
    let blocker = **camera_blocker;

    let resource_type_name = type_name::<T>().split("::").last().unwrap_or("???");
    println!("adding to blocker registry: {:#?} ", resource_type_name.to_string());
    if blocker == true {
        let resource_id = TypeId::of::<T>();
        let (_, blocked) = blocker_registry.entry(resource_id)
        .or_insert(
            (
            unqualified_type_name::<T>().unwrap_or("???").to_owned(), 
                true
            )
        );
        *blocked = true;
    } else {
        let resource_id = TypeId::of::<T>();

        let (_, blocked) = blocker_registry.entry(resource_id)
        .or_insert(
            (
            unqualified_type_name::<T>().unwrap_or("???").to_owned(), 
                false
            )
        );
        *blocked = false;
    }
}

/// check if camera still blocked, unblock it if it isn't.
pub(crate) fn set_block_status(
    blocker_registry: ResMut<CameraBlockerRegistry>,
    mut block_status: ResMut<CameraControlBlocked>,
) {
    let mut camera_still_blocked = false;
    for (_, blocker) in blocker_registry.0.values() {
        if blocker == &true {
            camera_still_blocked = true;
        }
    }
    block_status.0 = camera_still_blocked;
}

pub fn init_cameras(
    camera_control_mesh: Res<CameraControlMesh>,
    mut ambient_light: ResMut<AmbientLight>,
    mut commands: Commands
) {
    let selection_mesh = camera_control_mesh.0.clone();
    commands
        .spawn((
            Mesh3d(selection_mesh),
            Visibility::Visible,
            Transform::default(),
            MeshMaterial3d::<StandardMaterial>::default(),
            CameraSelectionMarker,
            Name::new("selection_marker")
        ));

    let perspective_headlight = commands
        .spawn((
            DirectionalLight {
            shadows_enabled: false,
            illuminance: 50.,
            ..default()
            },
            PerspectiveHeadlightTarget,
            Name::new("perspective_headlight")
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
            Name::new("perspective_base_camera")
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
            Name::new("orthographic_base_camera")

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
    ortho_cams: Query<Entity, With<OrthographicCameraRoot>>,
    persp_cams: Query<Entity, With<PerspectiveCameraRoot>>,
) {
    let projection_mode = *projection_mode;

    match projection_mode {
        ProjectionMode::Perspective => {
            for ortho_cam in ortho_cams {
                let Ok((mut camera, mut visibility)) = cameras.get_mut(ortho_cam) else {
                    continue
                };
                camera.is_active = false;
                *visibility = Visibility::Hidden;
            }
            for persp_cam in persp_cams {
                let Ok((mut camera, mut visibility)) = cameras.get_mut(persp_cam) else {
                    continue
                };
                camera.is_active = true;
                *visibility = Visibility::Inherited;
            }
        },
        ProjectionMode::Orthographic => {
            for ortho_cam in ortho_cams {
                let Ok((mut camera, mut visibility)) = cameras.get_mut(ortho_cam) else {
                    continue
                };
                camera.is_active = true;
                *visibility = Visibility::Inherited;
            }
            for persp_cam in persp_cams {
                let Ok((mut camera, mut visibility)) = cameras.get_mut(persp_cam) else {
                    continue
                };
                camera.is_active = false;
                *visibility = Visibility::Hidden;
            }
        },
    }
}

pub fn toggle_headlights(
    headlight_toggle: Res<HeadlightToggle>,
    projection_mode: Res<ProjectionMode>,
    mut visibility: Query<&mut Visibility>,
    mut perspective_headlights: Query<Entity, With<PerspectiveHeadlightTarget>>,
    mut orthographic_headlights: Query<Entity, With<PerspectiveHeadlightTarget>>
) {
    let targets = match *projection_mode {
        ProjectionMode::Perspective => {
            perspective_headlights.iter_mut().collect::<Vec<_>>()
        },
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

pub fn camera_controls(
    mut cursor_command: ResMut<CursorCommand>,
    mut keyboard_command: ResMut<KeyboardCommand>,
    mut cameras: Query<(&mut Projection, &mut Transform)>,
    projection_mode: Res<ProjectionMode>,
    camera_blocked: Res<CameraControlBlocked>,
    perspective_cam_root: Query<(Entity, &Children), With<PerspectiveCameraRoot>>,
    orthographic_cam_root: Query<(Entity, &Children), With<OrthographicCameraRoot>>,
) {
    // don't run camera controls if something else has taken priority over it.
    if camera_blocked.0 {
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

    match *projection_mode {
        ProjectionMode::Perspective => {
            let Ok((e, children)) = perspective_cam_root.single()
            .inspect_err(|err| warn!("could not get perspective cam entity due to: {:#}", err)) else {
                return;
            };
            
            let new_child_proj = {
                let Ok((mut persp_proj, mut persp_transform)) = cameras.get_mut(e)
                .inspect_err(|err| warn!("could not get perspective cam components due to {:#}", err)) else {
                    return
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


            
            let valid_children = children.iter()
            .filter(|n| cameras.contains(*n)).collect::<Vec<_>>();

            for child in &valid_children {
                if let Ok((mut child_proj, _)) = cameras.get_mut(*child) {
                    *child_proj = new_child_proj.clone();
                }
            }
        },
        ProjectionMode::Orthographic => {
            let Ok((e, children)) = orthographic_cam_root.single()
            .inspect_err(|err| warn!("could not get perspective cam entity due to: {:#}", err)) else {
                return;
            };
            
            let new_child_proj = {
                let Ok((mut ortho_proj, mut ortho_transform)) = cameras.get_mut(e)
                .inspect_err(|err| warn!("could not get perspective cam components due to {:#}", err)) else {
                    return
                };
                
                if let Projection::Orthographic(ortho_proj) = ortho_proj.as_mut() {
                    ortho_transform.translation += translation_delta;
                    ortho_transform.rotation *= rotation_delta;
                    ortho_proj.scale += scale_delta;
                }

                ortho_proj.clone()
            };

            let valid_children = children.iter()
            .filter(|n| cameras.contains(*n)).collect::<Vec<_>>();

            for child in &valid_children {
                if let Ok((mut child_proj, _)) = cameras.get_mut(*child) {
                    *child_proj = new_child_proj.clone();
                }
            }
        },
    }
}
pub fn update_orbit_center_marker(
    //camera: Single<(&CameraControls, &ProjectionMode)>,
    controls: Res<CameraControls>,
    projection_mode: Res<ProjectionMode>,
    keyboard_command: Res<KeyboardCommand>,
    cursor_command: Res<CursorCommand>,
    camera_orbit_mat: Res<CameraOrbitMat>,
    mut gizmo: Gizmos,
    mut marker_query: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
            &CameraSelectionMarker
        ),
        Without<Projection>,
    >,
) {
    //let (controls, mode) = *camera_controls;
    let Ok((mut marker_transform, mut marker_visibility, mut marker_material, _)) = marker_query
    .single_mut() 
    .inspect_err(|err| {
        warn!("could not update orbit marker due to: {:#}", err)
    })
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
            *marker_material =
                MeshMaterial3d(camera_orbit_mat.0.clone());
            marker_transform.translation = orbit_center;
            gizmo.sphere(
                Isometry3d::new(orbit_center, Quat::IDENTITY),
                0.1,
                LIME,
            );
        }
    // Panning
    } else if cursor_command.command_type == CameraCommandType::Pan {
        if let Some(cursor_selection) = cursor_command.cursor_selection {
            *marker_visibility = Visibility::Visible;
            *marker_material =
                MeshMaterial3d(camera_orbit_mat.0.clone());
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
