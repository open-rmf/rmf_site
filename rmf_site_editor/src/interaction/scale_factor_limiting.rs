use bevy::prelude::*;

use super::{CameraControls, ProjectionMode};

/// Adding this component will ensure that your 3d model has
/// a minimum size on the screen. This is useful for things like
/// tooltips and gizmos. Think of it as like setting a `min_width`
/// in CSS.
#[derive(Debug, Clone, Component)]
pub struct LimitScaleFactor {
    /// If the camera is closer than this, the size of the object
    /// will not apply any correction. Beyong this distance the `limit_size`
    /// system will rescale the object to make sure that the object is
    /// always visible in the screen space.
    pub distance_to_start_scaling: f32,
    /// Original scale of the object.
    pub original_scale: f32,
}

/// This system limits the amount of shrinkining that a model
/// undergoes in screen space. This allows users to zoom out
/// but also ensures that the item is visible no matter how far
/// the user is from the mesh. This is really useful for things
/// like gizmos and tool tips.
pub fn limit_size(
    item_to_limit_scale: Query<(&LimitScaleFactor, Entity)>,
    camera_controls: Res<CameraControls>,
    transforms: Query<&GlobalTransform>,
    mut editable_transforms: Query<&mut Transform>,
) {
    let view_cam_entity = match camera_controls.mode() {
        ProjectionMode::Perspective => camera_controls.perspective_camera_entities[0],
        ProjectionMode::Orthographic => camera_controls.orthographic_camera_entities[0],
    };

    let Ok(camera_transform) = transforms.get(view_cam_entity) else {
        return;
    };

    for (limits, entity) in item_to_limit_scale.iter() {
        let Ok(item_to_scale_transform) = transforms.get(entity) else {
            continue;
        };

        let dist = camera_transform.translation() - item_to_scale_transform.translation();

        if dist.length() > limits.distance_to_start_scaling {
            let Ok(mut item_to_scale) = editable_transforms.get_mut(entity) else {
                continue;
            };
            item_to_scale.scale = Vec3::splat(dist.length() / limits.distance_to_start_scaling)
                * limits.original_scale;
        } else {
            let Ok(mut item_to_scale) = editable_transforms.get_mut(entity) else {
                continue;
            };
            item_to_scale.scale = Vec3::ONE * limits.original_scale;
        }
    }
}
