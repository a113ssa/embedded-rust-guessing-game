pub fn convert_to_number(s: &str) -> u8 {
    s.parse::<u8>().unwrap_or_default()
}
