pub mod create_edges;
use create_edges::*;

pub mod create_path;
use create_path::*;

pub mod create_point;
use create_point::*;

pub mod place_object;
pub use place_object::*;

pub mod place_object_2d;
pub use place_object_2d::*;

pub mod replace_point;
use replace_point::*;

pub mod replace_side;
use replace_side::*;

pub mod select_anchor;
use rmf_site_format::Pending;
use rmf_site_picking::{Hover, Select, Selectable, SelectionFilter};
pub use select_anchor::*;

use crate::interaction::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use rmf_site_picking::Preview;

pub const SELECT_ANCHOR_MODE_LABEL: &'static str = "select_anchor";

#[derive(SystemParam)]
pub struct InspectorFilter<'w, 's> {
    selectables: Query<'w, 's, &'static Selectable, (Without<Preview>, Without<Pending>)>,
}

impl<'w, 's> SelectionFilter for InspectorFilter<'w, 's> {
    fn filter_pick(&mut self, select: Entity) -> Option<Entity> {
        self.selectables
            .get(select)
            .ok()
            .map(|selectable| selectable.element)
    }
    fn filter_select(&mut self, target: Entity) -> Option<Entity> {
        Some(target)
    }
    fn on_click(&mut self, hovered: Hover) -> Option<Select> {
        Some(Select::new(hovered.0))
    }
}

#[derive(Default, Resource)]
pub struct CreationSettings {
    pub direction_alignment: Vec<Vec2>,
}

impl CreationSettings {
    pub fn reset(&mut self) {
        self.direction_alignment = Vec::new();
    }
}

pub fn apply_creation_settings(
    creation_settings: Res<CreationSettings>,
    cursor: Res<Cursor>,
    transform: Query<&Transform>,
    mut move_to: EventWriter<MoveTo>,
) {
    let Ok(mut end_anchor_tf) = transform
        .get(cursor.level_anchor_placement)
        .map(|tf| tf.clone())
    else {
        return;
    };
    end_anchor_tf.translation = Vec3::default();
    for alignment in &creation_settings.direction_alignment {
        end_anchor_tf.translation += alignment.extend(0.0);
    }

    move_to.write(MoveTo {
        entity: cursor.level_anchor_placement,
        transform: end_anchor_tf,
    });
}
