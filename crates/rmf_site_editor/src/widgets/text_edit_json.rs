use bevy_egui::egui;
use serde::{Deserialize, Serialize};

pub struct TextEditJson<'de, T, S> {
    value: &'de mut T,
    value_json: &'de mut S,
    valid: &'de mut bool,
}

impl<'de, T, S> TextEditJson<'de, T, S>
where
    T: Deserialize<'de> + Serialize,
    S: egui::widgets::text_edit::TextBuffer,
{
    pub fn new(value: &'de mut T, value_json: &'de mut S, valid: &'de mut bool) -> Self {
        Self {
            value,
            value_json,
            valid,
        }
    }
}

impl<'de, T, S> egui::Widget for TextEditJson<'de, T, S>
where
    T: Deserialize<'de> + Serialize,
    S: egui::widgets::text_edit::TextBuffer,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let orignal_bg = ui.visuals().extreme_bg_color;
        if !*self.valid {
            ui.visuals_mut().extreme_bg_color = egui::Color32::DARK_RED;
        }
        let resp = ui.text_edit_multiline(self.value_json);
        if resp.changed() {
            match serde_json::from_str::<T>(self.value_json.as_str()) {
                Ok(value) => {
                    *self.value = value;
                    *self.valid = true;
                }
                Err(_) => {
                    *self.valid = false;
                }
            }
        }
        ui.visuals_mut().extreme_bg_color = orignal_bg;
        resp
    }
}
