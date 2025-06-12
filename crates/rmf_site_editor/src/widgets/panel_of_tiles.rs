/*
 * Copyright (C) 2024 Open Source Robotics Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/

use crate::widgets::prelude::*;

use bevy::prelude::*;
use bevy_egui::egui;
use smallvec::SmallVec;

/// Input type for [`WidgetSystem`]s that can be put into a "Panel of Tiles"
/// widget, such as the [`PropertiesPanel`]. See [`PropertiesTilePlugin`] for a
/// usage example.
pub struct Tile {
    /// The entity of the tile widget which is being rendered. This lets you
    /// store additional component data inside the entity which may be relevant
    /// to your widget.
    pub id: Entity,
    /// What kind of panel is this tile inside of. Use this if you want your
    /// widget layout to be different based on what kind of panel it was placed
    /// in.
    pub panel: PanelSide,
}

/// Reusable widget that defines a panel with "tiles" where each tile is a child widget.
pub fn show_panel_of_tiles(
    In(PanelWidgetInput { id, context }): In<PanelWidgetInput>,
    world: &mut World,
) {
    let children: Option<SmallVec<[Entity; 16]>> = world
        .get::<Children>(id)
        .map(|children| children.iter().collect());

    let Some(children) = children else {
        return;
    };
    if children.is_empty() {
        // Do not even begin to create a panel if there are no children to render
        return;
    }

    let Some(side) = world.get::<PanelSide>(id) else {
        error!("Side component missing for panel_of_tiles_widget {id:?}");
        return;
    };

    let side = *side;

    let config = world.get::<PanelConfig>(id).cloned().unwrap_or_default();

    side.get_panel()
        .map_vertical(|panel| {
            panel
                .resizable(config.resizable)
                .default_width(config.default_dimension)
        })
        .map_horizontal(|panel| {
            panel
                .resizable(config.resizable)
                .default_height(config.default_dimension)
        })
        .show(&context, |ui| {
            egui::ScrollArea::new(config.enable_scroll())
                .auto_shrink(config.auto_shrink())
                .show(ui, |ui| {
                    side.align(ui, |ui| render_tiles(ui, world, &children, side, id));
                });
        });
}

fn render_tiles(ui: &mut Ui, world: &mut World, children: &[Entity], side: PanelSide, id: Entity) {
    for &child in children {
        let tile = Tile {
            id: child,
            panel: side,
        };
        if let Err(err) = world.try_show_in(child, tile, ui) {
            error!(
                "Could not render child widget {child:?} in \
                                tile panel {id:?} on side {side:?}: {err:?}"
            );
        }
    }
}
