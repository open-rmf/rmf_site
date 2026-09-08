//! [`egui`] widgets for inspecting simulations and controlling playback.

use crate::event::DynDiscreteEvent;
use crate::playback::{
    DEFAULT_PLAYBACK_SPEED, PLAYBACK_SPEED_RANGE, SeekDirection, SimulationActivePlaybackView,
    SimulationPlaybackCommand, SimulationPlaybackSeek, SimulationReplayBehaviour,
};
use crate::simulation::{Simulation, SimulationComputeState, SimulationStep};
use crate::time::SimulationTime;
use bevy::prelude::*;
use bevy_egui::egui::{self, Color32};
use std::collections::BTreeMap;
use std::hash::Hash;
use std::time::Duration;

/// Maximum height of scrollable areas.
pub const SCROLL_AREA_MAX_HEIGHT: f32 = 400.0;

/// Padding around the contents of an event's code block.
const EVENT_CONTENTS_MARGIN: i8 = 4;

/// Margins around a pill.
const PILL_MARGIN: egui::Margin = egui::Margin {
    left: 6,
    right: 6,
    top: 1,
    bottom: 1,
};

/// Corner radius of a pill
const PILL_CORNER_RADIUS: u8 = 255;

/// A dropdown menu for selecting or deselecting a simulation for playback.
pub struct SimulationPlaybackSelector<'a> {
    simulations: Vec<(Entity, &'a str)>,
    active: Option<Entity>,
}

