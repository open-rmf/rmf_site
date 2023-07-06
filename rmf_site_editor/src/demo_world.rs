pub fn demo_office() -> Vec<u8> {
    return include_str!("../../assets/demo_maps/office.building.yaml")
        .as_bytes()
        .to_vec();
}

pub fn demo_workcell() -> Vec<u8> {
    return include_str!("../../assets/demo_workcells/demo.workcell.json")
        .as_bytes()
        .to_vec();
}

pub fn empty_site() -> Vec<u8> {
    return include_str!("../../assets/empty.site.ron")
        .as_bytes()
        .to_vec();
}
