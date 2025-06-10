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

use crate::{
    site::{
        AddModifier, Affiliation, ChangeCurrentScenario, Delete, GetModifier, InheritedTask,
        Modifier, Pending, Property, RecallTask, RemoveModifier, ScenarioMarker, ScenarioModifiers,
        StandardProperty, Task, TaskModifier, TaskParams, UpdateModifier, UpdateProperty,
    },
    widgets::tasks::{EditMode, EditModeEvent, EditTask},
    CurrentWorkspace,
};
use bevy::ecs::{
    hierarchy::ChildOf,
    system::{SystemParam, SystemState},
};
use bevy::prelude::*;
use std::collections::HashSet;

impl StandardProperty for TaskParams {}

impl Modifier<TaskParams> for TaskModifier {
    fn get(&self) -> Option<TaskParams> {
        self.params()
    }

    fn insert(for_element: Entity, in_scenario: Entity, value: TaskParams, world: &mut World) {
        let mut state: SystemState<(
            Query<(&mut TaskModifier, &Affiliation<Entity>)>,
            Query<
                (Entity, &ScenarioModifiers<Entity>, Ref<Affiliation<Entity>>),
                With<ScenarioMarker>,
            >,
            EventWriter<AddModifier>,
        )> = SystemState::new(world);
        let (mut task_modifiers, scenarios, _) = state.get_mut(world);

        // Insert task modifier entities when new tasks are created
        let Ok((_, scenario_modifiers, _)) = scenarios.get(in_scenario) else {
            return;
        };
        let mut new_modifiers = Vec::<(TaskModifier, Entity)>::new();

        if let Some((mut task_modifier, _)) = scenario_modifiers
            .get(&for_element)
            .and_then(|e| task_modifiers.get_mut(*e).ok())
        {
            // If a task modifier entity already exists for this scenario, update it
            let task_modifier = task_modifier.as_mut();
            match task_modifier {
                TaskModifier::Added(_) => *task_modifier = TaskModifier::added(value.clone()),
                TaskModifier::Inherited(inherited) => {
                    inherited.modified_params = Some(value.clone())
                }
                TaskModifier::Hidden => {}
            }
        } else {
            // If root modifier entity does not exist in this scenario, spawn one
            new_modifiers.push((TaskModifier::added(value.clone()), in_scenario));
        }

        // Retrieve root scenario of current scenario
        let mut current_root_entity: Entity = in_scenario;
        while let Ok((_, _, parent_scenario)) = scenarios.get(current_root_entity) {
            if let Some(parent_scenario_entity) = parent_scenario.0 {
                current_root_entity = parent_scenario_entity;
            } else {
                break;
            }
        }
        // Insert task modifier into all root scenarios outside of the current tree as hidden
        for (scenario_entity, _, parent_scenario) in scenarios.iter() {
            if parent_scenario.0.is_some() || scenario_entity == current_root_entity {
                continue;
            }
            new_modifiers.push((TaskModifier::Hidden, scenario_entity));
        }

        // Spawn all new modifier entities
        let new_modifier_entities = new_modifiers
            .iter()
            .map(|(modifier, scenario)| (world.spawn(modifier.clone()).id(), *scenario))
            .collect::<Vec<(Entity, Entity)>>();
        let (_, _, mut add_modifier) = state.get_mut(world);
        for (modifier_entity, scenario_entity) in new_modifier_entities.iter() {
            add_modifier.write(AddModifier::new(
                for_element,
                *modifier_entity,
                *scenario_entity,
            ));
        }
    }