impl<'a> SimulationPlaybackSelector<'a> {
    pub fn new(
        simulations: impl IntoIterator<Item = (Entity, &'a str)>,
        active: Option<Entity>,
    ) -> Self {
        Self {
            simulations: simulations.into_iter().collect(),
            active,
        }
    }

    pub fn show(self, ui: &mut egui::Ui, commands: &mut EventWriter<SimulationPlaybackCommand>) {
        if self.simulations.is_empty() {
            ui.label("No simulations available.");
            return;
        }

        ui.horizontal(|ui| {
            ui.label("Simulation:");
            Self::show_selection_combo_box(ui, &self.simulations, self.active, commands);
            Self::show_unset_button(ui, self.active, commands);
        });
    }

    fn show_selection_combo_box(
        ui: &mut egui::Ui,
        simulations: &[(Entity, &'a str)],
        active: Option<Entity>,
        commands: &mut EventWriter<SimulationPlaybackCommand>,
    ) {
        let mut selection = active;
        egui::ComboBox::from_id_salt("active_playback_simulation")
            .selected_text(Self::selected_text(simulations, active))
            .show_ui(ui, |ui| {
                for (entity, name) in simulations {
                    ui.selectable_value(&mut selection, Some(*entity), *name);
                }
            });

        if selection != active {
            commands.write(SimulationPlaybackCommand::SetActiveSimulation(selection));
        }
    }

    fn selected_text(simulations: &[(Entity, &'a str)], active: Option<Entity>) -> &'a str {
        let Some(active) = active else {
            return "Select a simulation...";
        };

        simulations
            .iter()
            .find(|(entity, _)| *entity == active)
            .map(|(_, name)| *name)
            .expect("Active simulation not found.")
    }

    fn show_unset_button(
        ui: &mut egui::Ui,
        active: Option<Entity>,
        commands: &mut EventWriter<SimulationPlaybackCommand>,
    ) {
        ui.add_enabled_ui(active.is_some(), |ui| {
            if ui.button("❌").on_hover_text("Unload").clicked() {
                commands.write(SimulationPlaybackCommand::SetActiveSimulation(None));
            }
        });
    }
}

/// A menu for controlling simulation playback with a timeline.
pub struct SimulationPlaybackMenu<'a> {
    view: SimulationActivePlaybackView<'a>,
}

impl<'a> SimulationPlaybackMenu<'a> {
    pub fn new(view: SimulationActivePlaybackView<'a>) -> Self {
        Self { view }
    }

    pub fn show(self, ui: &mut egui::Ui, commands: &mut EventWriter<SimulationPlaybackCommand>) {
        let view = self.view;
        Self::show_control_button_row(ui, view, commands);
        Self::show_replay_behaviour_control(ui, view, commands);
        Self::show_timeline(ui, view, commands);
    }

    /// A row of buttons to control play/pause, start/end, and stepping through the simulation.
    fn show_control_button_row(
        ui: &mut egui::Ui,
        view: SimulationActivePlaybackView,
        commands: &mut EventWriter<SimulationPlaybackCommand>,
    ) {
        let can_revert = !view.playback.at_start();
        let can_advance = !view.at_last_available();

        ui.horizontal(|ui| {
            Self::show_command_button(
                ui,
                can_revert,
                "Start",
                "Seek to start",
                SimulationPlaybackCommand::SeekToStart,
                commands,
            );
            Self::show_command_button(
                ui,
                can_revert,
                "Previous",
                "Step backwards",
                SimulationPlaybackCommand::Seek(SimulationPlaybackSeek::StepDelta {
                    steps: 1,
                    direction: SeekDirection::Revert,
                }),
                commands,
            );
            if ui
                .button(if view.playback.is_playing() {
                    "Pause"
                } else {
                    "Play"
                })
                .clicked()
            {
                commands.write(SimulationPlaybackCommand::TogglePlayPause);
            }
            Self::show_command_button(
                ui,
                can_advance,
                "Next",
                "Step forwards",
                SimulationPlaybackCommand::Seek(SimulationPlaybackSeek::StepDelta {
                    steps: 1,
                    direction: SeekDirection::Advance,
                }),
                commands,
            );
            Self::show_command_button(
                ui,
                can_advance,
                "End",
                "Seek to end",
                SimulationPlaybackCommand::SeekToLast,
                commands,
            );
            Self::show_speed_drag_value(ui, view.playback.speed, commands);
        });
    }

    fn show_command_button(
        ui: &mut egui::Ui,
        enabled: bool,
        label: &str,
        hover_text: &str,
        command: SimulationPlaybackCommand,
        commands: &mut EventWriter<SimulationPlaybackCommand>,
    ) {
        if ui
            .add_enabled(enabled, egui::Button::new(label))
            .on_hover_text(hover_text)
            .clicked()
        {
            commands.write(command);
        }
    }

    fn show_speed_drag_value(
        ui: &mut egui::Ui,
        speed: f32,
        commands: &mut EventWriter<SimulationPlaybackCommand>,
    ) {
        let mut speed = speed;
        let changed = ui
            .add(
                egui::DragValue::new(&mut speed)
                    .range(PLAYBACK_SPEED_RANGE)
                    .speed(0.1)
                    .suffix("x"),
            )
            .on_hover_text("Playback speed")
            .changed();
        if changed {
            commands.write(SimulationPlaybackCommand::SetSpeed(speed));
        }

        Self::show_command_button(
            ui,
            speed != DEFAULT_PLAYBACK_SPEED,
            "🔄",
            "Reset playback speed",
            SimulationPlaybackCommand::SetSpeed(DEFAULT_PLAYBACK_SPEED),
            commands,
        );
    }

    /// Show a menu to toggle replay behaviour, and configure pause duration.
    fn show_replay_behaviour_control(
        ui: &mut egui::Ui,
        view: SimulationActivePlaybackView,
        commands: &mut EventWriter<SimulationPlaybackCommand>,
    ) {
        let SimulationReplayBehaviour(pause) = view.playback.replay_behaviour;
        let mut replay = pause.is_some();
        let mut pause_secs = pause.unwrap_or_default().as_secs_f32();

        let mut changed = false;
        ui.horizontal(|ui| {
            changed |= ui.checkbox(&mut replay, "Loop on Completion").changed();
            if replay {
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut pause_secs)
                            .range(0.0_f32..=10.0)
                            .speed(0.1)
                            .suffix("s"),
                    )
                    .on_hover_text("Pause before replaying")
                    .changed();
            }
        });

        if changed {
            commands.write(SimulationPlaybackCommand::SetReplayBehaviour(
                SimulationReplayBehaviour(replay.then(|| Duration::from_secs_f32(pause_secs))),
            ));
        }
    }

    /// Show a slider timeline to control the current playback position.
    fn show_timeline(
        ui: &mut egui::Ui,
        view: SimulationActivePlaybackView,
        commands: &mut EventWriter<SimulationPlaybackCommand>,
    ) {
        let total = view.simulation.duration();
        let end_secs = total.as_secs_f64();
        let mut changed_secs = view.time.elapsed().as_secs_f64();
        let mut changed = false;

        ui.scope(|ui| {
            ui.spacing_mut().slider_width = ui.available_width();
            changed = ui
                .add(
                    egui::Slider::new(&mut changed_secs, 0.0..=end_secs)
                        .show_value(false)
                        .trailing_fill(true),
                )
                .changed();
        });

        let elapsed = Duration::from_secs_f64(changed_secs.clamp(0.0, end_secs));
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.monospace(format!("{elapsed:.2?}/{total:.2?}"));
            });
        });

