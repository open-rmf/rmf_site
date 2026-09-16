//! Keyboard shortcuts for controlling simulation playback.

use crate::playback::{
    SeekDirection, SimulationPlaybackCommand, SimulationPlaybackSeek, SimulationPlaybackView,
};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Plugin for controlling the active playback simulation with the keyboard.
///
/// Insert a [`SimulationPlaybackKeymap`] to override the default keymap bindings.
pub struct SimulationPlaybackKeyboardPlugin;

impl Plugin for SimulationPlaybackKeyboardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationPlaybackKeymap>()
            .add_systems(Update, playback_keyboard_controls);
    }
}

/// An action that can be bound to a key in [`SimulationPlaybackKeymap`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimulationPlaybackKeyboardAction {
    /// Plays the simulation, or pauses it if it is already playing.
    TogglePlayPause,
    /// Adds `delta` to the current playback speed.
    ChangeSpeed { delta: f32 },
    /// Seeks `steps` simulation steps in `direction`.
    Step {
        steps: usize,
        direction: SeekDirection,
    },
    /// Seeks to the first computed simulation step.
    SeekToStart,
    /// Seeks to the last computed simulation step.
    SeekToEnd,
}

/// The action each key performs during playback.
#[derive(Resource, Clone, Debug, Deref, DerefMut)]
pub struct SimulationPlaybackKeymap(pub HashMap<KeyCode, SimulationPlaybackKeyboardAction>);

impl Default for SimulationPlaybackKeymap {
    fn default() -> Self {
        Self(HashMap::from_iter([
            (
                KeyCode::Space,
                SimulationPlaybackKeyboardAction::TogglePlayPause,
            ),
            (
                KeyCode::Equal,
                SimulationPlaybackKeyboardAction::ChangeSpeed { delta: 0.5 },
            ),
            (
                KeyCode::Minus,
                SimulationPlaybackKeyboardAction::ChangeSpeed { delta: -0.5 },
            ),
            (
                KeyCode::ArrowRight,
                SimulationPlaybackKeyboardAction::Step {
                    steps: 1,
                    direction: SeekDirection::Advance,
                },
            ),
            (
                KeyCode::ArrowLeft,
                SimulationPlaybackKeyboardAction::Step {
                    steps: 1,
                    direction: SeekDirection::Revert,
                },
            ),
            (KeyCode::Home, SimulationPlaybackKeyboardAction::SeekToStart),
            (KeyCode::End, SimulationPlaybackKeyboardAction::SeekToEnd),
        ]))
    }
}

fn playback_keyboard_controls(
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Res<SimulationPlaybackKeymap>,
    playback: SimulationPlaybackView,
    mut commands: EventWriter<SimulationPlaybackCommand>,
) {
    let Some(active) = playback.active() else {
        return;
    };

    for (&key, &action) in keymap.iter() {
        if !keys.just_pressed(key) {
            continue;
        }
        match action {
            SimulationPlaybackKeyboardAction::TogglePlayPause => {
                commands.write(SimulationPlaybackCommand::TogglePlayPause);
            }
            SimulationPlaybackKeyboardAction::ChangeSpeed { delta } => {
                commands.write(SimulationPlaybackCommand::SetSpeed(
                    active.playback.speed + delta,
                ));
            }
            SimulationPlaybackKeyboardAction::Step { steps, direction } => {
                commands.write(SimulationPlaybackCommand::Seek(
                    SimulationPlaybackSeek::StepDelta { steps, direction },
                ));
            }
            SimulationPlaybackKeyboardAction::SeekToStart => {
                commands.write(SimulationPlaybackCommand::SeekToStart);
            }
            SimulationPlaybackKeyboardAction::SeekToEnd => {
                commands.write(SimulationPlaybackCommand::SeekToLast);
            }
        }
    }
}
