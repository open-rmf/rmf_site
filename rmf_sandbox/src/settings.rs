pub struct Settings {
    pub graphics_quality: GraphicsQuality,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // todo: select based on WASM and GPU (or not)
            graphics_quality: GraphicsQuality::Low,
        }
    }
}

#[derive(PartialEq)]
pub enum GraphicsQuality {
    Low,
    Ultra,
}