        if changed {
            commands.write(SimulationPlaybackCommand::Seek(
                SimulationPlaybackSeek::Time {
                    time: SimulationTime::new(Duration::from_secs_f64(changed_secs)),
                },
            ));
        }
    }
}

/// A table for visualizing and seeking to events in the active simulation playback.
pub struct SimulationPlaybackEventTable<'a> {
    view: SimulationActivePlaybackView<'a>,
}

impl<'a> SimulationPlaybackEventTable<'a> {
    pub fn new(view: SimulationActivePlaybackView<'a>) -> Self {
        Self { view }
    }

    pub fn show(self, ui: &mut egui::Ui, commands: &mut EventWriter<SimulationPlaybackCommand>) {
        let steps = self.view.simulation.steps();
        if steps.is_empty() {
            ui.label("No steps computed yet.");
            return;
        }

        let follow = Self::show_follow_checkbox(ui);
        show_scroll_area(ui, "simulation_playback_event_table", |ui| {
            Self::show_steps(
                ui,
                steps,
                self.view.playback.applied_steps,
                follow,
                commands,
            );
        });
    }

    /// A checkbox toggling whether the table scrolls to the current step.
    fn show_follow_checkbox(ui: &mut egui::Ui) -> bool {
        let id = egui::Id::new("simulation_playback_event_table_follow");
        let mut follow = ui.data(|data| data.get_temp(id)).unwrap_or(true);
        ui.checkbox(&mut follow, "Follow")
            .on_hover_text("Automatically scroll to follow the current simulation playback");
        ui.data_mut(|data| data.insert_temp(id, follow));
        follow
    }

    fn show_steps(
        ui: &mut egui::Ui,
        steps: &BTreeMap<SimulationTime, SimulationStep>,
        applied_steps: usize,
        follow: bool,
        commands: &mut EventWriter<SimulationPlaybackCommand>,
    ) {
        egui::Grid::new("simulation_playback_event_table_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Time");
                ui.strong("Step");
                ui.end_row();

                for (index, (time, step)) in steps.iter().enumerate() {
                    let is_current = applied_steps > 0 && index == applied_steps - 1;

                    Self::show_step_time(ui, index, *time, is_current, follow, commands);
                    Self::show_step_events(ui, index, step);
                    ui.end_row();
                }
            });
    }

    fn show_step_time(
        ui: &mut egui::Ui,
        index: usize,
        time: SimulationTime,
        is_current: bool,
        follow: bool,
        commands: &mut EventWriter<SimulationPlaybackCommand>,
    ) {
        let response = ui.selectable_label(
            is_current,
            egui::RichText::new(format!("{:.2?}", time.elapsed())).monospace(),
        );
        if response.clicked() {
            commands.write(SimulationPlaybackCommand::Seek(
                SimulationPlaybackSeek::Step {
                    applied_steps: index + 1,
                },
            ));
        }

        if is_current && follow {
            response.scroll_to_me(Some(egui::Align::Center));
        }
    }

    fn show_step_events(ui: &mut egui::Ui, index: usize, step: &SimulationStep) {
        let event_count = step.event_count();
        if event_count == 1 {
            if let Some(event) = step.events().next() {
                Self::show_event(ui, index, event);
            }
            return;
        }

        egui::CollapsingHeader::new(format!("{event_count} Events"))
            .id_salt(index)
            .show(ui, |ui| {
                for (event_index, event) in step.events().enumerate() {
                    Self::show_event(ui, (index, event_index), event);
                }
            });
    }

    fn show_event(ui: &mut egui::Ui, id_salt: impl Hash, event: &dyn DynDiscreteEvent) {
        egui::CollapsingHeader::new(egui::RichText::new(event.name()).strong())
            .id_salt(id_salt)
            .show(ui, |ui| Self::show_event_contents(ui, event));
    }

    fn show_event_contents(ui: &mut egui::Ui, event: &dyn DynDiscreteEvent) {
        egui::Frame::new()
            .fill(ui.visuals().code_bg_color)
            .corner_radius(ui.visuals().noninteractive().corner_radius)
            .inner_margin(EVENT_CONTENTS_MARGIN)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(egui::RichText::new(format!("{event:#?}")).monospace());
            });
    }
}

