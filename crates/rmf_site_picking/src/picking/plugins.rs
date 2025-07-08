use bevy_app::prelude::*;
use rmf_site_camera::plugins::BlockerRegistryPlugin;

use crate::*;

/// Picking plugin/setup for this project.
pub(crate) struct PickingRMFPlugin;

impl Plugin for PickingRMFPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ChangePick>()
            .init_resource::<Picked>()
            .init_resource::<PickingBlockers>()
            .add_plugins(BlockerRegistryPlugin::<PickingBlockers>::default())
            .add_plugins(PickBlockerRegistration::<UiFocused>::default())
            .add_plugins(PickBlockerRegistration::<IteractionMaskHovered>::default())
            .add_systems(First, update_picked)
            .add_systems(PreUpdate, check_ui_focus);
    }
}
