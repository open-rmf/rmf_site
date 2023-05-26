use bevy::prelude::*;
use bevy_egui::{
    egui::{self, DragValue, Slider},
    EguiContext,
};
use rmf_site_format::{AnchorParams, AssociatedGraphs, Edge};

use crate::site::{should_display_lane, update_lane_visuals, LaneEnds, LaneSegments, SiteAssets};
pub struct PreferenceEvent;

const LANE_DEFAULT_SIZE: f32 = 0.5;

#[derive(Debug, Clone, Resource)]
pub struct PreferenceParameters {
    pub default_lane_width: f32,
    /// TODO(arjo): scale anchors as well.
    pub default_anchor_size: f32,
}

impl PreferenceParameters {
    pub fn scale_lane_ends_by(&self) -> f32 {
        self.default_lane_width / LANE_DEFAULT_SIZE
    }
}

impl Default for PreferenceParameters {
    fn default() -> Self {
        Self {
            default_lane_width: LANE_DEFAULT_SIZE,
            default_anchor_size: 0.25,
        }
    }
}

#[derive(Debug, Clone, Resource, Default)]
pub struct PreferencePanel {
    enable: bool,
}

fn enable_preference_ui(
    mut events: EventReader<PreferenceEvent>,
    mut panel: ResMut<PreferencePanel>,
) {
    for event in events.iter() {
        panel.enable = true;
    }
}

struct RedrawLanes;

fn draw_preference_ui(
    mut panel: ResMut<PreferencePanel>,
    mut preference_paramters: ResMut<PreferenceParameters>,
    mut egui_context: ResMut<EguiContext>,
    mut redraw_lanes: EventWriter<RedrawLanes>,
) {
    if !panel.enable {
        return;
    }

    egui::Window::new("Preferences").show(egui_context.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label("Default Lane Width: ");
            let lane_width_entry = DragValue::new(&mut preference_paramters.default_lane_width)
                .clamp_range(0.0..=10000.0);
            if ui.add(lane_width_entry).changed() {
                redraw_lanes.send(RedrawLanes);
            }
        });
        if ui.button("Done").clicked() {
            panel.enable = false;
            redraw_lanes.send(RedrawLanes);
        }
    });
}

fn redraw_lanes(
    redraw_lanes: EventReader<RedrawLanes>,
    mut lanes: Query<(Entity, &Edge<Entity>, &LaneSegments)>,
    preferences: Res<PreferenceParameters>,
    mut assets: ResMut<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    anchors: AnchorParams,
    mut transforms: Query<&mut Transform>,
    lane_ends: Query<Entity, With<LaneEnds>>,
) {
    if redraw_lanes.len() == 0 {
        return;
    }

    for (e, edge, segments) in &mut lanes {
        update_lane_visuals(e, edge, segments, &anchors, &preferences, &mut transforms);
    }

    for entity in &lane_ends {
        let mut transform = transforms.get_mut(entity);
        if let Ok(mut transform) = transform {
            transform.scale.x = preferences.default_lane_width / LANE_DEFAULT_SIZE;
            transform.scale.y = preferences.default_lane_width / LANE_DEFAULT_SIZE;
        }
    }
}

pub struct PreferencePlugin;

impl Plugin for PreferencePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PreferenceEvent>()
            .add_event::<RedrawLanes>()
            .init_resource::<PreferencePanel>()
            .init_resource::<PreferenceParameters>()
            .add_system_to_stage(CoreStage::Update, enable_preference_ui)
            .add_system_to_stage(CoreStage::PostUpdate, draw_preference_ui.label("ui"))
            .add_system_to_stage(CoreStage::PostUpdate, redraw_lanes.after("ui"));
    }
}
