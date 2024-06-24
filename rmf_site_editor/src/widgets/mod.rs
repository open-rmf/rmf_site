/*
 * Copyright (C) 2022 Open Source Robotics Foundation
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

//! The site editor allows you to insert your own egui widgets into the UI.
//! Simple examples of custom widgets can be found in the docs for
//! [`PropertiesTilePlugin`] and [`InspectionPlugin`].
//!
//! There are three categories of widgets that the site editor provides
//! out-of-the-box support for inserting, but the widget system itself is
//! highly extensible, allowing you to define your own categories of widgets.
//!
//! The three categories provided out of the box include:
//! - [Panel widget][1]: Add a new panel to the UI.
//! - Tile widget: Add a tile into a [panel of tiles][2] such as the [`PropertiesPanel`]. Use [`PropertiesTilePlugin`] to make a new tile widget that goes inside of the standard `PropertiesPanel`.
//! - [`InspectionPlugin`]: Add a widget to the [`MainInspector`] to display more information about the currently selected entity.
//!
//! In our terminology, there are two kinds of panels:
//! - Side panels: A vertical column widget on the left or right side of the screen.
//!   - [`PropertiesPanel`] is usually a side panel placed on the right side of the screen.
//!   - [`FuelAssetBrowser`] is a side panel typically placed on the left side of the screen.
//!   - [`Diagnostics`] is a side panel that interactively flags issues that have been found in the site.
//! - Top / Bottom Panels:
//!   - The [`MenuBarPlugin`] provides a menu bar at the top of the screen.
//!     - Create an entity with a [`Menu`] component to create a new menu inside the menu bar.
//!     - Add an entity with a [`MenuItem`] component as a child to a menu entity to add a new item into a menu.
//!     - The [`FileMenu`], [`ToolMenu`], and [`ViewMenu`] are resources that provide access to various standard menus.
//!   - The [`ConsoleWidgetPlugin`] provides a console at the bottom of the screen to display information, warning, and error messages.
//!
//! [1]: crate::widgets::PanelWidget
//! [2]: crate::widgets::show_panel_of_tiles

use crate::{
    interaction::{Hover, PickingBlockers},
    AppState,
};
use bevy::{
    ecs::{
        system::{SystemParam, SystemState},
        world::EntityWorldMut,
    },
    prelude::*,
};
use bevy_egui::{
    egui::{self, Ui},
    EguiContexts,
};

pub mod building_preview;
use building_preview::*;

pub mod console;
use console::*;

pub mod creation;
use creation::*;

pub mod diagnostics;
use diagnostics::*;

pub mod fuel_asset_browser;
pub use fuel_asset_browser::*;

pub mod icons;
pub use icons::*;

pub mod inspector;
pub use inspector::*;

pub mod menu_bar;
pub use menu_bar::*;

pub mod move_layer;
pub use move_layer::*;

pub mod panel_of_tiles;
pub use panel_of_tiles::*;

pub mod panel;
pub use panel::*;

pub mod properties_panel;
pub use properties_panel::*;

pub mod sdf_export_menu;
pub use sdf_export_menu::*;

pub mod selector_widget;
pub use selector_widget::*;

pub mod view_groups;
use view_groups::*;

pub mod view_layers;
use view_layers::*;

pub mod view_levels;
use view_levels::*;

pub mod view_lights;
use view_lights::*;

pub mod view_nav_graphs;
use view_nav_graphs::*;

pub mod view_occupancy;
use view_occupancy::*;

pub mod prelude {
    //! This module gives easy access to the traits, structs, and plugins that
    //! we expect downstream users are likely to want easy access to if they are
    //! implementing and inserting their own widgets.

    pub use super::{
        properties_panel::*, Inspect, InspectionPlugin, PanelSide, PanelWidget, PanelWidgetInput,
        PropertiesPanel, PropertiesTilePlugin, ShareableWidget, ShowError, ShowResult,
        ShowSharedWidget, Tile, TryShowWidgetEntity, TryShowWidgetWorld, Widget, WidgetSystem,
    };
    pub use bevy::ecs::{
        system::{SystemParam, SystemState},
        world::World,
    };
    pub use bevy_egui::egui::Ui;
}

/// This plugin provides the standard UI layout that was designed for the common
/// use cases of the site editor.
#[derive(Default)]
pub struct StandardUiPlugin {}

impl Plugin for StandardUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            IconsPlugin::default(),
            MenuBarPlugin::default(),
            SdfExportMenuPlugin::default(),
            StandardPropertiesPanelPlugin::default(),
            FuelAssetBrowserPlugin::default(),
            DiagnosticsPlugin::default(),
            ConsoleWidgetPlugin::default(),
        ))
        .add_systems(Startup, init_ui_style)
        .add_systems(
            Update,
            site_ui_layout.run_if(AppState::in_displaying_mode()),
        )
        .add_systems(
            PostUpdate,
            (
                resolve_light_export_file,
                resolve_nav_graph_import_export_files,
            )
                .run_if(AppState::in_site_mode()),
        );
    }
}

/// This component should be given to an entity that needs to be rendered as a
/// nested widget in the UI.
///
/// For standard types of widgets you don't need to create this component yourself,
/// instead use one of the generic convenience plugins:
/// - [`InspectionPlugin`]
/// - [`PropertiesTilePlugin`]
#[derive(Component)]
pub struct Widget<Input = (), Output = ()> {
    inner: Option<Box<dyn ExecuteWidget<Input, Output> + 'static + Send + Sync>>,
    _ignore: std::marker::PhantomData<(Input, Output)>,
}

impl<Input, Output> Widget<Input, Output>
where
    Input: 'static + Send + Sync,
    Output: 'static + Send + Sync,
{
    pub fn new<W>(world: &mut World) -> Self
    where
        W: WidgetSystem<Input, Output> + 'static + Send + Sync,
    {
        let inner = InnerWidget::<Input, Output, W> {
            state: SystemState::new(world),
            _ignore: Default::default(),
        };

        Self {
            inner: Some(Box::new(inner)),
            _ignore: Default::default(),
        }
    }
}

/// Do not implement this widget directly. Instead create a struct that derives
/// [`SystemParam`] and then implement [`WidgetSystem`] for that struct.
pub trait ExecuteWidget<Input, Output> {
    fn show(&mut self, input: Input, ui: &mut Ui, world: &mut World) -> Output;
}

/// Implement this on a [`SystemParam`] struct to make it a widget that can be
/// plugged into the site editor UI.
///
/// See documentation of [`PropertiesTilePlugin`] or [`InspectionPlugin`] to see
/// examples of using this.
pub trait WidgetSystem<Input = (), Output = ()>: SystemParam {
    fn show(input: Input, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> Output;
}

struct InnerWidget<Input, Output, W: WidgetSystem<Input, Output> + 'static> {
    state: SystemState<W>,
    _ignore: std::marker::PhantomData<(Input, Output)>,
}

impl<Input, Output, W> ExecuteWidget<Input, Output> for InnerWidget<Input, Output, W>
where
    W: WidgetSystem<Input, Output>,
{
    fn show(&mut self, input: Input, ui: &mut Ui, world: &mut World) -> Output {
        let u = W::show(input, ui, &mut self.state, world);
        self.state.apply(world);
        u
    }
}

pub type ShowResult<T = ()> = Result<T, ShowError>;

/// Errors that can happen while attempting to show a widget.
#[derive(Debug)]
pub enum ShowError {
    /// The entity whose widget you are trying to show is missing from the world
    EntityMissing,
    /// There is no [`Widget`] component for the entity
    WidgetMissing,
    /// The entity has a [`Widget`] component, but the widget is already in use,
    /// which implies that we are trying to render the widget recursively, and
    /// that is not supported due to soundness issues.
    Recursion,
}

/// Trait implemented on [`World`] to let it render child widgets. Note that
/// this is not able to render widgets recursively, so you should make sure not
/// to have circular dependencies in your widget structure.
pub trait TryShowWidgetWorld {
    /// Try to show a widget that has `()` for input and output belonging to the
    /// specified entity.
    fn try_show(&mut self, entity: Entity, ui: &mut Ui) -> ShowResult<()> {
        self.try_show_out(entity, (), ui)
    }

    /// Same as [`Self::try_show`] but takes an input that will be fed to the widget.
    fn try_show_in<Input>(&mut self, entity: Entity, input: Input, ui: &mut Ui) -> ShowResult<()>
    where
        Input: 'static + Send + Sync,
    {
        self.try_show_out(entity, input, ui)
    }

    /// Same as [`Self::try_show`] but takes an input for the widget and provides
    /// an output from the widget.
    fn try_show_out<Output, Input>(
        &mut self,
        entity: Entity,
        input: Input,
        ui: &mut Ui,
    ) -> ShowResult<Output>
    where
        Input: 'static + Send + Sync,
        Output: 'static + Send + Sync;
}

impl TryShowWidgetWorld for World {
    fn try_show_out<Output, Input>(
        &mut self,
        entity: Entity,
        input: Input,
        ui: &mut Ui,
    ) -> ShowResult<Output>
    where
        Input: 'static + Send + Sync,
        Output: 'static + Send + Sync,
    {
        let Some(mut entity_mut) = self.get_entity_mut(entity) else {
            return Err(ShowError::EntityMissing);
        };
        entity_mut.try_show_out(input, ui)
    }
}

/// Same as [`TryShowWidgetWorld`] but is implemented for [`EntityWorldMut`] so
/// you do not need to specify the target entity.
pub trait TryShowWidgetEntity {
    /// Try to show a widget that has `()` for input and output
    fn try_show(&mut self, ui: &mut Ui) -> ShowResult<()> {
        self.try_show_out((), ui)
    }

    fn try_show_in<Input>(&mut self, input: Input, ui: &mut Ui) -> ShowResult<()>
    where
        Input: 'static + Send + Sync,
    {
        self.try_show_out(input, ui)
    }

    fn try_show_out<Output, Input>(&mut self, input: Input, ui: &mut Ui) -> ShowResult<Output>
    where
        Input: 'static + Send + Sync,
        Output: 'static + Send + Sync;
}

impl<'w> TryShowWidgetEntity for EntityWorldMut<'w> {
    fn try_show_out<Output, Input>(&mut self, input: Input, ui: &mut Ui) -> ShowResult<Output>
    where
        Input: 'static + Send + Sync,
        Output: 'static + Send + Sync,
    {
        let Some(mut widget) = self.get_mut::<Widget<Input, Output>>() else {
            return Err(ShowError::WidgetMissing);
        };

        let Some(mut inner) = widget.inner.take() else {
            return Err(ShowError::Recursion);
        };

        let output = self.world_scope(|world| inner.show(input, ui, world));

        if let Some(mut widget) = self.get_mut::<Widget<Input, Output>>() {
            widget.inner = Some(inner);
        }

        Ok(output)
    }
}

/// This is a marker trait to indicate that the system state of a widget can be
/// safely shared across multiple renders of the widget. For example, the system
/// parameters do not use the [`Changed`] filter. It is the responsibility of
/// the user to ensure that sharing this widget will not have any bad side
/// effects.
///
/// [`ShareableWidget`]s can be used by the [`ShowSharedWidget`] trait which is
/// implemented for the [`World`] struct.
pub trait ShareableWidget {}

/// A resource to store a widget so that it can be reused multiple times in one
/// render pass.
#[derive(Resource)]
pub struct SharedWidget<W: SystemParam + ShareableWidget + 'static> {
    state: SystemState<W>,
}

/// This gives a convenient function for rendering a widget using a world.
pub trait ShowSharedWidget {
    fn show<W, Output, Input>(&mut self, input: Input, ui: &mut Ui) -> Output
    where
        W: ShareableWidget + WidgetSystem<Input, Output> + 'static;
}

impl ShowSharedWidget for World {
    fn show<W, Output, Input>(&mut self, input: Input, ui: &mut Ui) -> Output
    where
        W: ShareableWidget + WidgetSystem<Input, Output> + 'static,
    {
        if !self.contains_resource::<SharedWidget<W>>() {
            let widget = SharedWidget::<W> {
                state: SystemState::new(self),
            };
            self.insert_resource(widget);
        }

        self.resource_scope::<SharedWidget<W>, Output>(|world, mut widget| {
            let u = W::show(input, ui, &mut widget.state, world);
            widget.state.apply(world);
            u
        })
    }
}

/// This system renders all UI panels in the application and makes sure that the
/// UI rendering works correctly with the picking system, and any other systems
/// as needed.
pub fn site_ui_layout(
    world: &mut World,
    panel_widgets: &mut QueryState<(Entity, &mut PanelWidget)>,
    egui_context_state: &mut SystemState<EguiContexts>,
) {
    render_panels(world, panel_widgets, egui_context_state);

    let mut egui_context = egui_context_state.get_mut(world);
    let ctx = egui_context.ctx_mut();
    let ui_has_focus =
        ctx.wants_pointer_input() || ctx.wants_keyboard_input() || ctx.is_pointer_over_area();

    if let Some(mut picking_blocker) = world.get_resource_mut::<PickingBlockers>() {
        picking_blocker.ui = ui_has_focus;
    }

    if ui_has_focus {
        // If the UI has focus and there were no hover events emitted by the UI,
        // then we should emit a None hover event
        let mut hover = world.resource_mut::<Events<Hover>>();
        if hover.is_empty() {
            hover.send(Hover(None));
        }
    }
}

fn init_ui_style(mut egui_context: EguiContexts) {
    // I think the default egui dark mode text color is too dim, so this changes
    // it to a brighter white.
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(250, 250, 250));
    egui_context.ctx_mut().set_visuals(visuals);
}
