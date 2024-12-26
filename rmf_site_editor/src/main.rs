#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_js() {
    extern crate console_error_panic_hook;
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    librmf_site_editor::run(vec!["web".to_owned()]);
}

fn main() {
    librmf_site_editor::run(std::env::args().collect());
}
