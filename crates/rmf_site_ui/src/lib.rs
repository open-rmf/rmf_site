pub mod menu_bar;
use bevy_ecs::{prelude::*, system::{SystemParam, SystemState}};
use bevy_egui::egui::Ui;
pub use menu_bar::*;

pub mod header_panel;
pub use header_panel::*;

pub mod panel_of_tiles;
pub use panel_of_tiles::*;

pub mod panel;
pub use panel::*;

/// Implement this on a [`SystemParam`] struct to make it a widget that can be
/// plugged into the site editor UI.
///
/// See documentation of [`PropertiesTilePlugin`] or [`InspectionPlugin`] to see
/// examples of using this.
pub trait WidgetSystem<Input = (), Output = ()>: SystemParam {
    fn show(input: Input, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> Output;
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
        let Ok(mut entity_mut) = self.get_entity_mut(entity) else {
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