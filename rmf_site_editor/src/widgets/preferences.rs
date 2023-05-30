use bevy::prelude::*;
use bevy_egui::{
    egui::{self, DragValue, Slider},
    EguiContext,
};
use rmf_site_format::{AnchorParams, AssociatedGraphs, Edge, SiteProperties, Preferences};

use crate::site::{should_display_lane, update_lane_visuals, LaneEnds, LaneSegments, SiteAssets};
pub struct PreferenceEvent;
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
    mut site_properties: Query<&mut SiteProperties>,
    mut egui_context: ResMut<EguiContext>,
    mut redraw_lanes: EventWriter<RedrawLanes>,
) {
    if !panel.enable {
        return;
    }

    let mut properties = site_properties.get_single_mut();
    
    if let Ok(props) = properties.as_mut() {
        // If SiteProperties doesn't exist it is probably because it has not loaded 
        if props.preferences.is_none() {
            props.preferences = Some(Default::default());
            println!("Setting default value");
        }

        if let Some(preferences) = props.preferences.as_mut() {
            egui::Window::new("Preferences").show(egui_context.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Default Lane Width: ");
                    let lane_width_entry = DragValue::new(&mut preferences.default_lane_width)
                        .clamp_range(0.0..=10000.0);
                    if ui.add(lane_width_entry).changed() {
                        redraw_lanes.send(RedrawLanes);
                    }
                });
                if ui.button("Close").clicked() {
                    panel.enable = false;
                }
            });
        }
        
    }

}

fn redraw_lanes(
    redraw_lanes: EventReader<RedrawLanes>,
    mut lanes: Query<(Entity, &Edge<Entity>, &LaneSegments)>,
    site_properties: Query<&SiteProperties>,
    anchors: AnchorParams,
    mut transforms: Query<&mut Transform>,
    lane_ends: Query<Entity, With<LaneEnds>>,
) {
    // TODO(arjo): Refactor so no panics
    let preferences = site_properties.get_single().unwrap_or(&Default::default()).preferences.unwrap_or_default();

    if redraw_lanes.len() == 0 {
        return;
    }

    for (e, edge, segments) in &mut lanes {
        update_lane_visuals(e, edge, segments, &anchors, &preferences, &mut transforms);
    }

    for entity in &lane_ends {
        let mut transform = transforms.get_mut(entity);
        if let Ok(mut transform) = transform {
            transform.scale.x = preferences.scale_lane_ends_by();
            transform.scale.y = preferences.scale_lane_ends_by();
        }
    }
}

pub struct PreferencePlugin;

impl Plugin for PreferencePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PreferenceEvent>()
            .add_event::<RedrawLanes>()
            .init_resource::<PreferencePanel>()
            .add_system_to_stage(CoreStage::Update, enable_preference_ui)
            .add_system_to_stage(CoreStage::PostUpdate, draw_preference_ui.label("ui"))
            .add_system_to_stage(CoreStage::PostUpdate, redraw_lanes.after("ui"));
    }
}
