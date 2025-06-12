use bevy_ecs::prelude::*;

#[derive(Component)]
pub struct CameraSelectionMarker;

#[derive(Component)]
pub struct PerspectiveHeadlightTarget;

#[derive(Component)]
pub struct OrthographicHeadlightTarget;

#[derive(Component)]
pub struct PerspectiveCameraRoot;


#[derive(Component)]
pub struct OrthographicCameraRoot;