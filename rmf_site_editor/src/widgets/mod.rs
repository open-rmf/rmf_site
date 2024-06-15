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

use crate::{
    interaction::{
        ChangeMode, HeadlightToggle, Hover, MoveTo, PickingBlockers, Select, SpawnPreview,
    },
    log::LogHistory,
    occupancy::CalculateGrid,
    recency::ChangeRank,
    site::{
        AlignSiteDrawings, AssociatedGraphs, BeginEditDrawing, Change, ConsiderAssociatedGraph,
        ConsiderLocationTag, CurrentLevel, Delete, DrawingMarker, ExportLights, FinishEditDrawing,
        GlobalDrawingVisibility, GlobalFloorVisibility, JointProperties, LayerVisibility,
        MergeGroups, PhysicalLightToggle, SaveNavGraphs, Texture, ToggleLiftDoorAvailability,
    },
    workcell::CreateJoint,
    AppState, CreateNewWorkspace, CurrentWorkspace, LoadWorkspace, SaveWorkspace,
    ValidateWorkspace,
};
use bevy::{
    asset::embedded_asset,
    ecs::{
        query::Has,
        system::{SystemParam, SystemState, BoxedSystem},
        world::EntityWorldMut,
    },
    prelude::*,
};
use bevy_egui::{
    egui::{self, Button, CollapsingHeader, Ui},
    EguiContexts,
};
use smallvec::SmallVec;
use rmf_site_format::*;

pub mod create;
use create::*;

pub mod menu_bar;
use menu_bar::*;

pub mod view_groups;
use view_groups::*;

pub mod diagnostic_window;
use diagnostic_window::*;

pub mod view_layers;
use view_layers::*;

pub mod view_levels;
use view_levels::{LevelDisplay, LevelParams, ViewLevels, ViewLevelsPlugin};

pub mod view_lights;
use view_lights::*;

pub mod view_nav_graphs;
use view_nav_graphs::*;

pub mod view_occupancy;
use view_occupancy::*;

pub mod console;
pub use console::*;

pub mod icons;
pub use icons::*;

pub mod inspector;
// use inspector::{InspectorParams, InspectorWidget, SearchForFiducial, SearchForTexture, ExInspectorWidget};
use inspector::*;

pub mod move_layer;
pub use move_layer::*;

pub mod new_model;
pub use new_model::*;

pub mod selection_widget;
pub use selection_widget::*;

#[derive(Resource, Clone, Default)]
pub struct PendingDrawing {
    pub source: AssetSource,
    pub recall_source: RecallAssetSource,
}

#[derive(Resource, Clone, Default)]
pub struct PendingModel {
    pub source: AssetSource,
    pub recall_source: RecallAssetSource,
    pub scale: Scale,
}

#[derive(Default)]
pub struct StandardUiLayout;

