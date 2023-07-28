use bevy::prelude::*;

pub mod color_based_picking;
pub use color_based_picking::*;

pub mod color_entity_mapping;
pub use color_entity_mapping::*;

use super::{LINE_PICKING_LAYER, POINT_PICKING_LAYER};

pub struct ColorBasedPicker;

impl Plugin for ColorBasedPicker {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(resize_notificator::<LINE_PICKING_LAYER>)
        .add_system(resize_notificator::<POINT_PICKING_LAYER>)
        .add_system_to_stage(
            CoreStage::PostUpdate,
            buffer_to_selection::<POINT_PICKING_LAYER>,
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            buffer_to_selection::<LINE_PICKING_LAYER>,
        )
        .init_resource::<ColorEntityMap>()
        .add_system(color_entity_mapping_system::<LINE_PICKING_LAYER>)
        .add_system(color_entity_mapping_system::<POINT_PICKING_LAYER>);
    }
}