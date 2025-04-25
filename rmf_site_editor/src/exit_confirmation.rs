use crate::AppState;
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use bevy_egui::{egui, EguiContexts};

#[derive(Resource, Default)]
pub struct SiteChanged(pub bool);

#[derive(Resource, Default)]
pub struct ExitConfirmationDialog {
    pub visible: bool,
}

pub struct ExitConfirmationPlugin;

impl Plugin for ExitConfirmationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SiteChanged>()
            .init_resource::<ExitConfirmationDialog>()
            .add_systems(Update, handle_exit_requests)
            .add_systems(Update, show_exit_confirmation_dialog);
    }
}

fn handle_exit_requests(
    mut exit_confirmation_dialog: ResMut<ExitConfirmationDialog>,
    mut close_events: EventReader<WindowCloseRequested>,
    mut app_exit: EventWriter<AppExit>,
    site_changed: Res<SiteChanged>,
    app_state: Res<State<AppState>>,
) {
    for _ in close_events.read() {
        if app_state.get() == &AppState::MainMenu {
            app_exit.send(AppExit);
        }

        if site_changed.0 == true {
            exit_confirmation_dialog.visible = true;
        }
    }
}

fn show_exit_confirmation_dialog(
    mut contexts: EguiContexts,
    mut exit_confirmation_dialog: ResMut<ExitConfirmationDialog>,
    mut app_exit: EventWriter<AppExit>,
) {
    if !exit_confirmation_dialog.visible {
        return;
    }
    egui::Window::new("Exit Confirmation")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut(), |ui| {
            ui.label("Are you sure you want to exit?");
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Yes").clicked() {
                    app_exit.send(AppExit);
                }
                if ui.button("Cancel").clicked() {
                    exit_confirmation_dialog.visible = false;
                }
            });
        });
}
