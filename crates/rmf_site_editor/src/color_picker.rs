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
        static mut IDX: usize = 0;
        let color = unsafe {
            let color = DEFAULT_COLORS[IDX];
            IDX += 1;
            if IDX == DEFAULT_COLORS.len() {
                IDX = 0
            }
            color
        };
        return color;
    }
}
