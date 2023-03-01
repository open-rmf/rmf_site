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

use bevy::prelude::*;
use std::{
    marker::PhantomData,
    collections::{HashSet, HashMap},
    ops::Deref,
};

#[derive(Debug, Clone, Component)]
pub struct RecencyRanking<T: Component> {
    /// Entities are ordered from lowest to highest rank. Higher ranks should be
    /// displayed over lower ranks.
    entities: Vec<Entity>,
    _ignore: PhantomData<T>,
}

impl<T: Component> RecencyRanking<T> {
    pub fn new() -> Self {
        Self { entities: default(), _ignore: default() }
    }

    pub fn entities(&self) -> &Vec<Entity> {
        &self.entities
    }

    pub fn from_u32(
        ranking: &Vec<u32>,
        id_to_entity: &HashMap<u32, Entity>
    ) -> Result<Self, u32> {
        let entities: Result<Vec<Entity>, u32> = ranking.iter().map(
            |id| id_to_entity.get(id).copied().ok_or(*id)
        ).collect();
        let entities = entities?;
        Ok(Self { entities, _ignore: default() })
    }
}

impl<T: Component> Default for RecencyRanking<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Component> Deref for RecencyRanking<T> {
    type Target = Vec<Entity>;
    fn deref(&self) -> &Self::Target {
        &self.entities
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChangeRank<T: Component> {
    of: Entity,
    by: RankAdjustment,
    _ignore: PhantomData<T>,
}

impl<T: Component> ChangeRank<T> {
    pub fn new(of: Entity, by: RankAdjustment) -> Self {
        Self { of, by, _ignore: default() }
    }

    pub fn of(&self) -> Entity {
        self.of
    }

    pub fn by(&self) -> RankAdjustment {
        self.by
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RankAdjustment {
    /// Move the entity's rank up (positive) or down (negative) by the given amount.
    Delta(i64),
    /// Set the entity's rank to an exact value.
    ToRank(usize),
}

/// Attach this component to entities that should not be included in recency
/// ranking. Removing this component will treat the entity like it is newly
/// added.
#[derive(Default, Clone, Copy, Component)]
pub struct SuppressRecencyRank;

#[derive(Default)]
pub struct RecencyRankingPlugin<T>(PhantomData<T>);

impl<T: Component> Plugin for RecencyRankingPlugin<T> {
    fn build(&self, app: &mut App) {
        app
        .add_event::<ChangeRank<T>>()
        .add_system(update_recency_rank::<T>);
    }
}

fn update_recency_rank<T: Component>(
    mut rankings: Query<(Entity, &mut RecencyRanking<T>)>,
    new_entities: Query<Entity, (Added<T>, Without<SuppressRecencyRank>)>,
    moved_entities: Query<Entity, (Changed<Parent>, With<T>, Without<SuppressRecencyRank>)>,
    newly_suppressed_entities: Query<Entity, (With<T>, Added<SuppressRecencyRank>)>,
    unsuppressed_entities: RemovedComponents<SuppressRecencyRank>,
    no_longer_relevant: RemovedComponents<T>,
    parents: Query<&Parent>,
    mut rank_changes: EventReader<ChangeRank<T>>,
) {
    for e in new_entities.iter().chain(unsuppressed_entities.iter()) {
        dbg!();
        let mut next = Some(e);
        while let Some(in_scope) = next {
            if let Ok((_, mut ranking)) = rankings.get_mut(in_scope) {
                // The new entity is within the scope of this ranking.

                // First check if the entity is already ranked. This will happen
                // when loading a world. Do not push the entity to the top rank
                // if it already has a rank.
                dbg!();
                if ranking.entities.iter().find(|check| **check == e).is_none() {
                    dbg!();
                    ranking.entities.push(e);
                }
            }

            next = parents.get(in_scope).ok().map(|p| p.get());
        }
    }

    for e in &moved_entities {
        if new_entities.contains(e) {
            // Ignore newly added entities because those are managed up above
            continue;
        }

        // If an entity's parent changes then the scope of its ranking may
        // change as well.
        let mut remain_in_scope = HashSet::new();
        let mut next = Some(e);
        while let Some(in_scope) = next {
            if let Ok((_, mut ranking)) = rankings.get_mut(in_scope) {
                remain_in_scope.insert(in_scope);
                if ranking.entities.iter().find(|check| **check == e).is_none() {
                    // The ranking does not already contain the moved entity, so
                    // we should push it to the top.
                    dbg!();
                    ranking.entities.push(e);
                }
            }

            next = parents.get(in_scope).ok().map(|p| p.get());
        }

        for (e_ranking, mut ranking) in &mut rankings {
            if !remain_in_scope.contains(&e_ranking) {
                // The entity is not supposed to remain in this scope, so remove
                // it if it is present.
                ranking.entities.retain(|check| *check != e);
            }
        }
    }

    for e in newly_suppressed_entities.iter().chain(no_longer_relevant.iter()) {
        for (_, mut ranking) in &mut rankings {
            ranking.entities.retain(|check| *check != e);
        }
    }

    for ChangeRank { of, by, .. } in rank_changes.iter() {
        let mut next = Some(*of);
        while let Some(in_scope) = next {
            if let Ok((_, mut ranking)) = rankings.get_mut(in_scope) {
                match by {
                    RankAdjustment::Delta(delta) => {
                        if let Some(original_rank) = ranking.entities.iter().position(|e| *e == *of) {
                            ranking.entities.retain(|e| *e != *of);
                            let new_rank = (original_rank as i64 + *delta).max(0) as usize;
                            if new_rank < ranking.entities.len() {
                                dbg!();
                                ranking.entities.insert(new_rank, *of);
                            } else {
                                dbg!();
                                ranking.entities.push(*of);
                            }
                        }
                    }
                    RankAdjustment::ToRank(pos) => {
                        ranking.entities.retain(|e| *e != *of);
                        if *pos < ranking.entities.len() {
                            dbg!();
                            ranking.entities.insert(*pos, *of);
                        } else {
                            dbg!();
                            ranking.entities.push(*of);
                        }
                    }
                }
            }

            next = parents.get(in_scope).ok().map(|p| p.get());
        }
    }
}
