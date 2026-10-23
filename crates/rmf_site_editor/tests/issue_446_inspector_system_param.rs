use rmf_site_editor::{
    bevy::{ecs::system::SystemState, prelude::*},
    widgets::inspector::Inspector,
};

#[test]
fn inspector_system_param_initializes() {
    let mut world = World::new();
    let _state = SystemState::<Inspector<'_, '_>>::new(&mut world);
}
