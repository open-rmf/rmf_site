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

use anyhow::{Error as Anyhow, anyhow};
use bevy_app::prelude::*;
use bevy_ecs::system::{ScheduleSystem, SystemParam};
use bevy_impulse::*;
use std::error::Error;
use std::fmt::Debug;

pub mod components;
pub use components::*;

pub mod plugins;
pub use plugins::*;

pub mod resources;
pub use resources::*;

pub mod events;
pub use events::*;

mod systems;
pub(crate) use systems::*;

#[derive(Debug, Clone, Copy)]
pub struct SelectionCandidate {
    /// The entity that's being requested as a selection
    pub candidate: Entity,
    /// The entity was created specifically to be selected, so if it ends up
    /// going unused by the workflow then it should be despawned.
    pub provisional: bool,
}

impl SelectionCandidate {
    pub fn new(candidate: Entity) -> SelectionCandidate {
        SelectionCandidate {
            candidate,
            provisional: false,
        }
    }

    pub fn provisional(candidate: Entity) -> SelectionCandidate {
        SelectionCandidate {
            candidate,
            provisional: true,
        }
    }
}

/// This allows an [`App`] to spawn a service that can stream Hover and
/// Select events that are managed by a filter. This can only be used with
/// [`App`] because some of the internal services are continuous, so they need
/// to be added to the schedule.
pub trait SpawnSelectionServiceExt {
    fn spawn_selection_service<F: SystemParam + 'static>(
        &mut self,
    ) -> Service<(), (), (Hover, Select)>
    where
        for<'w, 's> F::Item<'w, 's>: SelectionFilter;
}

impl SpawnSelectionServiceExt for App {
    fn spawn_selection_service<F: SystemParam + 'static>(
        &mut self,
    ) -> Service<(), (), (Hover, Select)>
    where
        for<'w, 's> F::Item<'w, 's>: SelectionFilter,
    {
        let picking_service = self.spawn_continuous_service(
            Update,
            picking_service::<F>.configure(|config: ScheduleConfigs<ScheduleSystem>| {
                config.in_set(SelectionServiceStages::Pick)
            }),
        );

        let hover_service = self.spawn_continuous_service(
            Update,
            hover_service::<F>.configure(|config: ScheduleConfigs<ScheduleSystem>| {
                config.in_set(SelectionServiceStages::Hover)
            }),
        );

        let select_service = self.spawn_continuous_service(
            Update,
            select_service::<F>.configure(|config: ScheduleConfigs<ScheduleSystem>| {
                config.in_set(SelectionServiceStages::Select)
            }),
        );

        self.world_mut()
            .spawn_workflow::<_, _, (Hover, Select), _>(|scope, builder| {
                let hover = builder.create_node(hover_service);
                builder.connect(hover.streams, scope.streams.0);
                builder.connect(hover.output, scope.terminate);

                let select = builder.create_node(select_service);
                builder.connect(select.streams, scope.streams.1);
                builder.connect(select.output, scope.terminate);

                // Activate all the services at the start
                scope.input.chain(builder).fork_clone((
                    |chain: Chain<_>| {
                        chain
                            .then(refresh_picked.into_blocking_callback())
                            .then(picking_service)
                            .connect(scope.terminate)
                    },
                    |chain: Chain<_>| chain.connect(hover.input),
                    |chain: Chain<_>| chain.connect(select.input),
                ));

                // This is just a dummy buffer to let us have a cleanup workflow
                let buffer = builder.create_buffer::<()>(BufferSettings::keep_all());
                builder.on_cleanup(buffer, |scope, builder| {
                    scope
                        .input
                        .chain(builder)
                        .trigger()
                        .then(clear_hover_select.into_blocking_callback())
                        .connect(scope.terminate);
                });
            })
    }
}

// TODO(@mxgrey): Remove flush stages when we move to bevy 0.13 which can infer
// when to flush
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SelectionServiceStages {
    Pick,
    PickFlush,
    Hover,
    HoverFlush,
    Select,
    SelectFlush,
}

#[derive(SystemParam)]
struct InspectorFilter<'w, 's> {
    selectables: Query<
        'w,
        's,
        &'static Selectable,
        (
            Without<Preview>,
            // Without<Pending>
        ),
    >,
}

impl<'w, 's> SelectionFilter for InspectorFilter<'w, 's> {
    fn filter_pick(&mut self, select: Entity) -> Option<Entity> {
        self.selectables
            .get(select)
            .ok()
            .map(|selectable| selectable.element)
    }
    fn filter_select(&mut self, target: Entity) -> Option<Entity> {
        Some(target)
    }
    fn on_click(&mut self, hovered: Hover) -> Option<Select> {
        Some(Select::new(hovered.0))
    }
}
pub type SelectionNodeResult = Result<(), Option<Anyhow>>;

pub trait CommonNodeErrors {
    type Value;
    fn or_broken_buffer(self) -> Result<Self::Value, Option<Anyhow>>;
    fn or_broken_state(self) -> Result<Self::Value, Option<Anyhow>>;
    fn or_broken_query(self) -> Result<Self::Value, Option<Anyhow>>;
}

impl<T, E: Error> CommonNodeErrors for Result<T, E> {
    type Value = T;
    fn or_broken_buffer(self) -> Result<Self::Value, Option<Anyhow>> {
        self.map_err(|err| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            Some(anyhow!(
                "The buffer in the workflow is broken: {err}. Backtrace:\n{backtrace}"
            ))
        })
    }

    fn or_broken_state(self) -> Result<Self::Value, Option<Anyhow>> {
        self.map_err(|err| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            Some(anyhow!(
                "The state is missing from the workflow buffer: {err}. Backtrace:\n{backtrace}"
            ))
        })
    }

    fn or_broken_query(self) -> Result<Self::Value, Option<Anyhow>> {
        self.map_err(|err| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            Some(anyhow!(
                "A query that should have worked failed: {err}. Backtrace:\n{backtrace}"
            ))
        })
    }
}

impl<T> CommonNodeErrors for Option<T> {
    type Value = T;
    fn or_broken_buffer(self) -> Result<Self::Value, Option<Anyhow>> {
        self.ok_or_else(|| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            Some(anyhow!(
                "The buffer in the workflow has been despawned. Backtrace:\n{backtrace}"
            ))
        })
    }

    fn or_broken_state(self) -> Result<Self::Value, Option<Anyhow>> {
        self.ok_or_else(|| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            Some(anyhow!(
                "The state is missing from the workflow buffer. Backtrace:\n{backtrace}"
            ))
        })
    }

    fn or_broken_query(self) -> Result<Self::Value, Option<Anyhow>> {
        self.ok_or_else(|| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            Some(anyhow!(
                "A query that should have worked failed. Backtrace:\n{backtrace}"
            ))
        })
    }
}

pub trait SelectionFilter: SystemParam {
    /// If the target entity is being picked, give back the entity that should
    /// be recognized as the hovered entity. Return [`None`] to behave as if
    /// nothing is being hovered.
    fn filter_pick(&mut self, target: Entity) -> Option<Entity>;

    /// If the target entity is being hovered or selected, give back the entity
    /// that should be recognized as the hovered or selected entity. Return
    /// [`None`] to deselect anything that might currently be selected.
    fn filter_select(&mut self, target: Entity) -> Option<Entity>;

    /// For the given hover state, indicate what kind of [`Select`] signal should
    /// be sent when the user clicks.
    fn on_click(&mut self, hovered: Hover) -> Option<Select>;
}
