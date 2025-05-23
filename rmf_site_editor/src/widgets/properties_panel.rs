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

use crate::widgets::{
    show_panel_of_tiles, BuildingPreviewPlugin, PanelSide, PanelWidget, StandardInspectorPlugin,
    Tile, ViewGroupsPlugin, ViewLayersPlugin, ViewLevelsPlugin, ViewLightsPlugin,
    ViewModelInstancesPlugin, ViewNavGraphsPlugin, ViewOccupancyPlugin, ViewScenariosPlugin,
    ViewTasks, Widget, WidgetSystem,
};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};

/// This plugins produces the standard properties panel. This is the panel which
/// includes widgets to display and edit all the properties in a site that we
/// expect are needed by common use cases of the editor.
#[derive(Default)]
pub struct StandardPropertiesPanelPlugin {}

impl Plugin for StandardPropertiesPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PropertiesPanelPlugin::new(PanelSide::Right),
            ViewLevelsPlugin::default(),
            ViewScenariosPlugin::default(),
            ViewModelInstancesPlugin::default(),
            ViewNavGraphsPlugin::default(),
            ViewLayersPlugin::default(),
            StandardInspectorPlugin::default(),
            ViewGroupsPlugin::default(),
            PropertiesTilePlugin::<ViewTasks>::new(),
            ViewLightsPlugin::default(),
            ViewOccupancyPlugin::default(),
            BuildingPreviewPlugin::default(),
        ));
    }
}

/// Use this plugin to add a single tile into the properties panel.
///
/// ```no_run
/// use bevy::prelude::{App, Query, Entity, Res};
/// use librmf_site_editor::{
///     SiteEditor, workspace::CurrentWorkspace,
///     site::NameOfSite,
///     widgets::prelude::*,
/// };
///
/// #[derive(SystemParam)]
/// pub struct HelloSiteWidget<'w, 's> {
///     sites: Query<'w, 's, &'static NameOfSite>,
///     current: Res<'w, CurrentWorkspace>,
/// }
///
/// impl<'w, 's> WidgetSystem<Tile> for HelloSiteWidget<'w, 's> {
///     fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
///         let mut params = state.get_mut(world);
///         if let Some(name) = params.current.root.map(|e| params.sites.get(e).ok()).flatten() {
///             ui.add_space(20.0);
///             ui.heading(format!("Hello, {}!", name.0));
///         }
///     }
/// }
///
/// fn main() {
///     let mut app = App::new();
///     app.add_plugins((
///         SiteEditor::default(),
///         PropertiesTilePlugin::<HelloSiteWidget>::new(),
///     ));
///
///     app.run();
/// }
/// ```
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
        let widget = Widget::<Tile>::new::<W>(app.world_mut());
        let properties_panel = app.world().resource::<PropertiesPanel>().id;
        app.world_mut()
            .spawn(widget)
            .insert(ChildOf(properties_panel));
    }
}

/// Get the ID of the properties panel.
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

/// This plugin builds a properties panel for the editor. It is usually recommended
/// to use [`StandardPropertiesPanelPlugin`] unless you need very specific
/// customization of the properties panel.
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
        let widget = PanelWidget::new(show_panel_of_tiles, app.world_mut());
        let id = app.world_mut().spawn((widget, self.side)).id();
        app.world_mut().insert_resource(PropertiesPanel {
            side: self.side,
            id,
        });
    }
}
