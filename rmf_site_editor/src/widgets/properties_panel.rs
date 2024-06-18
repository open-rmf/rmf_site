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

use crate::widgets::{tile_panel_widget, PanelSide, PanelWidget, Tile, Widget, WidgetSystem};
use bevy::prelude::*;

#[derive(Resource)]
pub struct PropertiesPanel {
    side: PanelSide,
    id: Entity,
}

impl PropertiesPanel {
    pub fn side(&self) -> PanelSide {
        self.side
    }

    pub fn id(&self) -> Entity {
        self.id
    }
}

pub struct PropertiesPanelPlugin {
    side: PanelSide,
}

impl PropertiesPanelPlugin {
    pub fn new(side: PanelSide) -> Self {
        Self { side }
    }
}

impl Default for PropertiesPanelPlugin {
    fn default() -> Self {
        Self::new(PanelSide::Right)
    }
}

impl Plugin for PropertiesPanelPlugin {
    fn build(&self, app: &mut App) {
        let widget = PanelWidget::new(tile_panel_widget, &mut app.world);
        let id = app.world.spawn((widget, self.side)).id();
        app.world.insert_resource(PropertiesPanel {
            side: self.side,
            id,
        });
    }
}

/// Create a plugin for a single tile in the properties panel
pub struct PropertiesTilePlugin<W>
where
    W: WidgetSystem<Tile> + 'static + Send + Sync,
{
    _ignore: std::marker::PhantomData<W>,
}

impl<W> PropertiesTilePlugin<W>
where
    W: WidgetSystem<Tile> + 'static + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<W> Plugin for PropertiesTilePlugin<W>
where
    W: WidgetSystem<Tile> + 'static + Send + Sync,
{
    fn build(&self, app: &mut App) {
        let widget = Widget::<Tile>::new::<W>(&mut app.world);
        let properties_panel = app.world.resource::<PropertiesPanel>().id;
        app.world.spawn(widget).set_parent(properties_panel);
    }
}
