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

use crate::*;
use bevy_app::prelude::*;
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
        let widget = Widget::<Tile>::new::<W>(app.world_mut());
        let header_panel = app.world().resource::<HeaderPanel>().id;
        app.world_mut().spawn(widget).insert(ChildOf(header_panel));
    }
}

#[derive(Resource)]
pub struct HeaderPanel {
    id: Entity,
    settings: PanelSettings,
}

impl HeaderPanel {
    pub fn id(&self) -> Entity {
        self.id
    }

    pub fn side(&self) -> PanelSide {
        self.settings.side
    }
}

/// This plugin builds a header panel for the editor
pub struct HeaderPanelPlugin {
    settings: PanelSettings,
}

impl HeaderPanelPlugin {
    pub fn new(settings: PanelSettings) -> Self {
        Self { settings }
    }
}

impl Default for HeaderPanelPlugin {
    fn default() -> Self {
        Self::new(PanelSettings::top())
    }
}

impl Plugin for HeaderPanelPlugin {
    fn build(&self, app: &mut App) {
        let widget = PanelWidget::new(show_panel_of_tiles, app.world_mut());
        let id = app
            .world_mut()
            .spawn((
                widget,
                self.settings,
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
        app.world_mut().insert_resource(HeaderPanel {
            settings: self.settings,
            id,
        });
    }
}
