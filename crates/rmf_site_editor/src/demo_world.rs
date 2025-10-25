pub fn demo_office() -> Vec<u8> {
    return include_str!("../assets/office.building.yaml")
        .as_bytes()
        .to_vec();
}
