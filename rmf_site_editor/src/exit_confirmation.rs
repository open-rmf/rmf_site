use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use bevy_egui::{egui, EguiContexts};

#[derive(Resource, Default)]
pub struct QuitDialog {
    pub visible: bool,
}

pub struct QuitPlugin;

impl Plugin for QuitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuitDialog>()
            .add_systems(Update, handle_quit_requests)
            .add_systems(Update, show_quit_dialog);
    }
}

fn handle_quit_requests(
    mut quit_dialog: ResMut<QuitDialog>,
    mut close_events: EventReader<WindowCloseRequested>,
) {
    for _ in close_events.read() {
        quit_dialog.visible = true;
    }
}

fn show_quit_dialog(
    mut contexts: EguiContexts,
    mut quit_dialog: ResMut<QuitDialog>,
    mut app_exit: EventWriter<AppExit>,
) {
    if !quit_dialog.visible {
        return;
    }
    egui::Window::new("Exit Confirmation")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut(), |ui| {
            ui.label("Are you sure you want to quit?");
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Yes").clicked() {
                    app_exit.send(AppExit);
                }
                if ui.button("Cancel").clicked() {
                    quit_dialog.visible = false;
                }
            });
        });
}
