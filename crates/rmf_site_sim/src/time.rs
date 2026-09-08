use bevy::prelude::*;
use std::ops::{Add, Sub};
use std::time::Duration;

/// A point in simulated time within a simulation, measuring the elapsed
/// duration since the beginning of the simulation, [`SimulationTime::zero`].
///
/// This type is only useful with [`Duration`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimulationTime(Duration);

impl SimulationTime {
    /// Create a new `SimulationTime` from the given elapsed duration.
    pub const fn new(elapsed: Duration) -> Self {
        Self(elapsed)
    }

    /// The start of a simulation, when no time has elapsed.
    pub const fn zero() -> Self {
        Self(Duration::ZERO)
    }

    /// The duration that elapsed since the start of the simulation.
    pub fn elapsed(&self) -> Duration {
        self.0
    }
}

impl Add<Duration> for SimulationTime {
    type Output = SimulationTime;

    /// # Panics
    ///
    /// Panics if the resulting time overflows, as [`Duration`] addition does.
    fn add(self, duration: Duration) -> Self {
        Self(self.0 + duration)
    }
}

impl Sub<Duration> for SimulationTime {
    type Output = SimulationTime;

    /// # Panics
    ///
    /// Panics if `duration` is longer than the time elapsed so far, as
    /// [`Duration`] subtraction does.
    fn sub(self, duration: Duration) -> Self {
        Self(self.0 - duration)
    }
}

impl From<Duration> for SimulationTime {
    fn from(elapsed: Duration) -> Self {
        Self(elapsed)
    }
}

impl From<SimulationTime> for Duration {
    fn from(time: SimulationTime) -> Self {
        time.0
    }
}

/// The current simulation time.
///
/// This resource may be read to get the current time by prediction systems during the
/// computation of a simulation, or visualization systems during playback.
#[derive(Resource, Debug, Default)]
pub struct SimulationClock {
    now: SimulationTime,
}

impl SimulationClock {
    /// The current simulation time.
    pub fn now(&self) -> SimulationTime {
        self.now
    }

    /// Advances the clock to `time`.
    ///
    /// # Panics
    ///
    /// Panics if `time` is earlier than the current time.
    pub(crate) fn advance_to(&mut self, time: SimulationTime) {
        assert!(
            time >= self.now,
            "Tried to advance to time {time:?} that is earlier than the current time {:?}.",
            self.now
        );
        self.now = time;
    }

    /// Sets the clock to `time`.
    pub(crate) fn set_to(&mut self, time: SimulationTime) {
        self.now = time;
    }
}
