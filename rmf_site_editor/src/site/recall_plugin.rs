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

use crate::site::SiteUpdateSet;
use bevy::{ecs::component::Mutable, prelude::*};
use rmf_site_format::Recall;

/// The set in which all [`RecallPlugin`]s run.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct UpdateRecallSet;

#[derive(Default)]
pub struct RecallPlugin<T: Recall + Component<Mutability = Mutable> + Default>
where
    T::Source: Component,
{
    _ignore: std::marker::PhantomData<T>,
}

impl<T: Recall + Component<Mutability = Mutable> + Default> Plugin for RecallPlugin<T>
where
    T::Source: Component,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                add_recaller::<T>.after(SiteUpdateSet::ProcessChanges),
                update_recaller::<T>.after(SiteUpdateSet::ProcessChanges),
            )
                .in_set(UpdateRecallSet),
        );
    }
}

fn add_recaller<Recaller: Recall + Component<Mutability = Mutable> + Default>(
    mut commands: Commands,
    new_sources: Query<(Entity, &Recaller::Source), (Added<Recaller::Source>, Without<Recaller>)>,
) where
    Recaller::Source: Component,
{
    for (e, source) in &new_sources {
        let mut recaller = Recaller::default();
        recaller.remember(source);
        let _ = commands.get_entity(e).map(|mut e_mut| {
            e_mut.insert(recaller);
        });
    }
}

fn update_recaller<Recaller: Recall + Component<Mutability = Mutable> + Default>(
    mut changed_sources: Query<(&Recaller::Source, &mut Recaller), Changed<Recaller::Source>>,
) where
    Recaller::Source: Component,
{
    for (source, mut recaller) in &mut changed_sources {
        recaller.remember(source);
    }
}
