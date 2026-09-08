//! Widgets to view and playback simulations using for the [rmf_site_editor](https://github.com/open-rmf/rmf_site).

use crate::interaction::egui::{
    SimulationOverview, SimulationPlaybackEventTable, SimulationPlaybackMenu,
    SimulationPlaybackSelector,
};
use crate::playback::{SimulationPlaybackCommand, SimulationPlaybackView};
use crate::simulation::Simulation;
use bevy::ecs::system::SystemParam;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::egui::{self, Ui};
use egui::Frame;
use rmf_site_egui::{Tile, WidgetSystem};
use rmf_site_format::NameInSite;

/// Vertical spacing between sections of a tile.
const SECTION_SPACING: f32 = 4.0;

/// A tile for displaying all available simulations and their states.
#[derive(SystemParam)]
pub struct SimulationOverviewTile<'w, 's> {
    simulations: Query<'w, 's, (Entity, &'static NameInSite, &'static Simulation)>,
}

impl<'w, 's> SimulationOverviewTile<'w, 's> {
    fn show_overview(&self, ui: &mut Ui) {
        SimulationOverview::new(
            self.simulations
                .iter()
                .map(|(entity, name, simulation)| (entity, name.as_str(), simulation)),
        )
        .show(ui);
    }
}

impl<'w, 's> WidgetSystem<Tile> for SimulationOverviewTile<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let params = state.get_mut(world);
        show_collapsible_section(ui, "Simulations", |ui| params.show_overview(ui));
    }
}

/// A panel to select and control playback for simulations.
#[derive(SystemParam)]
pub struct SimulationPlaybackTile<'w, 's> {
    playback: SimulationPlaybackView<'w, 's>,
    playback_commands: EventWriter<'w, SimulationPlaybackCommand>,
    simulations: Query<'w, 's, (Entity, &'static NameInSite), With<Simulation>>,
}

impl<'w, 's> SimulationPlaybackTile<'w, 's> {
    fn show_playback(&mut self, ui: &mut Ui) {
        self.show_selector(ui);
        self.show_active_playback(ui);
    }

    fn show_selector(&mut self, ui: &mut Ui) {
        let active_simulation = self
            .playback
            .active()
            .map(|active| active.playback.simulation_entity());
        SimulationPlaybackSelector::new(
            self.simulations
                .iter()
                .map(|(entity, name)| (entity, name.as_str())),
            active_simulation,
        )
        .show(ui, &mut self.playback_commands);
    }

    fn show_active_playback(&mut self, ui: &mut Ui) {
        let Some(active) = self.playback.active() else {
            return;
        };

        let commands = &mut self.playback_commands;
        ui.add_space(SECTION_SPACING);
        SimulationPlaybackMenu::new(active).show(ui, commands);
        ui.add_space(SECTION_SPACING);
        Frame::group(ui.style()).show(ui, |ui| {
            SimulationPlaybackEventTable::new(active).show(ui, commands);
        });
    }
}

impl<'w, 's> WidgetSystem<Tile> for SimulationPlaybackTile<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        show_collapsible_section(ui, "Playback", |ui| {
            params.show_playback(ui);
        });
    }
}

/// Shows a titled, collapsible section of a tile, followed by a separator.
pub fn show_collapsible_section(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui)) {
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, add_contents);
    ui.separator();
}
