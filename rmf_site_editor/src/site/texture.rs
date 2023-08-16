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

use rmf_site_format::{Texture, Affiliation, Category};
use bevy::prelude::*;

pub fn fetch_image_for_texture(
    mut commands: Commands,
    mut changed_textures: Query<(Entity, Option<&mut Handle<Image>>, &Texture), Changed<Texture>>,
    new_textures: Query<Entity, Added<Texture>>,
    asset_server: Res<AssetServer>,
) {
    for (e, image, texture) in &mut changed_textures {
        if let Some(mut image) = image {
            *image = asset_server.load(String::from(&texture.source));
        } else {
            let image: Handle<Image> = asset_server.load(String::from(&texture.source));
            commands.entity(e).insert(image);
        }
    }

    for e in &new_textures {
        commands.entity(e).insert(Category::TextureGroup);
    }
}

// Helper function for entities that need to access their affiliated texture
// information.
pub fn from_texture_source(
    texture_source: &Affiliation<Entity>,
    textures: &Query<(Option<&Handle<Image>>, &Texture)>,
) -> (Option<Handle<Image>>, Texture) {
    texture_source.0
    .map(|t| textures.get(t).ok())
    .flatten()
    .map(|(i, t)| (i.cloned(), t.clone()))
    .unwrap_or_else(|| (None, Texture::default()))
}
