use crate::SdfPlugin;
use bevy::prelude::*;
/// A plugin used to consolidate asset loaders for types supported in the Gazebo / ROS ecosystem.
pub struct AssetLoadersPlugin;

impl Plugin for AssetLoadersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((bevy_stl::StlPlugin, bevy_obj::ObjPlugin, SdfPlugin));
    }
}
