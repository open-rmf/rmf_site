use bevy::prelude::error;
use std::sync::Mutex;

static DEFAULT_COLORS: [[f32; 3]; 8] = [
    [1.0, 0.5, 0.3],
    [0.6, 1.0, 0.5],
    [0.6, 0.8, 1.0],
    [0.6, 0.2, 0.3],
    [0.1, 0.0, 1.0],
    [0.8, 0.4, 0.5],
    [0.9, 1.0, 0.0],
    [0.7, 0.5, 0.1],
];

pub struct ColorPicker;

impl ColorPicker {
    pub fn get_color() -> [f32; 3] {
        static IDX: Mutex<usize> = Mutex::new(0);
        let mut lock = IDX.lock();
        let color = if let Ok(ref mut index) = lock {
            let color = DEFAULT_COLORS[**index];
            **index += 1;
            if **index == DEFAULT_COLORS.len() {
                **index = 0;
            }
            color
        } else {
            error!(
                "ColorPicker::get_color - unable to acquire mutex for index, using default rgb of {:?}",
                DEFAULT_COLORS[0]
            );
            DEFAULT_COLORS[0]
        };
        return color;
    }
}
