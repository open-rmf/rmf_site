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

use crate::site::{
    load::{LoadingArgs, LoadingResult, LoadingSystem},
    save::{SavingArgs, SavingResult, SavingSystem},
    ExtensionSettings,
};
use bevy::{ecs::system::IntoSystem, prelude::*};

use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    sync::Arc,
};

#[derive(Resource, Default)]
pub struct ExtensionHooks {
    pub(crate) hooks: HashMap<Arc<str>, ExtensionHook>,
}

fn into_extension_error<E: Error + 'static>(error: E) -> Arc<dyn Error> {
    Arc::new(error)
}

#[derive(Default)]
pub(crate) struct ExtensionHook {
    pub(crate) saving: Option<SavingSystem>,
    pub(crate) loading: Option<LoadingSystem>,
    pub(crate) default_settings: ExtensionSettings,
}

pub trait SetSiteExtensionHook {
    fn set_site_extension_hook<
        SaveM,
        LoadM,
        SavingError: Error + 'static,
        LoadingError: Error + 'static,
    >(
        &mut self,
        extension: &str,
        default_settings: ExtensionSettings,
        saving: impl IntoSystem<In<SavingArgs>, SavingResult<SavingError>, SaveM>,
        loading: impl IntoSystem<In<LoadingArgs>, LoadingResult<LoadingError>, LoadM>,
    );
}

impl SetSiteExtensionHook for World {
    fn set_site_extension_hook<
        SaveM,
        LoadM,
        SavingError: Error + 'static,
        LoadingError: Error + 'static,
    >(
        &mut self,
        extension: &str,
        default_settings: ExtensionSettings,
        saving: impl IntoSystem<In<SavingArgs>, SavingResult<SavingError>, SaveM>,
        loading: impl IntoSystem<In<LoadingArgs>, LoadingResult<LoadingError>, LoadM>,
    ) {
        self.get_resource_or_insert_with(|| ExtensionHooks::default());
        self.resource_scope::<ExtensionHooks, _>(|world, mut extensions| {
            let hook = match extensions.hooks.entry(extension.into()) {
                Entry::Occupied(occupied) => occupied.into_mut(),
                Entry::Vacant(vacant) => vacant.insert(ExtensionHook {
                    default_settings,
                    ..Default::default()
                }),
            };

            let mut saving = Box::new(IntoSystem::into_system(saving.pipe(
                |In(result): In<SavingResult<SavingError>>| result.map_err(into_extension_error),
            )));
            saving.initialize(world);

            if hook.saving.replace(saving).is_some() {
                warn!("Replacing the saving hook for [{extension}] extension");
            }

            let mut loading = Box::new(IntoSystem::into_system(loading.pipe(
                |In(result): In<LoadingResult<LoadingError>>| result.map_err(into_extension_error),
            )));
            loading.initialize(world);

            if hook.loading.replace(loading).is_some() {
                warn!("Replacing the loading hook for [{extension}] extension");
            }
        });
    }
}
