use bevy_ecs::prelude::*;

/// Marks entity as the camera's pick marker.
#[derive(Component)]
pub struct CameraPickMarker;

/// Marks camera as perspective headlight togglable.
#[derive(Component)]
pub struct PerspectiveHeadlightTarget;

/// Marks camera as orthographic headlight togglable
#[derive(Component)]
pub struct OrthographicHeadlightTarget;

/// Marks the entity as the root camera of a perspective camera hierarchy.
#[derive(Component)]
pub struct PerspectiveCameraRoot;

/// Marks the entity as the root camera of a orthographic camera hierarchy.
#[derive(Component)]
pub struct OrthographicCameraRoot;