fn add_widgets_icons(app: &mut App) {
    // Taken from https://github.com/bevyengine/bevy/issues/10377#issuecomment-1858797002
    // TODO(luca) remove once we migrate to Bevy 0.13 that includes the fix
    #[cfg(any(not(target_family = "windows"), target_env = "gnu"))]
    {
        embedded_asset!(app, "src/", "icons/add.png");
        embedded_asset!(app, "src/", "icons/alignment.png");
        embedded_asset!(app, "src/", "icons/alpha.png");
        embedded_asset!(app, "src/", "icons/confirm.png");
        embedded_asset!(app, "src/", "icons/down.png");
        embedded_asset!(app, "src/", "icons/edit.png");
        embedded_asset!(app, "src/", "icons/empty.png");
        embedded_asset!(app, "src/", "icons/exit.png");
        embedded_asset!(app, "src/", "icons/global.png");
        embedded_asset!(app, "src/", "icons/hidden.png");
        embedded_asset!(app, "src/", "icons/hide.png");
        embedded_asset!(app, "src/", "icons/merge.png");
        embedded_asset!(app, "src/", "icons/opaque.png");
        embedded_asset!(app, "src/", "icons/reject.png");
        embedded_asset!(app, "src/", "icons/search.png");
        embedded_asset!(app, "src/", "icons/select.png");
        embedded_asset!(app, "src/", "icons/selected.png");
        embedded_asset!(app, "src/", "icons/to_bottom.png");
        embedded_asset!(app, "src/", "icons/to_top.png");
        embedded_asset!(app, "src/", "icons/trash.png");
        embedded_asset!(app, "src/", "icons/up.png");
    }
    #[cfg(all(target_family = "windows", not(target_env = "gnu")))]
    {
        embedded_asset!(app, "src\\", "icons\\add.png");
        embedded_asset!(app, "src\\", "icons\\alignment.png");
        embedded_asset!(app, "src\\", "icons\\alpha.png");
        embedded_asset!(app, "src\\", "icons\\confirm.png");
        embedded_asset!(app, "src\\", "icons\\down.png");
        embedded_asset!(app, "src\\", "icons\\edit.png");
        embedded_asset!(app, "src\\", "icons\\empty.png");
        embedded_asset!(app, "src\\", "icons\\exit.png");
        embedded_asset!(app, "src\\", "icons\\global.png");
        embedded_asset!(app, "src\\", "icons\\hidden.png");
        embedded_asset!(app, "src\\", "icons\\hide.png");
        embedded_asset!(app, "src\\", "icons\\merge.png");
        embedded_asset!(app, "src\\", "icons\\opaque.png");
        embedded_asset!(app, "src\\", "icons\\reject.png");
        embedded_asset!(app, "src\\", "icons\\search.png");
        embedded_asset!(app, "src\\", "icons\\select.png");
        embedded_asset!(app, "src\\", "icons\\selected.png");
        embedded_asset!(app, "src\\", "icons\\to_bottom.png");
        embedded_asset!(app, "src\\", "icons\\to_top.png");
        embedded_asset!(app, "src\\", "icons\\trash.png");
        embedded_asset!(app, "src\\", "icons\\up.png");
    }
}

impl Plugin for StandardUiLayout {
    fn build(&self, app: &mut App) {
        add_widgets_icons(app);
        app
            .init_resource::<Icons>()
            .add_plugins((
                StandardPropertiesPanelPlugin::default(),
                ConsoleWidgetPlugin::default(),
                MenuBarPlugin::default(),
            ))
            .init_resource::<LightDisplay>()
            .init_resource::<AssetGalleryStatus>()
            .init_resource::<OccupancyDisplay>()
            .init_resource::<DiagnosticWindowState>()
            .init_resource::<PendingDrawing>()
            .init_resource::<PendingModel>()
            .init_resource::<SearchForFiducial>()
            .add_plugins(MenuPluginManager)
            .init_resource::<SearchForTexture>()
            .init_resource::<GroupViewModes>()
            .add_systems(Startup, init_ui_style)
            .add_systems(
                Update,
                ex_site_ui_layout.run_if(in_state(AppState::SiteEditor)),
            )
            .add_systems(
                Update,
                workcell_ui_layout.run_if(in_state(AppState::WorkcellEditor)),
            )
            .add_systems(
                Update,
                site_drawing_ui_layout.run_if(in_state(AppState::SiteDrawingEditor)),
            )
            .add_systems(
                Update,
                site_visualizer_ui_layout.run_if(in_state(AppState::SiteVisualizer)),
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

#[derive(Default)]
pub struct StandardPropertiesPanelPlugin {

}

impl Plugin for StandardPropertiesPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PropertiesPanelPlugin::new(PanelSide::Right),
            ViewLevelsPlugin::default(),
            ViewNavGraphsPlugin::default(),
            ViewLayersPlugin::default(),
            StandardInspectorPlugin::default(),
            CreationPlugin::default(),
            ViewGroupsPlugin::default(),
            ViewLightsPlugin::default(),
        ));
    }
}

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

pub trait ExecuteWidget<Input, Output> {
    fn show(&mut self, input: Input, ui: &mut Ui, world: &mut World) -> Output;
}

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

pub type ShowResult<T=()> = Result<T, ShowError>;

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

pub trait TryShowWidgetWorld {
    fn try_show(
        &mut self,
        entity: Entity,
        ui: &mut Ui,
    ) -> ShowResult<()> {
        self.try_show_out(entity, (), ui)
    }

    fn try_show_in<Input>(
        &mut self,
        entity: Entity,
        input: Input,
        ui: &mut Ui,
    ) -> ShowResult<()>
    where
        Input: 'static + Send + Sync,
    {
        self.try_show_out(entity, input, ui)
    }

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

pub trait TryShowWidgetEntity {
    fn try_show(
        &mut self,
        ui: &mut Ui,
    ) -> ShowResult<()> {
        self.try_show_out((), ui)
    }