/// An overview pane of all simulations within the world.
pub struct SimulationOverview<'a> {
    simulations: Vec<(Entity, &'a str, &'a Simulation)>,
}

impl<'a> SimulationOverview<'a> {
    pub fn new(simulations: impl IntoIterator<Item = (Entity, &'a str, &'a Simulation)>) -> Self {
        Self {
            simulations: simulations.into_iter().collect(),
        }
    }

    pub fn show(self, ui: &mut egui::Ui) {
        let simulations = self.simulations;
        if simulations.is_empty() {
            ui.label("No simulations available.");
            return;
        }

        show_scroll_area(ui, "simulation_cards", |ui| {
            for (entity, name, simulation) in simulations {
                ui.push_id(entity, |ui| {
                    Self::show_simulation(ui, name, simulation);
                });
            }
        });
    }

    fn show_simulation(ui: &mut egui::Ui, name: &str, simulation: &Simulation) {
        let id = ui.make_persistent_id("simulation_card");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.strong(name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    Self::show_compute_state_pill(ui, simulation);
                });
            })
            .body(|ui| Self::show_details(ui, simulation));
    }

    fn show_compute_state_pill(ui: &mut egui::Ui, simulation: &Simulation) {
        let (label, color) =
            Self::compute_state_label(simulation.state(), simulation.compute_elapsed());

        egui::Frame::new()
            .fill(color)
            .corner_radius(PILL_CORNER_RADIUS)
            .inner_margin(PILL_MARGIN)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(label)
                        .color(Color32::BLACK)
                        .small()
                        .strong(),
                );
            });
    }

    fn show_details(ui: &mut egui::Ui, simulation: &Simulation) {
        let steps = simulation.steps();
        let event_count: usize = steps.values().map(|step| step.event_count()).sum();

        ui.label(format!(
            "Extracted entities: {}",
            simulation.init_state().0.entities().len()
        ));
        ui.label(format!("Steps: {}", steps.len()));
        ui.label(format!("Events: {event_count}"));
        if !steps.is_empty() {
            ui.label(format!("Duration: {:.2?}", simulation.duration()));
        }
    }

    fn compute_state_label(
        state: SimulationComputeState,
        elapsed: Duration,
    ) -> (String, egui::Color32) {
        match state {
            SimulationComputeState::Computing => (
                format!("Computing ({elapsed:.2?})"),
                Color32::from_rgb(240, 220, 90),
            ),
            SimulationComputeState::Complete => (
                format!("Computed ({elapsed:.2?})"),
                Color32::from_rgb(100, 240, 100),
            ),
            SimulationComputeState::Failed => (
                format!("Failed ({elapsed:.2?})"),
                Color32::from_rgb(240, 100, 100),
            ),
        }
    }
}

/// Shows contents in a scroll area bounded by [`SCROLL_AREA_MAX_HEIGHT`].
fn show_scroll_area(ui: &mut egui::Ui, id_salt: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::both()
        .id_salt(id_salt)
        .max_height(SCROLL_AREA_MAX_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, add_contents);
}
