use librmf_site_editor::run;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run_js() {
    extern crate console_error_panic_hook;
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    run(vec![]);
}
