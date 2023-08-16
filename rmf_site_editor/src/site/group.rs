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

use rmf_site_format::{Affiliation, Group};
use bevy::{
    ecs::system::{Command, EntityCommands},
    prelude::*,
};

pub struct MergeGroups {
    pub from_group: Entity,
    pub into_group: Entity,
}

#[derive(Component, Deref)]
pub struct Members(Vec<Entity>);

impl Members {
    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.0.iter()
    }
}

#[derive(Component, Clone, Copy)]
struct LastAffiliation(Option<Entity>);

pub fn update_affiliations(
    mut affiliations: Query<&mut Affiliation<Entity>>,
    mut merge_groups: EventReader<MergeGroups>,
    deleted_groups: RemovedComponents<Group>,
) {
    for merge in merge_groups.iter() {
        for mut affiliation in &mut affiliations {
            if affiliation.0.is_some_and(|a| a == merge.from_group) {
                affiliation.0 = Some(merge.into_group);
            }
        }
    }

    for deleted in &deleted_groups {
        for mut affiliation in &mut affiliations {
            if affiliation.0.is_some_and(|a| a == deleted) {
                affiliation.0 = None;
            }
        }
    }
}

pub fn update_members_of_groups(
    mut commands: Commands,
    changed_affiliation: Query<(Entity, &Affiliation<Entity>), Changed<Affiliation<Entity>>>,
) {
    for (e, affiliation) in &changed_affiliation {
        commands.entity(e).set_membership(affiliation.0);
    }
}

struct ChangeMembership {
    member: Entity,
    group: Option<Entity>,
}

impl Command for ChangeMembership {
    fn write(self, world: &mut World) {
        let last = world.get_entity(self.member)
            .map(|e| e.get::<LastAffiliation>()).flatten().cloned();
        if let Some(last) = last {
            if last.0 == self.group {
                // There is no effect from this change
                return;
            }

            if let Some(last) = last.0 {
                if let Some(mut e) = world.get_entity_mut(last) {
                    if let Some(mut members) = e.get_mut::<Members>() {
                        members.0.retain(|m| *m != self.member);
                    }
                }
            }
        }

        if let Some(new_group) = self.group {
            if let Some(mut e) = world.get_entity_mut(new_group) {
                if let Some(mut members) = e.get_mut::<Members>() {
                    members.0.push(self.member);
                } else {
                    e.insert(Members(vec![self.member]));
                }
            }
        }

        if let Some(mut e) = world.get_entity_mut(self.member) {
            e.insert(LastAffiliation(self.group));
        }
    }
}

pub trait SetMembershipExt {
    fn set_membership(&mut self, group: Option<Entity>) -> &mut Self;
}

impl<'w, 's, 'a> SetMembershipExt for EntityCommands<'w, 's, 'a> {
    fn set_membership(&mut self, group: Option<Entity>) -> &mut Self {
        let member = self.id();
        self.commands().add(ChangeMembership { member, group });
        self
    }
}
