use super::plugin::{SimulationActivePlayback, SimulationPlayback};
use crate::simulation::Simulation;
use crate::time::{SimulationClock, SimulationTime};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// A helper [`SystemParam`] to view the active playback and its corresponding
/// simulation.
#[derive(SystemParam)]
pub struct SimulationPlaybackView<'w, 's> {
    playback: Res<'w, SimulationPlayback>,
    clock: Option<Res<'w, SimulationClock>>,
    simulations: Query<'w, 's, &'static Simulation>,
}

impl SimulationPlaybackView<'_, '_> {
    pub fn active(&self) -> Option<SimulationActivePlaybackView<'_>> {
        let playback = self.playback.active_playback()?;
        let simulation = self.simulations.get(playback.simulation_entity()).ok()?;
        let clock = self.clock.as_ref()?;
        Some(SimulationActivePlaybackView {
            playback,
            simulation,
            time: clock.now(),
        })
    }
}

/// The active playback and its corresponding simulation.
#[derive(Clone, Copy)]
pub struct SimulationActivePlaybackView<'a> {
    pub playback: &'a SimulationActivePlayback,
    pub simulation: &'a Simulation,
    pub time: SimulationTime,
}

impl SimulationActivePlaybackView<'_> {
    /// Whether at the last available computed step, when computation of the
    /// simulation has yet to complete.
    pub fn at_last_available(&self) -> bool {
        self.playback.at_last_available(self.simulation)
    }

    /// Whether at the end of a simulation that has finished computation, and
    /// all available steps have been applied.
    pub fn at_end(&self) -> bool {
        self.playback.at_end(self.simulation)
    }
}