    fn try_show_in<Input>(
        &mut self,
        input: Input,
        ui: &mut Ui,
    ) -> ShowResult<()>
    where
        Input: 'static + Send + Sync,
    {
        self.try_show_out(input, ui)
    }

    fn try_show_out<Output, Input>(
        &mut self,
        input: Input,
        ui: &mut Ui,
    ) -> ShowResult<Output>
    where
        Input: 'static + Send + Sync,
        Output: 'static + Send + Sync;
}

impl<'w> TryShowWidgetEntity for EntityWorldMut<'w> {
    fn try_show_out<Output, Input>(
        &mut self,
        input: Input,
        ui: &mut Ui,
    ) -> ShowResult<Output>
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

        let output = self.world_scope(|world| {
            inner.show(input, ui, world)
        });

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
pub trait ShareableWidget { }

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
        W: ShareableWidget + WidgetSystem<Input, Output> + 'static
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

#[derive(Clone, Copy, Debug)]
pub struct Panel {
    pub id: Entity,
    pub side: PanelSide,
}

pub struct Tile {
    pub id: Entity,
    pub panel: PanelSide,
}

pub mod prelude {
    pub use super::{
        Widget, WidgetSystem, TryShowWidgetWorld, TryShowWidgetEntity,
        ShowResult, ShowError, Tile, ShowSharedWidget, ShareableWidget,
        Panel, PanelSide, PropertiesPanel, PanelWidget,
    };
    pub use bevy::ecs::system::{SystemState, SystemParam};
}

/// To create a panel widget (a widget that renders itself directly to one of
/// the egui side panels), add this component to an entity.
#[derive(Component)]
pub struct PanelWidget {
    inner: Option<BoxedSystem<Entity>>,
}

impl PanelWidget {
    pub fn new<M, S: IntoSystem<Entity, (), M>>(
        system: S,
        world: &mut World,
    ) -> Self {
        let mut system = Box::new(IntoSystem::into_system(system));
        system.initialize(world);
        Self { inner: Some(system) }
    }
}

fn ex_site_ui_layout(
    world: &mut World,
    panel_widgets: &mut QueryState<(Entity, &mut PanelWidget)>,
    egui_context_state: &mut SystemState<EguiContexts>,
) {
    let mut panels: SmallVec<[_; 16]> = panel_widgets
        .iter_mut(world)
        .map(|(entity, mut widget)| {
            (entity, widget.inner.take().expect("Inner system of PanelWidget is missing"))
        })
        .collect();

    for (e, inner) in &mut panels {
        inner.run(*e, world);
        inner.apply_deferred(world);
    }

    for (e, inner) in panels {
        if let Some(mut widget) = world.get_mut::<PanelWidget>(e) {
            let _ = widget.inner.insert(inner);
        }
    }

    let mut egui_context = egui_context_state.get_mut(world);
    let ctx = egui_context.ctx_mut();
    let ui_has_focus = ctx.wants_pointer_input()
        || ctx.wants_keyboard_input()
        || ctx.is_pointer_over_area();

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

#[derive(Clone, Copy, Debug, Component)]
pub enum PanelSide {
    Top,
    Bottom,
    Left,
    Right,
}

pub enum EguiPanel {
    Vertical(egui::SidePanel),
    Horizontal(egui::TopBottomPanel),
}

impl EguiPanel {
    pub fn map_vertical(
        self,
        f: impl FnOnce(egui::SidePanel) -> egui::SidePanel,
    ) -> Self {
        match self {
            Self::Vertical(panel) => Self::Vertical(f(panel)),
            other => other,
        }
    }

    pub fn map_horizontal(
        self,
        f: impl FnOnce(egui::TopBottomPanel) -> egui::TopBottomPanel,
    ) -> Self {
        match self {
            Self::Horizontal(panel) => Self::Horizontal(f(panel)),
            other => other,
        }
    }

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
    pub fn align<R>(
        self,
        ui: &mut Ui,
        f: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<R> {
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

    pub fn get_panel(self) -> EguiPanel {
        match self {
            Self::Left => EguiPanel::Vertical(egui::SidePanel::left("left_panel")),
            Self::Right => EguiPanel::Vertical(egui::SidePanel::right("right_panel")),
            Self::Top => EguiPanel::Horizontal(egui::TopBottomPanel::top("top_panel")),
            Self::Bottom => EguiPanel::Horizontal(egui::TopBottomPanel::bottom("bottom_panel")),
        }
    }
}

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
        app.world.insert_resource(PropertiesPanel { side: self.side, id });
    }
}

fn tile_panel_widget(
    In(panel): In<Entity>,
    world: &mut World,
    egui_contexts: &mut SystemState<EguiContexts>,
) {
    let children: Option<SmallVec<[Entity; 16]>> = world
        .get::<Children>(panel)
        .map(|children| children.iter().copied().collect());

    let Some(children) = children else {
        return;
    };
    if children.is_empty() {
        // Do not even begin to create a panel if there are no children to render
        return;
    }

    let Some(side) = world.get::<PanelSide>(panel) else {
        error!("Side component missing for tile_panel_widget {panel:?}");
        return;
    };

    let side = *side;
    let ctx = egui_contexts.get_mut(world).ctx_mut().clone();
    side.get_panel()
        .map_vertical(|panel| {
            // TODO(@mxgrey): Make this configurable via a component
            panel
            .resizable(true)
            .default_width(300.0)
        })
        .show(&ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for child in children {
                        let tile = Tile { id: child, panel: side };
                        if let Err(err) = world.try_show_in(child, tile, ui) {
                            error!(
                                "Could not render child widget {child:?} in \
                                tile panel {panel:?} on side {side:?}: {err:?}"
                            );
                        }
                    }
                });
        });
}

