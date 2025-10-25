use std::marker::PhantomData;

use bevy_ecs::system::ScheduleSystem;
use bytemuck::TransparentWrapper;
use crossflow::{
    AddContinuousServicesExt, AddServicesExt, IntoBlockingCallback, QuickContinuousServiceBuild,
    RequestExt, RunCommandsOnWorldExt, ScheduleConfigs, Service, SpawnWorkflowExt, flush_execution,
};
use std::fmt::Debug;

use bevy_app::prelude::*;
use rmf_site_camera::CameraControlsBlocker;

use crate::{picking::plugins::PickingRMFPlugin, select::InspectorFilter, *};

type SelectionService = Service<(), ()>;

/// Plugin for default selection behaviour within rmf_site.
///
/// `T` is the default service for selection behaviour within this plugin.
///
/// !!! Ensure `T`'s assocaited plugin is initialized before this one or this plugin will crash !!!
pub struct SelectionPlugin<T>
where
    T: Debug + Send + Sync + Resource + TransparentWrapper<SelectionService> + 'static,
{
    pub _a: PhantomData<T>,
}

impl<T> Default for SelectionPlugin<T>
where
    T: Debug + Send + Sync + Resource + TransparentWrapper<SelectionService> + 'static,
{
    fn default() -> Self {
        Self {
            _a: Default::default(),
        }
    }
}

impl<T> Plugin for SelectionPlugin<T>
where
    T: Debug + Send + Sync + Resource + TransparentWrapper<SelectionService> + 'static,
{
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                SelectionServiceStages::Pick,
                SelectionServiceStages::PickFlush,
                SelectionServiceStages::Hover,
                SelectionServiceStages::HoverFlush,
                SelectionServiceStages::Select,
                SelectionServiceStages::SelectFlush,
            )
                .chain(),
        )
        .add_plugins(PickingRMFPlugin)
        .init_resource::<SelectionBlockers>()
        .init_resource::<Selection>()
        .init_resource::<Hovering>()
        .add_event::<DoubleClickSelect>()
        .add_event::<Select>()
        .add_event::<Hover>()
        .add_event::<RunSelector>()
        .add_plugins(CameraControlsBlocker::<UiFocused>::default())
        .init_resource::<UiFocused>()
        .init_resource::<IteractionMaskHovered>()
        .add_systems(
            Update,
            (
                (ApplyDeferred, flush_execution())
                    .chain()
                    .in_set(SelectionServiceStages::PickFlush),
                (ApplyDeferred, flush_execution())
                    .chain()
                    .in_set(SelectionServiceStages::HoverFlush),
                (ApplyDeferred, flush_execution())
                    .chain()
                    .in_set(SelectionServiceStages::SelectFlush),
            ),
        )
        .add_systems(
            Update,
            (make_selectable_entities_pickable, send_double_click_event),
        );
        let default_selection_service = app.world().get_resource::<T>();
        let Some(default_selection_service) = default_selection_service else {
            panic!(
                "{:#?}'s plugin, must be initialized before this plugin",
                default_selection_service
            );
        };
        let default_selection_service =
            TransparentWrapper::peel_ref(default_selection_service).clone();
        let new_selector_service = app.spawn_event_streaming_service::<RunSelector>(Update);
        let selection_workflow = app.world_mut().spawn_io_workflow(build_selection_workflow(
            default_selection_service,
            new_selector_service,
        ));

        // Get the selection workflow running
        app.world_mut().command(|commands| {
            commands.request((), selection_workflow).detach();
        });
    }
}

/// Plugin for selection behaviour for open_rmf site's inspector.
#[derive(Default)]
pub struct InspectorServicePlugin;

impl Plugin for InspectorServicePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(KeyboardServicePlugin);
        let inspector_select_service = app.spawn_selection_service::<InspectorFilter>();
        let inspector_cursor_transform = app.spawn_continuous_service(
            Update,
            inspector_cursor_transform.configure(|config: ScheduleConfigs<ScheduleSystem>| {
                config.in_set(SelectionServiceStages::Pick)
            }),
        );
        let selection_update = app.spawn_service(selection_update);
        let keyboard_just_pressed = app
            .world()
            .resource::<KeyboardServices>()
            .keyboard_just_pressed;

        let inspector_service = app.world_mut().spawn_workflow(|scope, builder| {
            let fork_input = scope.input.fork_clone(builder);
            fork_input
                .clone_chain(builder)
                .then(inspector_cursor_transform)
                .unused();
            fork_input
                .clone_chain(builder)
                .then_node(keyboard_just_pressed)
                .streams
                .chain(builder)
                .then(deselect_on_esc.into_blocking_callback())
                .unused();
            let selection = fork_input
                .clone_chain(builder)
                .then_node(inspector_select_service);
            selection
                .streams
                .select
                .chain(builder)
                .then(selection_update)
                .unused();
            builder.connect(selection.output, scope.terminate);
        });

        app.world_mut().insert_resource(InspectorServiceConfigs {
            inspector_select_service,
            inspector_cursor_transform,
            selection_update,
        });
        app.world_mut()
            .insert_resource(InspectorService(inspector_service));
    }
}
