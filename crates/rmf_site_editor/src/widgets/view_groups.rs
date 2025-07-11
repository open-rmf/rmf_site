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

use crate::{
    interaction::ObjectPlacement,
    site::{
        Affiliation, Change, Delete, FiducialMarker, Group, MergeGroups, ModelInstance,
        ModelMarker, NameInSite, SiteID, Texture,
    },
    widgets::{prelude::*, SelectorWidget},
    AppState, CurrentWorkspace, Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, CollapsingHeader, Ui};
use std::any::TypeId;

/// Add a widget for viewing different kinds of groups.
#[derive(Default)]
pub struct ViewGroupsPlugin {}

impl Plugin for ViewGroupsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GroupViewModes>()
            .add_plugins(PropertiesTilePlugin::<ViewGroups>::new());
    }
}

#[derive(SystemParam)]
pub struct ViewGroups<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    textures:
        Query<'w, 's, (&'static NameInSite, Option<&'static SiteID>), (With<Texture>, With<Group>)>,
    fiducials: Query<
        'w,
        's,
        (&'static NameInSite, Option<&'static SiteID>),
        (With<FiducialMarker>, With<Group>),
    >,
    model_descriptions: Query<
        'w,
        's,
        (&'static NameInSite, Option<&'static SiteID>),
        (With<ModelMarker>, With<Group>),
    >,
    icons: Res<'w, Icons>,
    group_view_modes: ResMut<'w, GroupViewModes>,
    app_state: Res<'w, State<AppState>>,
    events: ViewGroupsEvents<'w, 's>,
}