#[derive(SystemParam)]
pub struct ChangeEvents<'w> {
    pub lane_motion: EventWriter<'w, Change<Motion>>,
    pub lane_reverse: EventWriter<'w, Change<ReverseLane>>,
    pub name: EventWriter<'w, Change<NameInSite>>,
    pub pose: EventWriter<'w, Change<Pose>>,
    pub door: EventWriter<'w, Change<DoorType>>,
    pub lift_cabin: EventWriter<'w, Change<LiftCabin<Entity>>>,
    pub asset_source: EventWriter<'w, Change<AssetSource>>,
    pub pixels_per_meter: EventWriter<'w, Change<PixelsPerMeter>>,
    pub physical_camera_properties: EventWriter<'w, Change<PhysicalCameraProperties>>,
    pub light: EventWriter<'w, Change<LightKind>>,
    pub level_elevation: EventWriter<'w, Change<LevelElevation>>,
    pub color: EventWriter<'w, Change<DisplayColor>>,
    pub visibility: EventWriter<'w, Change<Visibility>>,
    pub associated_graphs: EventWriter<'w, Change<AssociatedGraphs<Entity>>>,
    pub location_tags: EventWriter<'w, Change<LocationTags>>,
    pub affiliation: EventWriter<'w, Change<Affiliation<Entity>>>,
    pub search_for_fiducial: ResMut<'w, SearchForFiducial>,
    pub search_for_texture: ResMut<'w, SearchForTexture>,
    pub distance: EventWriter<'w, Change<Distance>>,
    pub texture: EventWriter<'w, Change<Texture>>,
    pub joint_properties: EventWriter<'w, Change<JointProperties>>,
    pub merge_groups: EventWriter<'w, MergeGroups>,
    pub filtered_issues: EventWriter<'w, Change<FilteredIssues<Entity>>>,
    pub filtered_issue_kinds: EventWriter<'w, Change<FilteredIssueKinds>>,
}

#[derive(SystemParam)]
pub struct WorkcellChangeEvents<'w> {
    pub mesh_constraints: EventWriter<'w, Change<MeshConstraint<Entity>>>,
    pub name_in_workcell: EventWriter<'w, Change<NameInWorkcell>>,
    pub workcell_name: EventWriter<'w, Change<NameOfWorkcell>>,
    pub scale: EventWriter<'w, Change<Scale>>,
    pub primitive_shapes: EventWriter<'w, Change<PrimitiveShape>>,
}

#[derive(SystemParam)]
pub struct FileEvents<'w> {
    pub save: EventWriter<'w, SaveWorkspace>,
    pub load_workspace: EventWriter<'w, LoadWorkspace>,
    pub new_workspace: EventWriter<'w, CreateNewWorkspace>,
    pub diagnostic_window: ResMut<'w, DiagnosticWindowState>,
}

