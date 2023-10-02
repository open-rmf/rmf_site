use bevy::prelude::*;

pub mod rendering_helper;
pub use rendering_helper::*;

pub mod color_entity_mapping;
pub use color_entity_mapping::*;

pub mod camera_capture;
pub use camera_capture::*;

use super::{LINE_PICKING_LAYER, POINT_PICKING_LAYER};

pub struct ColorBasedPicker;

pub struct GPUPickItem(pub Entity);

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
            .add_event::<GPUPickItem>()
            .add_system(new_objectcolor_entity_mapping::<LINE_PICKING_LAYER>)
            .add_system(new_objectcolor_entity_mapping::<POINT_PICKING_LAYER>)
            .add_system(sync_polyline_selection_buffer)
            .add_plugin(ImageCopyPlugin);
    }
}
