pub fn convert_to_number(s: &str) -> u8 {
    match s.parse::<u8>() {
        Ok(number) => number,
        Err(_e) => 0,
    }
}
