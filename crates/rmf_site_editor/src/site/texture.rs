/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use rmf_site_format::{Affiliation, Category, Group, Texture};

#[derive(Component)]
pub struct TextureNeedsAssignment;

#[derive(Component, Clone, Debug)]
pub struct TextureImage(pub Handle<Image>);

pub fn fetch_image_for_texture(
    mut commands: Commands,
    mut changed_textures: Query<(Entity, Option<&mut TextureImage>, &Texture), Changed<Texture>>,
    new_textures: Query<Entity, Added<Texture>>,
    asset_server: Res<AssetServer>,
) {
    for (e, image, texture) in &mut changed_textures {
        let asset_path = match String::try_from(&texture.source) {
            Ok(asset_path) => asset_path,
            Err(err) => {
                error!(
                    "Invalid syntax while creating asset path: {err}. \
                    Check that your asset information was input correctly. \
                    Current value:\n{:?}",
                    texture.source,
                );
                continue;
            }
        };

        if let Some(mut image) = image {
            *image = TextureImage(asset_server.load_override(asset_path));
        } else {
            let image: Handle<Image> = asset_server.load_override(asset_path);
            commands.entity(e).insert(TextureImage(image));
        }
    }

    for e in &new_textures {
        commands.entity(e).insert(Category::TextureGroup);
    }
}

pub fn detect_last_selected_texture<T: Component>(
    mut commands: Commands,
    child_of: Query<&ChildOf>,
    mut last_selected: Query<&mut LastSelectedTexture<T>>,
    changed_affiliations: Query<&Affiliation, (Changed<Affiliation>, With<T>)>,
    mut removed_groups: RemovedComponents<Group>,
) {
    if let Some(Affiliation(Some(affiliation))) = changed_affiliations.iter().last() {
        let Ok(child_of) = child_of.get(*affiliation) else {
            return;
        };
        if let Ok(mut last) = last_selected.get_mut(child_of.parent()) {
            last.selection = Some(*affiliation);
        } else {
            commands
                .entity(child_of.parent())
                .insert(LastSelectedTexture {
                    selection: Some(*affiliation),
                    marker: std::marker::PhantomData::<T>::default(),
                });
        }
    }

    for group in removed_groups.read() {
        for mut last in &mut last_selected {
            if last.selection.is_some_and(|l| l == group) {
                last.selection = None;
            }
        }
    }
}

pub fn apply_last_selected_texture<T: Component>(
    mut commands: Commands,
    child_of: Query<&ChildOf>,
    last_selected: Query<&LastSelectedTexture<T>>,
    mut unassigned: Query<(Entity, &mut Affiliation), (With<TextureNeedsAssignment>, With<T>)>,
) {
    for (e, mut affiliation) in &mut unassigned {
        let mut search = e;
        let last = loop {
            if let Ok(last) = last_selected.get(search) {
                break Some(last);
            }

            if let Ok(child_of) = child_of.get(search) {
                search = child_of.parent();
            } else {
                break None;
            }
        };
        if let Some(last) = last {
            affiliation.0 = last.selection;
        }
        commands.entity(e).remove::<TextureNeedsAssignment>();
    }
}

#[derive(Component)]
pub struct LastSelectedTexture<T> {
    selection: Option<Entity>,
    marker: std::marker::PhantomData<T>,
}

// Helper function for entities that need to access their affiliated texture
// information.
pub fn from_texture_source(
    texture_source: &Affiliation,
    textures: &Query<(Option<&TextureImage>, &Texture)>,
) -> (Option<Handle<Image>>, Texture) {
    texture_source
        .0
        .map(|t| textures.get(t).ok())
        .flatten()
        .map(|(i, t)| (i.and_then(|img| Some(img.0.clone())), t.clone()))
        .unwrap_or_else(|| (None, Texture::default()))
}
