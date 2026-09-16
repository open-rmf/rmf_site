//! This is an example of integrating this crate with an existing Bevy application,
//! using the RMF Site Editor. There are four main models, represented as prediction
//! systems within this simulation with the following functions:
//!
//! - `request_generator` Sets a task to active at their requested time.
//! - `planner` Plans trajectories for robots assigned to these tasks.
//! - `robot` Moves a robot along its assigned trajectory, but holds at a distance for any obstacle.
//!    If the obstacle is a door, it will command it to open and close as it passes.
//! - `door` Moves a door between its closed and open states on command.
//!
//!     ┌───────────────────┐     ┌─────────┐     ┌───────┐     ┌──────┐
//!     │ request_generator │     │ planner │     │ robot │     │ door │
//!     └─────────┬─────────┘     └────┬────┘     └───┬───┘     └───┬──┘
//!               │                    │              │             │
//!               │ TaskState::Active  │              │             │
//!               ├───────────────────►│              │             │
//!               │                    │              │             │
//!               │                    │ AssignRobotTrajectory      │
//!               │                    ├─────────────►│             │
//!               │                    │              │             │
//!               │                    │              │ Pose        │
//!               │                    │              ├──┐          │
//!               │                    │              │  │          │
//!               │                    │              │◄─┘          │
//!               │                    │              │             │
//!               │                    │              │ DoorCommand::Open
//!               │                    │              ├────────────►│
//!               │                    │              │             │
//!               │                    │              │ RobotTrajectory::hold
//!               │                    │              ├──┐          │
//!               │                    │              │  │          │
//!               │                    │              │◄─┘          │
//!               │                    │              │             │
//!               │                    │              │             │ DoorState::Opening
//!               │                    │              │             ├──┐
//!               │                    │              │             │  │
//!               │                    │              │             │◄─┘
//!               │                    │              │             │
//!               │                    │              │ DoorState::Open
//!               │                    │              │◄────────────┤
//!               │                    │              │             │
//!               │                    │              │ RobotTrajectory::resume
//!               │                    │              ├──┐          │
//!               │                    │              │  │          │
//!               │                    │              │◄─┘          │
//!               │                    │              │             │
//!               │                    │              │ Pose        │
//!               │                    │              ├──┐          │
//!               │                    │              │  │          │
//!               │                    │              │◄─┘          │
//!               │                    │              │             │
//!               │                    │              │ DoorCommand::Close
//!               │                    │              ├────────────►│
//!               │                    │              │             │
//!               │                    │              │             │ DoorState::Closing
//!               │                    │              │             ├──┐
//!               │                    │              │             │  │
//!               │                    │              │             │◄─┘
//!               │                    │              │             │
//!               │                    │              │             │ DoorState::Closed
//!               │                    │              │             ├──┐
//!               │                    │              │             │  │
//!               │                    │              │             │◄─┘
//!               │                    │              │             │
//!               │                    │              │ TaskState::Complete
//!               │                    │              ├──┐          │
//!               │                    │              │  │          │
//!               │                    │              │◄─┘          │
//!               │                    │              │             │
//!
use bevy::{
    ecs::query::QueryData,
    ecs::system::{Command, SystemParam, SystemState},
    prelude::*,
};
use rmf_site_editor::SiteEditor;
use rmf_site_editor::color_picker::ColorPicker;
use rmf_site_editor::layers::ZLayer;
use rmf_site_editor::occupancy::{Cell, Grid};
use rmf_site_editor::site::{
    Affiliation, Angle, Bottom, CircleCollision, CurrentLevel, DifferentialDrive, DoorMarker,
    DoorType, Edge, GoToPlace, LevelHeightParam, LocationTags, NameInSite, Point, Pose, Robot,
    Rotation, SiteAssets, Task, TaskParams, Top, find_door_position_tfs, line_stroke_transform,
};
use rmf_site_sim::event::{CandidateComponentEventWriter, CandidateEventWriter};
use rmf_site_sim::playback::SimulationPlaybackPlugin;
use rmf_site_sim::time::SimulationClock;
use rmf_site_sim::time::SimulationTime;
use rmf_site_sim::{SimulationBuilder, SimulationPlugin};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

mod mapf;
mod simulation;
mod ui;
mod visualization;

use simulation::*;
use ui::SimulationUiPlugin;
use visualization::*;

fn main() {
    App::new()
        .add_plugins((SiteEditor::default(), DiscreteEventSimulationPlugin))
        .run();
}

#[derive(Default)]
struct DiscreteEventSimulationPlugin;

impl Plugin for DiscreteEventSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SimulationPlugin,
            SimulationPlaybackPlugin,
            SimulationUiPlugin,
        ));
    }
}
