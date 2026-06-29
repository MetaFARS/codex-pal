pub fn string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string cannot fail")
}

pub fn string_array(values: &[&str]) -> String {
    serde_json::to_string(values).expect("serializing string array cannot fail")
}
