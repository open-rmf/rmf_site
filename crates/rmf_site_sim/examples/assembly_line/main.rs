//! In this example, we simulate an assembly line consisting of autonomous
//! mobile robots (AMRs), conveyors, and processing stations. These entities
//!
//!
//! ```text
//! ┌────────┐  ┌───────┐  ┌───────────┐  ┌────────────┐
//! │ area_1 │  │ amr_1 │  │ station_1 │  │ conveyor_1 │
//! └───┬────┘  └───┬───┘  └─────┬─────┘  └──────┬─────┘
//!     │           │            │               │
//!     │ PassProduct(i)         │               │
//!     ├──────────►│            │               │
//!     │ PassProduct(i + 1)     │               │
//!     ├──────────►│            │               │
//!     │           │ MoveTo(station_1)          │
//!     │           ├──┐         │               │
//!     │           │◄─┘ ArriveAt                │
//!     │           │ PassProduct(i)             │
//!     │           ├───────────►│               │
//!     │           │            │ ArriveAt(i)   │
//!     │           │            ├──┐            │
//!     │           │            │◄─┘            │
//!     │           │            │ PassProduct(i)│
//!     │           │            ├──────────────►│
//!     │           │            │               │ ArriveAt(i)
//!     │           │            │               ├──┐
//!     │           │            │               │◄─┘
//!     │           │            │               │
//! ```
//!

use bevy::prelude::*;
use bevy::sprite::Anchor;
use rmf_site_sim::SimulationPlugin;
use rmf_site_sim::interaction::keyboard::SimulationPlaybackKeyboardPlugin;
use rmf_site_sim::playback::{
    SimulationPlaybackCommand, SimulationPlaybackPlugin, SimulationReplayBehaviour,
};
use rmf_site_sim::time::{SimulationClock, SimulationTime};
use std::collections::HashMap;
use std::time::Duration;

mod simulation;
mod visualization;

use simulation::*;
use visualization::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            SimulationPlugin,
            SimulationPlaybackPlugin,
            SimulationPlaybackKeyboardPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}