#[derive(SystemParam)]
pub struct PanelResources<'w> {
    pub level: ResMut<'w, LevelDisplay>,
    pub nav_graph: ResMut<'w, NavGraphDisplay>,
    pub light: ResMut<'w, LightDisplay>,
    pub occupancy: ResMut<'w, OccupancyDisplay>,
    pub log_history: ResMut<'w, LogHistory>,
    pub pending_model: ResMut<'w, PendingModel>,
    pub pending_drawings: ResMut<'w, PendingDrawing>,
}

#[derive(SystemParam)]
pub struct Requests<'w> {
    pub hover: ResMut<'w, Events<Hover>>,
    pub select: ResMut<'w, Events<Select>>,
    pub move_to: EventWriter<'w, MoveTo>,
    pub current_level: ResMut<'w, CurrentLevel>,
    pub current_workspace: ResMut<'w, CurrentWorkspace>,
    pub change_mode: ResMut<'w, Events<ChangeMode>>,
    pub delete: EventWriter<'w, Delete>,
    pub toggle_door_levels: EventWriter<'w, ToggleLiftDoorAvailability>,
    pub toggle_headlights: ResMut<'w, HeadlightToggle>,
    pub toggle_physical_lights: ResMut<'w, PhysicalLightToggle>,
    pub spawn_preview: EventWriter<'w, SpawnPreview>,
    pub export_lights: EventWriter<'w, ExportLights>,
    pub save_nav_graphs: EventWriter<'w, SaveNavGraphs>,
    pub calculate_grid: EventWriter<'w, CalculateGrid>,
    pub consider_tag: EventWriter<'w, ConsiderLocationTag>,
    pub consider_graph: EventWriter<'w, ConsiderAssociatedGraph>,
    pub align_site: EventWriter<'w, AlignSiteDrawings>,
    pub validate_workspace: EventWriter<'w, ValidateWorkspace>,
    pub create_joint: EventWriter<'w, CreateJoint>,
}

#[derive(SystemParam)]
pub struct LayerEvents<'w> {
    pub floors: EventWriter<'w, ChangeRank<FloorMarker>>,
    pub drawings: EventWriter<'w, ChangeRank<DrawingMarker>>,
    pub nav_graphs: EventWriter<'w, ChangeRank<NavGraphMarker>>,
    pub layer_vis: EventWriter<'w, Change<LayerVisibility>>,
    pub preferred_alpha: EventWriter<'w, Change<PreferredSemiTransparency>>,
    pub global_floor_vis: EventWriter<'w, Change<GlobalFloorVisibility>>,
    pub global_drawing_vis: EventWriter<'w, Change<GlobalDrawingVisibility>>,
    pub begin_edit_drawing: EventWriter<'w, BeginEditDrawing>,
    pub finish_edit_drawing: EventWriter<'w, FinishEditDrawing>,
    pub icons: Res<'w, Icons>,
}

/// We collect all the events into its own SystemParam because we are not
/// allowed to receive more than one EventWriter of a given type per system call
/// (for borrow-checker reasons). Bundling them all up into an AppEvents
/// parameter at least makes the EventWriters easy to pass around.
#[derive(SystemParam)]
pub struct AppEvents<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub change: ChangeEvents<'w>,
    pub workcell_change: WorkcellChangeEvents<'w>,
    pub display: PanelResources<'w>,
    pub request: Requests<'w>,
    pub file_events: FileEvents<'w>,
    pub layers: LayerEvents<'w>,
    pub new_model: NewModelParams<'w>,
    pub app_state: Res<'w, State<AppState>>,
    pub next_app_state: ResMut<'w, NextState<AppState>>,
}

