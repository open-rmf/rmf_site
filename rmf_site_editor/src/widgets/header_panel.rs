/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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

use crate::widgets::{
    show_panel_of_tiles, PanelConfig, PanelSide, PanelWidget, ScrollConfig, Tile, Widget,
    WidgetSystem,
};
use bevy::prelude::*;

pub struct HeaderTilePlugin<W>
where
    W: WidgetSystem<Tile> + 'static + Send + Sync,
{
    _ignore: std::marker::PhantomData<W>,
}

impl<W> HeaderTilePlugin<W>
where
    W: WidgetSystem<Tile> + 'static + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<W> Plugin for HeaderTilePlugin<W>
where
    W: WidgetSystem<Tile> + 'static + Send + Sync,
{
    fn build(&self, app: &mut App) {
        let widget = Widget::<Tile>::new::<W>(&mut app.world);
        let header_panel = app.world.resource::<HeaderPanel>().id;
        app.world.spawn(widget).set_parent(header_panel);
    }
}

#[derive(Resource)]
pub struct HeaderPanel {
    id: Entity,
    side: PanelSide,
}

impl HeaderPanel {
    pub fn id(&self) -> Entity {
        self.id
    }

    pub fn side(&self) -> PanelSide {
        self.side
    }
}

/// This plugin builds a header panel for the editor
pub struct HeaderPanelPlugin {
    side: PanelSide,
}

impl HeaderPanelPlugin {
    pub fn new(side: PanelSide) -> Self {
        Self { side }
    }
}

impl Default for HeaderPanelPlugin {
    fn default() -> Self {
        Self::new(PanelSide::Top)
    }
}

impl Plugin for HeaderPanelPlugin {
    fn build(&self, app: &mut App) {
        let widget = PanelWidget::new(show_panel_of_tiles, &mut app.world);
        let id = app
            .world
            .spawn((
                widget,
                self.side,
                PanelConfig {
                    resizable: false,
                    default_dimension: 30.0,
                    horizontal_scrolling: ScrollConfig {
                        enable_scroll: false,
                        auto_shrink: true,
                    },
                    vertical_scrolling: ScrollConfig {
                        enable_scroll: false,
                        auto_shrink: true,
                    },
                },
            ))
            .id();
        app.world.insert_resource(HeaderPanel {
            side: self.side,
            id,
        });
    }
}
