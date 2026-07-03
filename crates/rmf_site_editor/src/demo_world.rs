pub fn demo_office() -> Vec<u8> {
    return include_str!("../assets/office.site.json")
        .as_bytes()
        .to_vec();
}