fn site_ui_layout(
    mut egui_context: EguiContexts,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
    open_sites: Query<Entity, With<NameOfSite>>,
    inspector_params: InspectorParams,
    create_params: CreateParams,
    levels: LevelParams,
    lights: LightParams,
    nav_graphs: NavGraphParams,
    diagnostic_params: DiagnosticParams,
    layers: LayersParams,
    mut groups: GroupParams,
    mut events: AppEvents,
    file_menu: Res<FileMenu>,
    children: Query<&Children>,
    top_level_components: Query<(), Without<Parent>>,
) {
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(300.0)
        .show(egui_context.ctx_mut(), |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        CollapsingHeader::new("Levels")
                            .default_open(true)
                            .show(ui, |ui| {
                                ViewLevels::new(&levels, &mut events)
                                    .for_editing_visibility()
                                    .show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Navigation Graphs")
                            .default_open(true)
                            .show(ui, |ui| {
                                ViewNavGraphs::new(&nav_graphs, &mut events).show(ui, &open_sites);
                            });
                        ui.separator();
                        // TODO(MXG): Consider combining Nav Graphs and Layers
                        CollapsingHeader::new("Layers")
                            .default_open(false)
                            .show(ui, |ui| {
                                ViewLayers::new(&layers, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Inspect")
                            .default_open(true)
                            .show(ui, |ui| {
                                InspectorWidget::new(&inspector_params, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Create")
                            .default_open(false)
                            .show(ui, |ui| {
                                CreateWidget::new(&create_params, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Groups")
                            .default_open(false)
                            .show(ui, |ui| {
                                ViewGroups::new(&mut groups, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Lights")
                            .default_open(false)
                            .show(ui, |ui| {
                                ViewLights::new(&lights, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Occupancy")
                            .default_open(false)
                            .show(ui, |ui| {
                                ViewOccupancy::new(&mut events).show(ui);
                            });
                        if ui.add(Button::new("Building preview")).clicked() {
                            events.next_app_state.set(AppState::SiteVisualizer);
                        }
                    });
                });
        });

    egui::TopBottomPanel::bottom("log_console")
        .resizable(true)
        .min_height(30.)
        .max_height(300.)
        .show(egui_context.ctx_mut(), |ui| {
            ui.add_space(10.0);
            ConsoleWidget::new(&mut events).show(ui);
        });

    if events.file_events.diagnostic_window.show {
        egui::SidePanel::left("diagnostic_window")
            .resizable(true)
            .exact_width(320.0)
            .show(egui_context.ctx_mut(), |ui| {
                DiagnosticWindow::new(&mut events, &diagnostic_params).show(ui);
            });
    }
    if events.new_model.asset_gallery_status.show {
        egui::SidePanel::left("asset_gallery")
            .resizable(true)
            .exact_width(320.0)
            .show(egui_context.ctx_mut(), |ui| {
                NewModel::new(&mut events).show(ui);
            });
    }

    let egui_context = egui_context.ctx_mut();
    let ui_has_focus = egui_context.wants_pointer_input()
        || egui_context.wants_keyboard_input()
        || egui_context.is_pointer_over_area();

    if let Some(picking_blocker) = &mut picking_blocker {
        picking_blocker.ui = ui_has_focus;
    }

    if ui_has_focus {
        // If the UI has focus and there were no hover events emitted by the UI,
        // then we should emit a None hover event
        if events.request.hover.is_empty() {
            events.request.hover.send(Hover(None));
        }
    }
}

fn site_drawing_ui_layout(
    mut egui_context: EguiContexts,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
    inspector_params: InspectorParams,
    create_params: CreateParams,
    mut events: AppEvents,
    file_menu: Res<FileMenu>,
    children: Query<&Children>,
    top_level_components: Query<(), Without<Parent>>,
) {
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .show(egui_context.ctx_mut(), |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        CollapsingHeader::new("Inspect")
                            .default_open(true)
                            .show(ui, |ui| {
                                InspectorWidget::new(&inspector_params, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Create")
                            .default_open(true)
                            .show(ui, |ui| {
                                CreateWidget::new(&create_params, &mut events).show(ui);
                            });
                        ui.separator();
                        if ui
                            .add(Button::image_and_text(
                                events.layers.icons.exit.egui(),
                                "Return to site editor",
                            ))
                            .clicked()
                        {
                            events
                                .layers
                                .finish_edit_drawing
                                .send(FinishEditDrawing(None));
                        }
                    });
                });
        });

    egui::TopBottomPanel::bottom("log_console")
        .resizable(true)
        .min_height(30.)
        .max_height(300.)
        .show(egui_context.ctx_mut(), |ui| {
            ui.add_space(10.0);
            ConsoleWidget::new(&mut events).show(ui);
        });

    let egui_context = egui_context.ctx_mut();
    let ui_has_focus = egui_context.wants_pointer_input()
        || egui_context.wants_keyboard_input()
        || egui_context.is_pointer_over_area();

    if let Some(picking_blocker) = &mut picking_blocker {
        picking_blocker.ui = ui_has_focus;
    }

    if ui_has_focus {
        // If the UI has focus and there were no hover events emitted by the UI,
        // then we should emit a None hover event
        if events.request.hover.is_empty() {
            events.request.hover.send(Hover(None));
        }
    }
}

fn site_visualizer_ui_layout(
    mut egui_context: EguiContexts,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
    mut events: AppEvents,
    levels: LevelParams,
    file_menu: Res<FileMenu>,
    top_level_components: Query<(), Without<Parent>>,
    children: Query<&Children>,
) {
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(300.0)
        .show(egui_context.ctx_mut(), |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        CollapsingHeader::new("Levels")
                            .default_open(true)
                            .show(ui, |ui| {
                                ViewLevels::new(&levels, &mut events).show(ui);
                            });
                        ui.separator();
                        if ui.add(Button::image_and_text(
                            events.layers.icons.alignment.egui(),
                            "Align Drawings",
                        ))
                            .on_hover_text("Align all drawings in the site based on their fiducials and measurements")
                            .clicked()
                        {
                            if let Some(site) = events.request.current_workspace.root {
                                events.request.align_site.send(AlignSiteDrawings(site));
                            }
                        }
                        if ui.add(Button::image_and_text(
                            events.layers.icons.exit.egui(),
                            "Return to site editor"
                        )).clicked() {
                            events.next_app_state.set(AppState::SiteEditor);
                        }
                    });
                });
        });

    egui::TopBottomPanel::bottom("log_console")
        .resizable(true)
        .min_height(30.)
        .max_height(300.)
        .show(egui_context.ctx_mut(), |ui| {
            ui.add_space(10.0);
            ConsoleWidget::new(&mut events).show(ui);
        });

    let egui_context = egui_context.ctx_mut();
    let ui_has_focus = egui_context.wants_pointer_input()
        || egui_context.wants_keyboard_input()
        || egui_context.is_pointer_over_area();

    if let Some(picking_blocker) = &mut picking_blocker {
        picking_blocker.ui = ui_has_focus;
    }

    if ui_has_focus {
        // If the UI has focus and there were no hover events emitted by the UI,
        // then we should emit a None hover event
        if events.request.hover.is_empty() {
            events.request.hover.send(Hover(None));
        }
    }
}

fn workcell_ui_layout(
    mut egui_context: EguiContexts,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
    inspector_params: InspectorParams,
    create_params: CreateParams,
    mut events: AppEvents,
    file_menu: Res<FileMenu>,
    top_level_components: Query<(), Without<Parent>>,
    children: Query<&Children>,
) {
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .show(egui_context.ctx_mut(), |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        CollapsingHeader::new("Inspect")
                            .default_open(true)
                            .show(ui, |ui| {
                                InspectorWidget::new(&inspector_params, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Create")
                            .default_open(true)
                            .show(ui, |ui| {
                                CreateWidget::new(&create_params, &mut events).show(ui);
                            });
                        ui.separator();
                    });
                });
        });

    egui::TopBottomPanel::bottom("log_console")
        .resizable(true)
        .min_height(30.)
        .max_height(300.)
        .show(egui_context.ctx_mut(), |ui| {
            ui.add_space(10.0);
            ConsoleWidget::new(&mut events).show(ui);
        });

    if events.new_model.asset_gallery_status.show {
        egui::SidePanel::left("asset_gallery")
            .resizable(true)
            .exact_width(320.0)
            .show(egui_context.ctx_mut(), |ui| {
                NewModel::new(&mut events).show(ui);
            });
    }

    let egui_context = egui_context.ctx_mut();
    let ui_has_focus = egui_context.wants_pointer_input()
        || egui_context.wants_keyboard_input()
        || egui_context.is_pointer_over_area();

    if let Some(picking_blocker) = &mut picking_blocker {
        picking_blocker.ui = ui_has_focus;
    }

    if ui_has_focus {
        // If the UI has focus and there were no hover events emitted by the UI,
        // then we should emit a None hover event
        if events.request.hover.is_empty() {
            events.request.hover.send(Hover(None));
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