#[derive(SystemParam)]
pub struct ViewGroupsEvents<'w, 's> {
    current_workspace: ResMut<'w, CurrentWorkspace>,
    selector: SelectorWidget<'w, 's>,
    merge_groups: EventWriter<'w, MergeGroups>,
    delete: EventWriter<'w, Delete>,
    name: EventWriter<'w, Change<NameInSite>>,
    commands: Commands<'w, 's>,
    object_placement: ObjectPlacement<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewGroups<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if *params.app_state.get() != AppState::SiteEditor {
            return;
        }
        CollapsingHeader::new("Groups")
            .default_open(false)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w, 's> ViewGroups<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        let Some(site) = self.events.current_workspace.root else {
            return;
        };
        let modes = &mut *self.group_view_modes;
        if !modes.site.is_some_and(|s| s == site) {
            modes.reset(site);
        }
        let Ok(children) = self.children.get(site) else {
            return;
        };
        CollapsingHeader::new("Model Descriptions").show(ui, |ui| {
            Self::show_groups(
                children,
                &self.model_descriptions,
                &mut modes.model_descriptions,
                &self.icons,
                &mut self.events,
                ui,
            );
        });
        CollapsingHeader::new("Textures").show(ui, |ui| {
            Self::show_groups(
                children,
                &self.textures,
                &mut modes.textures,
                &self.icons,
                &mut self.events,
                ui,
            );
        });
        CollapsingHeader::new("Fiducials").show(ui, |ui| {
            Self::show_groups(
                children,
                &self.fiducials,
                &mut modes.fiducials,
                &self.icons,
                &mut self.events,
                ui,
            );
        });
    }

    fn show_groups<'b, T: Component>(
        children: impl IntoIterator<Item = &'b Entity>,
        q_groups: &Query<(&NameInSite, Option<&SiteID>), (With<T>, With<Group>)>,
        mode: &mut GroupViewMode,
        icons: &Res<Icons>,
        events: &mut ViewGroupsEvents,
        ui: &mut Ui,
    ) {
        ui.horizontal(|ui| match mode {
            GroupViewMode::View => {
                if ui
                    .add(Button::image_and_text(icons.merge.egui(), "merge"))
                    .on_hover_text("Merge two groups")
                    .clicked()
                {
                    info!("Select a group whose members will be merged into another group");
                    *mode = GroupViewMode::SelectMergeFrom;
                }
                if ui
                    .add(Button::image_and_text(icons.trash.egui(), "delete"))
                    .on_hover_text("Delete a group")
                    .clicked()
                {
                    info!("Deleting a group will make all its members unaffiliated");
                    *mode = GroupViewMode::Delete;
                }
            }
            GroupViewMode::MergeFrom(_) | GroupViewMode::SelectMergeFrom => {
                if ui
                    .add(Button::image_and_text(icons.exit.egui(), "cancel"))
                    .on_hover_text("Cancel the merge")
                    .clicked()
                {
                    *mode = GroupViewMode::View;
                }
            }
            GroupViewMode::Delete => {
                if ui
                    .add(Button::image_and_text(icons.exit.egui(), "cancel"))
                    .on_hover_text("Cancel the delete")
                    .clicked()
                {
                    *mode = GroupViewMode::View;
                }
            }
        });

        for child in children {
            let Ok((name, site_id)) = q_groups.get(*child) else {
                continue;
            };
            let text = site_id
                .map(|s| format!("{}", s.0.clone()))
                .unwrap_or_else(|| "*".to_owned());
            ui.horizontal(|ui| {
                match mode.clone() {
                    GroupViewMode::View => {
                        if TypeId::of::<T>() == TypeId::of::<ModelMarker>() {
                            if ui
                                .add(Button::image(icons.add.egui()))
                                .on_hover_text("Add a new model instance of this group")
                                .clicked()
                            {
                                let model_instance: ModelInstance = ModelInstance {
                                    description: Affiliation(Some(child.clone())),
                                    ..Default::default()
                                };
                                events.object_placement.place_object_2d(model_instance);
                            }
                        };
                        events.selector.show_widget(*child, ui);
                    }
                    GroupViewMode::SelectMergeFrom => {
                        if ui
                            .add(Button::image_and_text(icons.merge.egui(), &text))
                            .on_hover_text("Merge the members of this group into another group")
                            .clicked()
                        {
                            *mode = GroupViewMode::MergeFrom(*child);
                        }
                    }
                    GroupViewMode::MergeFrom(merge_from) => {
                        if merge_from == *child {
                            if ui
                                .add(Button::image_and_text(icons.exit.egui(), &text))
                                .on_hover_text("Cancel merge")
                                .clicked()
                            {
                                *mode = GroupViewMode::View;
                            }
                        } else {
                            if ui
                                .add(Button::image_and_text(icons.confirm.egui(), &text))
                                .on_hover_text("Merge into this group")
                                .clicked()
                            {
                                events.merge_groups.write(MergeGroups {
                                    from_group: merge_from,
                                    into_group: *child,
                                });
                                *mode = GroupViewMode::View;
                            }
                        }
                    }
                    GroupViewMode::Delete => {
                        if ui
                            .add(Button::image_and_text(icons.trash.egui(), &text))
                            .on_hover_text("Delete this group")
                            .clicked()
                        {
                            if TypeId::of::<T>() == TypeId::of::<ModelMarker>() {
                                events.delete.write(Delete::new(*child).and_dependents());
                            } else {
                                events.commands.entity(*child).despawn();
                                *mode = GroupViewMode::View;
                            }
                        }
                    }
                }

                let mut new_name = name.0.clone();
                if ui.text_edit_singleline(&mut new_name).changed() {
                    events.name.write(Change::new(NameInSite(new_name), *child));
                }
            });
        }
    }
}

#[derive(Default, Clone, Copy)]
pub enum GroupViewMode {
    #[default]
    View,
    SelectMergeFrom,
    MergeFrom(Entity),
    Delete,
}

#[derive(Default, Resource)]
pub struct GroupViewModes {
    site: Option<Entity>,
    textures: GroupViewMode,
    fiducials: GroupViewMode,
    model_descriptions: GroupViewMode,
}

impl GroupViewModes {
    pub fn reset(&mut self, site: Entity) {
        *self = GroupViewModes::default();
        self.site = Some(site);
    }
}