    fn insert_on_new_scenario(in_scenario: Entity, world: &mut World) {
        let mut state: SystemState<(
            Query<&Children>,
            Query<(&TaskModifier, &Affiliation<Entity>)>,
            Query<Entity, (With<Task>, Without<Pending>)>,
            EventWriter<AddModifier>,
            EventWriter<ChangeCurrentScenario>,
        )> = SystemState::new(world);
        let (children, task_modifiers, task_entity, _, _) = state.get_mut(world);

        // Insert task modifier entities when new root scenarios are created
        let mut have_task = HashSet::new();
        if let Ok(scenario_children) = children.get(in_scenario) {
            for child in scenario_children {
                if let Ok((_, a)) = task_modifiers.get(*child) {
                    if let Some(a) = a.0 {
                        have_task.insert(a);
                    }
                }
            }
        }

        let mut target_tasks = HashSet::new();
        for task_entity in task_entity.iter() {
            if !have_task.contains(&task_entity) {
                target_tasks.insert(task_entity);
            }
        }

        let mut new_modifiers = Vec::<(Entity, Entity)>::new();
        for target in target_tasks.iter() {
            // Mark all task modifiers as Hidden
            new_modifiers.push((*target, world.commands().spawn(TaskModifier::Hidden).id()));
        }

        let (_, _, _, mut add_modifier, mut change_current_scenario) = state.get_mut(world);
        for (task_entity, modifier_entity) in new_modifiers.iter() {
            add_modifier.write(AddModifier::new(
                *task_entity,
                *modifier_entity,
                in_scenario,
            ));
        }

        change_current_scenario.write(ChangeCurrentScenario(in_scenario));
    }

    fn check_inclusion(
        &self,
        for_element: Entity,
        in_scenario: Entity,
        get_modifier: &GetModifier<Self>,
    ) -> bool {
        let mut included: Option<bool> = None;
        let mut entity = in_scenario;
        while included.is_none() {
            if let Some(modifier) = get_modifier.get(entity, for_element) {
                included = match modifier {
                    TaskModifier::Added(_) => Some(true),
                    TaskModifier::Inherited(inherited) => {
                        if inherited.explicit_inclusion {
                            Some(true)
                        } else {
                            None
                        }
                    }
                    TaskModifier::Hidden => Some(false),
                };
            }
            let Some(parent_entity) = get_modifier
                .scenarios
                .get(entity)
                .ok()
                .and_then(|(_, p)| p.0)
            else {
                break;
            };
            entity = parent_entity;
        }
        included.unwrap_or(false)
    }
}

#[derive(Clone, Debug)]
pub enum UpdateTaskModifier {
    Include,
    Hide,
    Modify(TaskParams),
    ResetInclusion,
    ResetParams,
}

/// Updates the current EditTask entity based on the triggered edit mode event
pub fn handle_task_edit(
    mut commands: Commands,
    mut delete: EventWriter<Delete>,
    mut edit_mode: EventReader<EditModeEvent>,
    mut edit_task: ResMut<EditTask>,
    pending_tasks: Query<&mut Task, With<Pending>>,
    current_workspace: Res<CurrentWorkspace>,
) {
    // TODO(@xiyuoh) fix bug where the egui panel glitches when the EditTask resource is being accessed
    if let Some(edit) = edit_mode.read().last() {
        match edit.mode {
            EditMode::New(task_entity) => {
                if let Some(site_entity) = current_workspace.root {
                    commands.entity(task_entity).insert(ChildOf(site_entity));
                }
                edit_task.0 = Some(task_entity);
            }
            EditMode::Edit(task_entity) => {
                if let Some(pending_task) = edit_task.0.filter(|e| pending_tasks.get(*e).is_ok()) {
                    delete.write(Delete::new(pending_task));
                }
                edit_task.0 = task_entity;
            }
        }
    }
}

#[derive(SystemParam)]
pub struct UpdateTaskParams<'w, 's> {
    add_modifier: EventWriter<'w, AddModifier>,
    remove_modifier: EventWriter<'w, RemoveModifier>,
    recall_task: Query<'w, 's, &'static RecallTask>,
    scenarios: Query<
        'w,
        's,
        (
            &'static ScenarioModifiers<Entity>,
            &'static Affiliation<Entity>,
        ),
        With<ScenarioMarker>,
    >,
    task_modifiers: Query<'w, 's, (&'static mut TaskModifier, &'static Affiliation<Entity>)>,
    update_property: EventWriter<'w, UpdateProperty>,
}

