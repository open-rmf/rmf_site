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

//! `rmf_site_sim` is a library for performing [discrete-event simulation](https://en.wikipedia.org/wiki/Discrete-event_simulation) (DES) in Bevy,
//! with support for the [rmf_site_editor](https://github.com/open-rmf/rmf_site).
//!
//! # Core Concepts
//!
//! If you're already familiar with DES, the following table summarizes common nomenclature and definitions for DES concepts, as well as how we represent them in `rmf_site_sim`.
//!
//! | DES Concept | Definition | Representation in `rmf_site_sim` |
//! | --- | --- | --- |
//! | Model | An abstract representation of a real-world system being simulated. | A collection of Bevy ECS systems ([`bevy::prelude::System`](bevy::ecs::system::System)) configured in a [`SimulationBuilder`]. |
//! | System | A representation of a single entity type within a model. | [`bevy::prelude::System`](bevy::ecs::system::System) |
//! | State | A state vector describing the state of the world at any time-instance. | A [`simulation::SimulationState`], which wraps a [`bevy::prelude::World`](bevy::ecs::world::World), containing all entities, as well as resource and component values. |
//! | Entity | An instance that requires representation in the model. | [`bevy::prelude::Entity`](bevy::ecs::entity::Entity) |
//! | Attributes | Properties of an entity, or the system as a whole. | [`bevy::prelude::Component`](bevy::ecs::component::Component), [`bevy::prelude::Resource`](bevy::ecs::prelude::Resource) |
//! | Event | An instantaneous occurrence that changes the state of the system. | An [`event::DiscreteEvent`], implemented automatically for any cloneable [`bevy::prelude::Command`](bevy::ecs::system::Command). |
//! | Event Notice | A record of an event that may occur at some simulation time and the necessary parameters to execute it. | Predicted using the [`event::CandidateEventWriter`], [`event::CandidateComponentEventWriter`], and [`event::CandidateResourceEventWriter`] system parameters. |
//! | Future Event List (FEL) | A list of event notices for future events, ordered by time of occurrence. | The [`event::CandidateDiscreteEvents`] resource, which should not be modified directly. |
//! | Clock | A variable representing the current value of simulated time. | The [`time::SimulationClock`] resource. |
//! | Output | Data produced by running the simulation. | A [`SimulationStep`] for each computed simulation time, collected by the [`Simulation`] component. |
//!
//! # Introduction
//!
//! A simulated model is expressed as a set of *prediction systems*, added with
//! [`SimulationBuilder::add_prediction_systems`], which are ordinary [`bevy::prelude::System`](bevy::ecs::system::System)
//! functions. Typically, each prediction system may be used to model the behaviour of a single type of entity, or a set of component types.
//!
//! ## Predictions Systems
//!
//! A prediction system should be a pure function that can perform any non-mutating
//! operation on the world state, and submit mutations as candidate events using the
//! [`CandidateEventWriter`](event::CandidateEventWriter). All prediction systems within a [`Simulation`] will be
//! executed after every event.
//!
//! ## Candidate Events Priority
//!
//! Every candidate event submitted during a run of the [`SimulationPredict`]
//! schedule is ranked, and only the single highest priority candidate is
//! executed, with the others discarded. Priority is determined by:
//!
//! 1. **Time.**: Whichever event is predicted for the earliest time.
//! 2. **Event Priority** (Incoming Feature)
//! 3. **System Order** : Prediction systems are extended to execute in a total order over the [`bevy::ecs::schedule::ScheduleConfigs`] with which they are registered. An event predicted by a system executed earlier has higher priority.
//!
//! # Common Patterns
//!
//! ## Only Make Predictions When Necessary
//!
//! Prediction systems are re-run after every event, which can be expensive.
//! Prefer to make predictions only when an entity is not in its expected state.
//!
//! ```ignore
//! /// Plans a trajectory for robots that do not have one yet.
//! fn planner(robots: Query<(Entity, &Pose, &Goal), Without<Trajectory>>) { /* ... */ }
//! ```
//! If required, an expensive computation can be cached using conventional Bevy [`Component`][`bevy::ecs::component::Component`]s
//! or [`Resource`][`bevy::ecs::prelude::Resource`]s. However, direct world mutation of this kind should not be used in any other way.
//!
//! ## Avoid Direct World Mutation
//!
//! The output of a simulation is the list of events that were executed, and
//! any direct world mutation not expressed through a candidate event will silently
//! not be recorded as such.
//!
//! # Examples
//!
//! Each example is a complete model, in rough order of complexity. Run one with
//! `cargo run --example <name>`.
//!
//! | Example | Description |
//! | --- | --- |
//! | [`client_server`](https://github.com/open-rmf/rmf_site/blob/main/sim/crates/rmf_site_sim/examples/client_server.rs) | A simple headless example of a server responding to client requests. |
//! | [`robot_fleet`](https://github.com/open-rmf/rmf_site/blob/main/sim/crates/rmf_site_sim/examples/robot_fleet.rs) | A fleet of robots navigating to randomly assigned goals, with animation. |
//! | [`assembly_line`](https://github.com/open-rmf/rmf_site/blob/main/sim/crates/rmf_site_sim/examples/assembly_line/main.rs) | An assembly line of mobile robots, conveyors, and processing stations passing products along, with animation. |
//! | [`rmf_site_editor`](https://github.com/open-rmf/rmf_site/blob/main/sim/crates/rmf_site_sim/examples/rmf_site_editor/main.rs) | Integration with the [RMF Site Editor](https://github.com/open-rmf/rmf_site). |
pub mod compute;
pub mod event;
pub mod interaction;
pub mod playback;
pub mod schedule;
pub mod simulation;
pub mod sync;
pub mod time;

pub use schedule::*;
pub use simulation::*;
