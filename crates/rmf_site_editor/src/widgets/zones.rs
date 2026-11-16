/*
 * Copyright (C) 2026 Open Source Robotics Foundation
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
    interaction::AnchorSelection,
    site::{
        Change, CurrentLevel, Delete, NameInSite, ZoneFilter, ZoneMarker, ZoneSet, ZoneSetFilter,
        ZoneSetMarker, ZoneSets,
    },
    widgets::{prelude::*, Icons, Inspect, InspectionPlugin},
    AppState, CurrentWorkspace,
};
use bevy::prelude::*;
use bevy_egui::egui::{Button, ComboBox, ImageButton, RichText, TextEdit};
use rmf_site_egui::*;
use std::collections::HashMap;

pub(crate) struct ZonesPlugin;

impl Plugin for ZonesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PropertiesTilePlugin::<ViewZoneSets>::new("Zones"),
            HeaderTilePlugin::<ZoneCreation>::new(),
            InspectionPlugin::<InspectZoneSets>::new(),
        ));
    }
}

#[derive(SystemParam)]
struct ZoneCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    level: Res<'w, CurrentLevel>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl WidgetSystem<Tile> for ZoneCreation<'_, '_> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if ui
                .add_enabled(params.level.0.is_some(), Button::new("⬡"))
                .on_hover_text("Zone")
                .clicked()
            {
                params.anchor_selection.create_zone();
            }
        }
    }
}

#[derive(SystemParam)]
struct ViewZoneSets<'w, 's> {
    sets: Query<'w, 's, (Entity, &'static NameInSite, &'static ChildOf), With<ZoneSetMarker>>,
    workspace: Res<'w, CurrentWorkspace>,
    app_state: Res<'w, State<AppState>>,
    filter: ResMut<'w, ZoneFilter>,
    delete: EventWriter<'w, Delete>,
    icons: Res<'w, Icons>,
    edits: Local<'s, HashMap<Entity, String>>,
    commands: Commands<'w, 's>,
    new_name: Local<'s, String>,
}

impl WidgetSystem<Tile> for ViewZoneSets<'_, '_> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if *params.app_state.get() != AppState::SiteEditor {
            return;
        }
        let Some(site) = params.workspace.root else {
            return;
        };
        let mut sets: Vec<_> = params
            .sets
            .iter()
            .filter(|(_, _, parent)| parent.parent() == site)
            .map(|(entity, name, _)| (entity, name.0.clone()))
            .collect();
        sets.sort_by(|a, b| a.1.cmp(&b.1));
        if params.filter.site != Some(site) {
            params.filter.site = Some(site);
            params.filter.selection = ZoneSetFilter::All;
        }
        let selected = match params.filter.selection {
            ZoneSetFilter::All => "All zones",
            ZoneSetFilter::Unassigned => "Without a set",
            ZoneSetFilter::Set(set) => sets
                .iter()
                .find(|(entity, _)| *entity == set)
                .map(|(_, name)| name.as_str())
                .unwrap_or("All zones"),
        };
        let mut filter = params.filter.selection;
        ui.horizontal(|ui| {
            ui.label("Filter viewport:");
            ComboBox::from_id_salt("zone_filter")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut filter, ZoneSetFilter::All, "All zones");
                    ui.selectable_value(&mut filter, ZoneSetFilter::Unassigned, "Without a set");
                    for (entity, name) in &sets {
                        ui.selectable_value(&mut filter, ZoneSetFilter::Set(*entity), name);
                    }
                });
        });
        if filter != params.filter.selection {
            params.filter.selection = filter;
        }
        ui.separator();
        params
            .edits
            .retain(|entity, _| sets.iter().any(|(set, _)| set == entity));
        for (entity, name) in &sets {
            ui.push_id(entity, |ui| {
                ui.horizontal(|ui| {
                    let draft = params.edits.entry(*entity).or_insert_with(|| name.clone());
                    let response = ui.add(TextEdit::singleline(draft).desired_width(160.0));
                    if response.lost_focus() {
                        let new_name = draft.trim().to_owned();
                        if new_name != *name
                            && !new_name.is_empty()
                            && !sets
                                .iter()
                                .any(|(other, name)| other != entity && *name == new_name)
                        {
                            params
                                .commands
                                .trigger(Change::new(NameInSite(new_name), *entity));
                        }
                        params.edits.remove(entity);
                    } else if !response.has_focus() {
                        params.edits.remove(entity);
                    }
                    if ui
                        .add(ImageButton::new(params.icons.trash.egui()))
                        .on_hover_text("Delete zone set")
                        .clicked()
                    {
                        params.delete.write(Delete::new(*entity));
                    }
                });
            });
        }
        if sets.is_empty() {
            ui.label(
                RichText::new("No zone sets currently exist. Add a named set below.")
                    .weak()
                    .italics(),
            );
        }
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut *params.new_name)
                    .hint_text(RichText::new("Set name").weak().italics())
                    .desired_width(160.0),
            );
            let name = params.new_name.trim().to_owned();
            let valid = !name.is_empty() && !sets.iter().any(|(_, existing)| *existing == name);
            if ui.add_enabled(valid, Button::new("Add")).clicked() {
                params.commands.spawn((
                    ZoneSet {
                        name: NameInSite(name),
                        marker: ZoneSetMarker,
                    },
                    ChildOf(site),
                ));
                params.new_name.clear();
            }
        });
    }
}

#[derive(SystemParam)]
struct InspectZoneSets<'w, 's> {
    zones: Query<'w, 's, &'static ZoneSets<Entity>, With<ZoneMarker>>,
    sets: Query<'w, 's, (Entity, &'static NameInSite, &'static ChildOf), With<ZoneSetMarker>>,
    workspace: Res<'w, CurrentWorkspace>,
    commands: Commands<'w, 's>,
    icons: Res<'w, Icons>,
    choice: Local<'s, Option<Entity>>,
}

impl ShareableWidget for InspectZoneSets<'_, '_> {}

impl WidgetSystem<Inspect> for InspectZoneSets<'_, '_> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        let Ok(membership) = params.zones.get(selection) else {
            return;
        };
        let Some(site) = params.workspace.root else {
            return;
        };
        let mut membership = membership.clone();
        let mut sets: Vec<_> = params
            .sets
            .iter()
            .filter(|(_, _, parent)| parent.parent() == site)
            .map(|(entity, name, _)| (entity, name.0.clone()))
            .collect();
        sets.sort_by(|a, b| a.1.cmp(&b.1));
        ui.label("Associated Zone Sets");
        if sets.is_empty() {
            ui.label(
                RichText::new("No zone sets currently exist. Create a set in the Zones tab.")
                    .weak()
                    .italics(),
            );
        }
        let mut changed = false;
        for (entity, name) in &sets {
            if !membership.0.contains(entity) {
                continue;
            }
            ui.push_id(entity, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add(ImageButton::new(params.icons.trash.egui()))
                        .on_hover_text("Remove from zone set")
                        .clicked()
                    {
                        membership.0.remove(entity);
                        changed = true;
                    }
                    ui.label(name);
                });
            });
        }
        let unused: Vec<_> = sets
            .iter()
            .filter(|(entity, _)| !membership.0.contains(entity))
            .collect();
        if let Some((first, _)) = unused.first() {
            let mut choice = params
                .choice
                .filter(|choice| unused.iter().any(|(entity, _)| entity == choice))
                .unwrap_or(*first);
            ui.horizontal(|ui| {
                let add = ui.button("Add").clicked();
                ComboBox::from_id_salt(("add_zone_set", selection))
                    .selected_text(
                        &unused
                            .iter()
                            .find(|(entity, _)| *entity == choice)
                            .unwrap()
                            .1,
                    )
                    .show_ui(ui, |ui| {
                        for (entity, name) in &unused {
                            ui.selectable_value(&mut choice, *entity, name);
                        }
                    });
                if add {
                    membership.0.insert(choice);
                    changed = true;
                }
            });
            *params.choice = Some(choice);
        }
        if changed {
            params.commands.trigger(Change::new(membership, selection));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_egui::egui::{self, Event, PointerButton};

    fn show_sets(
        context: &egui::Context,
        world: &mut World,
        state: &mut SystemState<ViewZoneSets>,
        events: Vec<Event>,
    ) -> egui::FullOutput {
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(500.0, 300.0),
                )),
                events,
                ..default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    ViewZoneSets::show(
                        Tile {
                            id: Entity::PLACEHOLDER,
                            panel: PanelSettings::right(),
                        },
                        ui,
                        state,
                        world,
                    );
                });
            },
        );
        state.apply(world);
        output
    }

    fn focus_name(
        label: &str,
        context: &egui::Context,
        world: &mut World,
        state: &mut SystemState<ViewZoneSets>,
    ) {
        let output = show_sets(context, world, state, vec![]);
        let position = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.pos + text.galley.size() / 2.0)
                }
                _ => None,
            })
            .expect("name field should be visible");
        for pressed in [true, false] {
            show_sets(
                context,
                world,
                state,
                vec![
                    Event::PointerMoved(position),
                    Event::PointerButton {
                        pos: position,
                        button: PointerButton::Primary,
                        pressed,
                        modifiers: default(),
                    },
                ],
            );
        }
    }

    fn key(key: egui::Key, modifiers: egui::Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn rename_keeps_intermediate_edits_until_focus_is_lost() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<bevy_egui::EguiUserTextures>()
            .init_resource::<Icons>()
            .init_resource::<ZoneFilter>()
            .add_event::<Delete>()
            .add_plugins(crate::site::ChangePlugin::<NameInSite>::default());
        let site = app.world_mut().spawn_empty().id();
        app.insert_resource(CurrentWorkspace {
            root: Some(site),
            ..default()
        })
        .insert_resource(State::new(AppState::SiteEditor));
        let world = app.world_mut();
        world.spawn((
            ZoneSet {
                name: NameInSite("A".into()),
                marker: ZoneSetMarker,
            },
            ChildOf(site),
        ));
        let set = world
            .spawn((
                ZoneSet {
                    name: NameInSite("AB".into()),
                    marker: ZoneSetMarker,
                },
                ChildOf(site),
            ))
            .id();
        let mut state = SystemState::<ViewZoneSets>::new(world);
        let context = egui::Context::default();
        focus_name("AB", &context, world, &mut state);
        let select_all = egui::Modifiers {
            ctrl: true,
            command: true,
            ..default()
        };
        show_sets(
            &context,
            world,
            &mut state,
            vec![
                key(egui::Key::A, select_all),
                key(egui::Key::Backspace, default()),
            ],
        );
        // A valid rename can pass through empty text and another set's name.
        assert_eq!(state.get_mut(world).edits.get(&set).unwrap(), "");
        show_sets(&context, world, &mut state, vec![Event::Text("A".into())]);
        assert_eq!(state.get_mut(world).edits.get(&set).unwrap(), "A");
        assert_eq!(world.get::<NameInSite>(set).unwrap().0, "AB");
        show_sets(&context, world, &mut state, vec![Event::Text("C".into())]);
        assert_eq!(world.get::<NameInSite>(set).unwrap().0, "AB");
        show_sets(
            &context,
            world,
            &mut state,
            vec![key(egui::Key::Enter, default())],
        );
        assert_eq!(world.get::<NameInSite>(set).unwrap().0, "AC");
    }
}
