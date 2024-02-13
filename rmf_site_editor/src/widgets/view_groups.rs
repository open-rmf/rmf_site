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
    site::{Change, FiducialMarker, Members, MergeGroups, NameInSite, SiteID, Texture},
    widgets::{inspector::SelectionWidget, AppEvents},
    Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, CollapsingHeader, ImageButton, Ui};

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
}

impl GroupViewModes {
    pub fn reset(&mut self, site: Entity) {
        *self = GroupViewModes::default();
        self.site = Some(site);
    }
}

#[derive(SystemParam)]
pub struct GroupParams<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    textures: Query<'w, 's, (&'static NameInSite, Option<&'static SiteID>), With<Texture>>,
    fiducials: Query<'w, 's, (&'static NameInSite, Option<&'static SiteID>), With<FiducialMarker>>,
    icons: Res<'w, Icons>,
    group_view_modes: ResMut<'w, GroupViewModes>,
}

pub struct ViewGroups<'a, 'w1, 's1, 'w2, 's2> {
    params: &'a mut GroupParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 's1, 'w2, 's2> ViewGroups<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(params: &'a mut GroupParams<'w1, 's1>, events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { params, events }
    }

    pub fn show(self, ui: &mut Ui) {
        let Some(site) = self.events.request.current_workspace.root else {
            return;
        };
        let modes = &mut *self.params.group_view_modes;
        if !modes.site.is_some_and(|s| s == site) {
            modes.reset(site);
        }
        let Ok(children) = self.params.children.get(site) else {
            return;
        };
        CollapsingHeader::new("Textures").show(ui, |ui| {
            Self::show_groups(
                children,
                &self.params.textures,
                &mut modes.textures,
                &self.params.icons,
                self.events,
                ui,
            );
        });
        CollapsingHeader::new("Fiducials").show(ui, |ui| {
            Self::show_groups(
                children,
                &self.params.fiducials,
                &mut modes.fiducials,
                &self.params.icons,
                self.events,
                ui,
            );
        });
    }

    fn show_groups<'b, T: Component>(
        children: impl IntoIterator<Item = &'b Entity>,
        q_groups: &Query<(&NameInSite, Option<&SiteID>), With<T>>,
        mode: &mut GroupViewMode,
        icons: &Res<Icons>,
        events: &mut AppEvents,
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
                        SelectionWidget::new(*child, site_id.cloned(), &icons, events).show(ui);
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
                                events.change.merge_groups.send(MergeGroups {
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
                            events.commands.entity(*child).despawn_recursive();
                            *mode = GroupViewMode::View;
                        }
                    }
                }

                let mut new_name = name.0.clone();
                if ui.text_edit_singleline(&mut new_name).changed() {
                    events
                        .change
                        .name
                        .send(Change::new(NameInSite(new_name), *child));
                }
            });
        }
    }
}
