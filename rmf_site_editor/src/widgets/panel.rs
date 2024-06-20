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

use bevy::{prelude::*, ecs::system::{BoxedSystem, SystemState}};
use bevy_egui::{EguiContexts, egui::{self, Ui}};
use smallvec::SmallVec;

/// To create a panel widget (a widget that renders itself directly to one of
/// the egui side or top/bottom panels), add this component to an entity.
///
/// Use the context field of the input to create a panel with one of the following:
/// - [`EguiPanel::show`]
/// - [`egui::SidePanel::left`]
/// - [`egui::SidePanel::right`]
/// - [`egui::TopBottomPanel::top`]
/// - [`egui::TopBottomPanel::bottom`]
#[derive(Component)]
pub struct PanelWidget {
    inner: Option<BoxedSystem<PanelWidgetInput>>,
}

/// Input provided to panel widgets.
pub struct PanelWidgetInput {
    /// The entity of the panel widget.
    pub id: Entity,
    /// The context that the panel should use for rendering.
    pub context: egui::Context,
}

impl PanelWidget {
    /// Pass in a system that takes takes [`PanelWidgetInput`] as its input parameter.
    pub fn new<M, S: IntoSystem<PanelWidgetInput, (), M>>(system: S, world: &mut World) -> Self {
        let mut system = Box::new(IntoSystem::into_system(system));
        system.initialize(world);
        Self {
            inner: Some(system),
        }
    }
}

/// This function can be used to render all panels in an application, either by
/// adding this function to a schedule as a system or by calling it from inside
/// of an exclusive system. Note that this is automatically run by
/// [`site_ui_layout`][1] so there is no need to use this function yourself
/// unless you are not using the [`StandardUiLayout`][1].
///
/// [1]: crate::widgets::site_ui_layout
/// [2]: crate::widgets::StandardUiLayout
pub fn render_panels(
    world: &mut World,
    panel_widgets: &mut QueryState<(Entity, &mut PanelWidget)>,
    egui_contexts: &mut SystemState<EguiContexts>,
) {
    let context = egui_contexts.get_mut(world).ctx_mut().clone();
    let mut panels: SmallVec<[_; 16]> = panel_widgets
        .iter_mut(world)
        .map(|(entity, mut widget)| {
            (
                entity,
                widget
                    .inner
                    .take()
                    .expect("Inner system of PanelWidget is missing"),
            )
        })
        .collect();


    for (e, inner) in &mut panels {
        inner.run(PanelWidgetInput{ id: *e, context: context.clone() }, world);
        inner.apply_deferred(world);
    }

    for (e, inner) in panels {
        if let Some(mut widget) = world.get_mut::<PanelWidget>(e) {
            let _ = widget.inner.insert(inner);
        }
    }
}

/// Indicate which side a panel is on
#[derive(Clone, Copy, Debug, Component)]
pub enum PanelSide {
    Top,
    Bottom,
    Left,
    Right,
}

/// Wrapper to hold either a vertical or horizontal egui panel
pub enum EguiPanel {
    Vertical(egui::SidePanel),
    Horizontal(egui::TopBottomPanel),
}

impl EguiPanel {
    /// Modify this panel if it's a vertical panel
    pub fn map_vertical(self, f: impl FnOnce(egui::SidePanel) -> egui::SidePanel) -> Self {
        match self {
            Self::Vertical(panel) => Self::Vertical(f(panel)),
            other => other,
        }
    }

    /// Modify this panel if it's a horizontal panel
    pub fn map_horizontal(
        self,
        f: impl FnOnce(egui::TopBottomPanel) -> egui::TopBottomPanel,
    ) -> Self {
        match self {
            Self::Horizontal(panel) => Self::Horizontal(f(panel)),
            other => other,
        }
    }

    /// Display something in this panel.
    pub fn show<R>(
        self,
        ctx: &egui::Context,
        add_content: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<R> {
        match self {
            Self::Vertical(panel) => panel.show(ctx, add_content),
            Self::Horizontal(panel) => panel.show(ctx, add_content),
        }
    }
}

impl PanelSide {
    /// Is the long direction of the panel horizontal
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    /// Is the long direction of the panel vertical
    pub fn is_vertical(&self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    /// Align the Ui to line up with the long direction of the panel
    pub fn align<R>(self, ui: &mut Ui, f: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
        if self.is_horizontal() {
            ui.horizontal(f)
        } else {
            ui.vertical(f)
        }
    }

    /// Align the Ui to run orthogonal to long direction of the panel,
    /// i.e. the Ui will run along the short direction of the panel.
    pub fn orthogonal<R>(
        self,
        ui: &mut Ui,
        f: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<R> {
        if self.is_horizontal() {
            ui.vertical(f)
        } else {
            ui.horizontal(f)
        }
    }

    /// Get the egui panel that is associated with this panel type.
    pub fn get_panel(self) -> EguiPanel {
        match self {
            Self::Left => EguiPanel::Vertical(egui::SidePanel::left("left_panel")),
            Self::Right => EguiPanel::Vertical(egui::SidePanel::right("right_panel")),
            Self::Top => EguiPanel::Horizontal(egui::TopBottomPanel::top("top_panel")),
            Self::Bottom => EguiPanel::Horizontal(egui::TopBottomPanel::bottom("bottom_panel")),
        }
    }
}
