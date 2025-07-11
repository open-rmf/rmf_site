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
use rmf_site_format::SiteID;
use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
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
        Self {
            entities: default(),
            _ignore: default(),
        }
    }

    pub fn from_entities(entities: Vec<Entity>) -> Self {
        Self {
            entities,
            _ignore: default(),
        }
    }

    pub fn entities(&self) -> &Vec<Entity> {
        &self.entities
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

#[derive(Debug, Clone, Copy, Component)]
pub struct RecencyRank<T: Component> {
    rank: usize,
    out_of: usize,
    _ignore: PhantomData<T>,
}

impl<T: Component> PartialEq for RecencyRank<T> {
    fn eq(&self, other: &Self) -> bool {
        self.rank.eq(&other.rank)
    }
}

impl<T: Component> Eq for RecencyRank<T> {}

impl<T: Component> PartialOrd for RecencyRank<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.rank.partial_cmp(&other.rank)
    }
}

impl<T: Component> Ord for RecencyRank<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank.cmp(&other.rank)
    }
}

impl<T: Component> RecencyRank<T> {
    fn new(rank: usize, out_of: usize) -> Self {
        Self {
            rank,
            out_of,
            _ignore: default(),
        }
    }

    pub fn rank(&self) -> usize {
        self.rank
    }

    pub fn out_of(&self) -> usize {
        self.out_of
    }

    pub fn proportion(&self) -> f32 {
        self.rank() as f32 / self.out_of() as f32
    }
}

#[derive(Debug, Clone, Copy, Event)]
pub struct ChangeRank<T: Component> {
    of: Entity,
    by: RankAdjustment,
    _ignore: PhantomData<T>,
}

impl<T: Component> ChangeRank<T> {
    pub fn new(of: Entity, by: RankAdjustment) -> Self {
        Self {
            of,
            by,
            _ignore: default(),
        }
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
    /// Set the entity's rank to the top position.
    ToTop,
    /// Set the entity's rank to the bottom position.
    ToBottom,
}

impl RankAdjustment {
    pub fn label(&self) -> &'static str {
        match self {
            RankAdjustment::Delta(v) => {
                if *v > 0 {
                    "Move up"
                } else if *v < 0 {
                    "Move down"
                } else {
                    "Move nowhere"
                }
            }
            RankAdjustment::ToTop => "Move to top",
            RankAdjustment::ToBottom => "Move to bottom",
        }
    }
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
        app.add_event::<ChangeRank<T>>().add_systems(
            Update,
            (
                update_recency_rankings::<T>,
                update_recency_ranks::<T>.after(update_recency_rankings::<T>),
            ),
        );
    }
}

fn update_recency_rankings<T: Component>(
    mut rankings: Query<(Entity, &mut RecencyRanking<T>)>,
    new_entities: Query<Entity, (Added<T>, Without<SuppressRecencyRank>)>,
    moved_entities: Query<Entity, (Changed<ChildOf>, With<T>, Without<SuppressRecencyRank>)>,
    newly_suppressed_entities: Query<Entity, (With<T>, Added<SuppressRecencyRank>)>,
    mut unsuppressed_entities: RemovedComponents<SuppressRecencyRank>,
    mut no_longer_relevant: RemovedComponents<T>,
    child_of: Query<&ChildOf>,
    mut rank_changes: EventReader<ChangeRank<T>>,
) {
    for e in new_entities.iter().chain(unsuppressed_entities.read()) {
        let mut next = Some(e);
        while let Some(in_scope) = next {
            if let Ok((_, mut ranking)) = rankings.get_mut(in_scope) {
                // The new entity is within the scope of this ranking.

                // First check if the entity is already ranked. This will happen
                // when loading a world. Do not push the entity to the top rank
                // if it already has a rank.
                if ranking.entities.iter().find(|check| **check == e).is_none() {
                    ranking.entities.push(e);
                }
            }

            next = child_of.get(in_scope).ok().map(|co| co.parent());
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
                    ranking.entities.push(e);
                }
            }

            next = child_of.get(in_scope).ok().map(|co| co.parent());
        }

        for (e_ranking, mut ranking) in &mut rankings {
            if !remain_in_scope.contains(&e_ranking) {
                // The entity is not supposed to remain in this scope, so remove
                // it if it is present.
                ranking.entities.retain(|check| *check != e);
            }
        }
    }

    for e in newly_suppressed_entities
        .iter()
        .chain(no_longer_relevant.read())
    {
        for (_, mut ranking) in &mut rankings {
            ranking.entities.retain(|check| *check != e);
        }
    }

    for ChangeRank { of, by, .. } in rank_changes.read() {
        let mut next = Some(*of);
        while let Some(in_scope) = next {
            if let Ok((_, mut ranking)) = rankings.get_mut(in_scope) {
                match by {
                    RankAdjustment::Delta(delta) => {
                        if let Some(original_rank) = ranking.entities.iter().position(|e| *e == *of)
                        {
                            ranking.entities.retain(|e| *e != *of);
                            let new_rank = (original_rank as i64 + *delta).max(0) as usize;
                            if new_rank < ranking.entities.len() {
                                ranking.entities.insert(new_rank, *of);
                            } else {
                                ranking.entities.push(*of);
                            }
                        }
                    }
                    RankAdjustment::ToTop => {
                        ranking.entities.retain(|e| *e != *of);
                        ranking.entities.push(*of);
                    }
                    RankAdjustment::ToBottom => {
                        ranking.entities.retain(|e| *e != *of);
                        ranking.entities.insert(0, *of);
                    }
                }
            }

            next = child_of.get(in_scope).ok().map(|co| co.parent());
        }
    }
}

fn update_recency_ranks<T: Component>(
    mut commands: Commands,
    rankings: Query<&RecencyRanking<T>, Changed<RecencyRanking<T>>>,
    mut ranks: Query<&mut RecencyRank<T>>,
) {
    for ranking in &rankings {
        let out_of = ranking.len();
        for (rank, e) in ranking.iter().enumerate() {
            if let Ok(mut r) = ranks.get_mut(*e) {
                r.rank = rank;
            } else {
                commands
                    .entity(*e)
                    .insert(RecencyRank::<T>::new(rank, out_of));
            }
        }
    }
}
