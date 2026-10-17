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
    exit_confirmation::SiteChanged,
    interaction::AnchorSelection,
    site::{
        Change, CurrentLevel, Group, NameInSite, ZoneFilter, ZoneMarker, ZoneSetFilter,
        ZoneSetMarker, ZoneSets,
    },
    widgets::{prelude::*, Inspect, InspectionPlugin},
    AppState, CurrentWorkspace,
};
use bevy::prelude::*;
use bevy_egui::egui::{ComboBox, TextEdit};
use rmf_site_egui::*;

pub(crate) struct ZonesPlugin;

#[derive(Resource)]
struct ZoneMenu(Entity);

impl Plugin for ZonesPlugin {
    fn build(&self, app: &mut App) {
        let tool = app.world().resource::<ToolMenu>().get();
        let item = app
            .world_mut()
            .spawn((MenuItem::Text("Add Zone".into()), ChildOf(tool)))
            .id();
        app.add_plugins((
            PropertiesTilePlugin::<ViewZoneSets>::new("Zone Sets"),
            InspectionPlugin::<InspectZoneSets>::new(),
        ))
        .insert_resource(ZoneMenu(item))
        .add_systems(Update, draw_zone.run_if(in_state(AppState::SiteEditor)));
    }
}

fn draw_zone(
    mut events: EventReader<MenuEvent>,
    menu: Res<ZoneMenu>,
    level: Res<CurrentLevel>,
    mut selection: AnchorSelection,
    mouse: Res<ButtonInput<MouseButton>>,
    mut pending: Local<bool>,
) {
    // Don't let the menu click place the first vertex.
    if *pending && !mouse.pressed(MouseButton::Left) && !mouse.just_released(MouseButton::Left) {
        *pending = false;
        if level.0.is_some() {
            selection.create_zone();
        }
    }
    for event in events.read() {
        if event.clicked() && event.source() == menu.0 {
            *pending = true;
        }
    }
}

#[derive(SystemParam)]
struct ViewZoneSets<'w, 's> {
    sets: Query<'w, 's, (Entity, &'static NameInSite, &'static ChildOf), With<ZoneSetMarker>>,
    workspace: Res<'w, CurrentWorkspace>,
    app_state: Res<'w, State<AppState>>,
    filter: ResMut<'w, ZoneFilter>,
    site_changed: ResMut<'w, SiteChanged>,
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
        ComboBox::from_label("Show")
            .selected_text(selected)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut params.filter.selection,
                    ZoneSetFilter::All,
                    "All zones",
                );
                ui.selectable_value(
                    &mut params.filter.selection,
                    ZoneSetFilter::Unassigned,
                    "Without a set",
                );
                for (entity, name) in &sets {
                    ui.selectable_value(
                        &mut params.filter.selection,
                        ZoneSetFilter::Set(*entity),
                        name,
                    );
                }
            });
        ui.separator();
        for (entity, name) in &sets {
            ui.push_id(entity, |ui| {
                ui.horizontal(|ui| {
                    let mut new_name = name.clone();
                    if ui
                        .add(TextEdit::singleline(&mut new_name).desired_width(160.0))
                        .changed()
                        && !new_name.trim().is_empty()
                        && !sets
                            .iter()
                            .any(|(other, name)| other != entity && *name == new_name)
                    {
                        params
                            .commands
                            .trigger(Change::new(NameInSite(new_name), *entity));
                    }
                    if ui.button("Delete").clicked() {
                        params.commands.entity(*entity).despawn();
                        params.site_changed.0 = true;
                    }
                });
            });
        }
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut *params.new_name)
                    .hint_text("Set name")
                    .desired_width(160.0),
            );
            let name = params.new_name.trim().to_owned();
            let valid = !name.is_empty() && !sets.iter().any(|(_, existing)| *existing == name);
            if ui
                .add_enabled(valid, bevy_egui::egui::Button::new("Add"))
                .clicked()
            {
                params
                    .commands
                    .spawn((NameInSite(name), ZoneSetMarker, Group, ChildOf(site)));
                params.site_changed.0 = true;
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
        ui.label("Zone sets");
        if sets.is_empty() {
            ui.label("Create a set in the Zone Sets tab.");
        }
        let mut changed = false;
        for (entity, name) in sets {
            let mut member = membership.0.contains(&entity);
            if ui.checkbox(&mut member, name).changed() {
                if member {
                    membership.0.insert(entity);
                } else {
                    membership.0.remove(&entity);
                }
                changed = true;
            }
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

    fn click_button(
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
            .expect("button should be visible");
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

    #[test]
    fn creating_and_deleting_zone_sets_marks_unsaved_changes() {
        let mut world = World::new();
        let site = world.spawn_empty().id();
        world.insert_resource(CurrentWorkspace {
            root: Some(site),
            ..default()
        });
        world.insert_resource(State::new(AppState::SiteEditor));
        world.init_resource::<ZoneFilter>();
        world.init_resource::<SiteChanged>();
        let mut state = SystemState::<ViewZoneSets>::new(&mut world);
        *state.get_mut(&mut world).new_name = "Survey".into();
        let context = egui::Context::default();
        show_sets(&context, &mut world, &mut state, vec![]);
        assert!(!world.resource::<SiteChanged>().0);

        click_button("Add", &context, &mut world, &mut state);
        assert!(world.resource::<SiteChanged>().0);
        let set = world
            .query_filtered::<Entity, With<ZoneSetMarker>>()
            .single(&world)
            .unwrap();
        assert_eq!(world.get::<ChildOf>(set).unwrap().parent(), site);

        // Saving clears the flag; deleting must mark the site changed again.
        world.resource_mut::<SiteChanged>().0 = false;
        click_button("Delete", &context, &mut world, &mut state);
        assert!(world.resource::<SiteChanged>().0);
        assert!(world.get_entity(set).is_err());
    }
}