pub fn handle_task_modifier_updates(
    world: &mut World,
    state: &mut SystemState<(
        EventReader<UpdateModifier<UpdateTaskModifier>>,
        UpdateTaskParams,
    )>,
) {
    let (mut update_events, _) = state.get_mut(world);
    if update_events.is_empty() {
        return;
    }

    let mut update_task_modifier = Vec::<(UpdateModifier<UpdateTaskModifier>, TaskParams)>::new();
    for update in update_events.read() {
        update_task_modifier.push((update.clone(), TaskParams::default()));
    }
    for (update, task_params) in update_task_modifier.iter_mut() {
        *task_params = TaskParams::get_fallback(update.element, update.scenario, world);
    }

    for (update, fallback_params) in update_task_modifier.iter() {
        let (_, mut params) = state.get_mut(world);
        let Ok((scenario_modifiers, scenario_parent)) = params.scenarios.get(update.scenario)
        else {
            continue;
        };

        if let Some((mut task_modifier, modifier_entity)) =
            scenario_modifiers.get(&update.element).and_then(|e| {
                params
                    .task_modifiers
                    .get_mut(*e)
                    .ok()
                    .map(|(m, _)| m)
                    .zip(Some(e))
            })
        {
            let task_modifier = task_modifier.as_mut();
            match &update.update {
                UpdateTaskModifier::Include => {
                    match task_modifier {
                        TaskModifier::Added(_) => continue,
                        TaskModifier::Inherited(inherited) => {
                            inherited.explicit_inclusion = true;
                        }
                        TaskModifier::Hidden => {
                            if let Some((recall_modifier, recall_params)) = params
                                .recall_task
                                .get(*modifier_entity)
                                .ok()
                                .and_then(|r| r.modifier.as_ref().zip(r.params.clone()))
                            {
                                // RecallTask exists, check for previous
                                match recall_modifier {
                                    TaskModifier::Added(_) => {
                                        *task_modifier = TaskModifier::added(recall_params);
                                    }
                                    TaskModifier::Inherited(_) => {
                                        *task_modifier = TaskModifier::Inherited(InheritedTask {
                                            modified_params: Some(recall_params),
                                            explicit_inclusion: true,
                                        });
                                    }
                                    TaskModifier::Hidden => {} // We don't recall Hidden modifiers
                                }
                            } else {
                                *task_modifier = match scenario_parent.0 {
                                    Some(_) => TaskModifier::inherited_with_inclusion(),
                                    None => TaskModifier::added(fallback_params.clone()),
                                }
                            }
                        }
                    }
                }
                UpdateTaskModifier::Hide => {
                    *task_modifier = TaskModifier::Hidden;
                }
                UpdateTaskModifier::Modify(new_params) => match task_modifier {
                    TaskModifier::Added(_) => {
                        *task_modifier = TaskModifier::added(new_params.clone())
                    }
                    TaskModifier::Inherited(inherited) => {
                        inherited.modified_params = Some(new_params.clone())
                    }
                    TaskModifier::Hidden => {}
                },
                UpdateTaskModifier::ResetParams | UpdateTaskModifier::ResetInclusion => {
                    let inherited = match task_modifier {
                        TaskModifier::Inherited(inherited) => inherited,
                        _ => continue,
                    };
                    match update.update {
                        UpdateTaskModifier::ResetParams => inherited.modified_params = None,
                        UpdateTaskModifier::ResetInclusion => inherited.explicit_inclusion = false,
                        _ => continue,
                    }
                    if !inherited.modified() {
                        params
                            .remove_modifier
                            .write(RemoveModifier::new(update.element, update.scenario));
                    }
                }
            }
        } else {
            let task_modifier = match &update.update {
                UpdateTaskModifier::Include => TaskModifier::inherited_with_inclusion(),
                UpdateTaskModifier::Hide => TaskModifier::Hidden,
                UpdateTaskModifier::Modify(new_params) => {
                    TaskModifier::inherited_with_params(new_params.clone())
                }
                UpdateTaskModifier::ResetParams | UpdateTaskModifier::ResetInclusion => continue,
            };
            let modifier_entity = world.commands().spawn(task_modifier).id();
            let (_, mut params) = state.get_mut(world);
            params.add_modifier.write(AddModifier::new(
                update.element,
                modifier_entity,
                update.scenario,
            ));
            continue;
        }

        let (_, mut params) = state.get_mut(world);
        params
            .update_property
            .write(UpdateProperty::new(update.element, update.scenario));
    }
}
