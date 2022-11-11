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

use crate::site::{SiteState, SiteUpdateLabel};
use bevy::prelude::*;
use rmf_site_format::Recall;

#[derive(Default)]
pub struct RecallPlugin<T: Recall + Component + Default>
where
    T::Source: Component,
{
    _ignore: std::marker::PhantomData<T>,
}

impl<T: Recall + Component + Default> Plugin for RecallPlugin<T>
where
    T::Source: Component,
{
    fn build(&self, app: &mut App) {
        app.add_system_set_to_stage(
            CoreStage::PreUpdate,
            SystemSet::on_update(SiteState::Display)
                .after(SiteUpdateLabel::ProcessChanges)
                .with_system(add_recaller::<T>)
                .with_system(update_recaller::<T>),
        );
    }
}

fn add_recaller<Recaller: Recall + Component + Default>(
    mut commands: Commands,
    new_sources: Query<(Entity, &Recaller::Source), (Added<Recaller::Source>, Without<Recaller>)>,
) where
    Recaller::Source: Component,
{
    for (e, source) in &new_sources {
        let mut recaller = Recaller::default();
        recaller.remember(source);
        commands.entity(e).insert(recaller);
    }
}

fn update_recaller<Recaller: Recall + Component + Default>(
    mut changed_sources: Query<(&Recaller::Source, &mut Recaller), Changed<Recaller::Source>>,
) where
    Recaller::Source: Component,
{
    for (source, mut recaller) in &mut changed_sources {
        recaller.remember(source);
    }
}
