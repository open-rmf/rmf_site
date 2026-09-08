use crate::time::SimulationTime;
use bevy::prelude::*;
use std::ops::RangeInclusive;
use std::time::Duration;

// TODO(@reuben-thomas): Should something similar be configurable at runtime?
// We may have high or low frequency event simulations.
// Another option would be constraining the range to maximum events / second, or maximum events / frame of 1 based on the active simulation.
/// The allowed playback speed range.
pub const PLAYBACK_SPEED_RANGE: RangeInclusive<f32> = 0.01..=100.0;

/// The playback speed applied when a simulation is activated for playback.
pub const DEFAULT_PLAYBACK_SPEED: f32 = 1.0;

/// Commands for controlling playback of an active Simulation.
#[derive(Event, Clone, Copy, Debug)]
pub enum SimulationPlaybackCommand {
    /// Selects a Simulation for playback.
    SetActiveSimulation(Option<Entity>),
    /// Plays from the current playback time, with the behaviour at end defined
    /// by [`SimulationReplayBehaviour`].
    Play,
    /// Pauses at the current playback time.
    Pause,
    /// Plays if playback is paused, and pauses otherwise.
    TogglePlayPause,
    /// Sets the speed multiplier of playback, clamped to
    /// [`PLAYBACK_SPEED_RANGE`].
    SetSpeed(f32),
    /// Seeks to the given target.
    Seek(SimulationPlaybackSeek),
    /// Seeks to the start of the simulation and pauses.
    SeekToStart,
    /// Seeks to the last computed step of the simulation and pauses.
    SeekToLast,
    /// Sets the behaviour of playback upon reaching the end of the active
    /// simulation.
    SetReplayBehaviour(SimulationReplayBehaviour),
}

/// A seek target for [`SimulationPlaybackCommand::Seek`].
#[derive(Clone, Copy, Debug)]
pub enum SimulationPlaybackSeek {
    /// Seek to the very first step at or after the specified time.
    Time { time: SimulationTime },
    /// Seek so that exactly `applied_steps` steps have been applied.
    Step { applied_steps: usize },
    /// Seek by the given number of steps in the specified direction.
    StepDelta {
        steps: usize,
        direction: SeekDirection,
    },
    /// Seek to the very first step at or after the time after the addition of
    /// the duration offset.
    TimeDelta {
        duration: Duration,
        direction: SeekDirection,
    },
    /// Seek to the last computed step.
    End,
}

/// The direction to seek in [`SimulationPlaybackSeek`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeekDirection {
    Advance,
    Revert,
}

/// The requested seek falls out of simulation's computed steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimulationPlaybackSeekOutOfBounds;

/// Behaviour of replay upon reaching the end of a simulation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SimulationReplayBehaviour(pub Option<Duration>);
