pub fn greeting(name: &str) -> String {
    let name = name.trim();
    format!("Hello, {}!", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_trims_whitespace() {
        assert_eq!(greeting(" Astrid "), "Hello, Astrid!");
    }

    #[test]
    fn required_greeting_file_matches() {
        let text = std::fs::read_to_string("GREETING.txt")
            .expect("create GREETING.txt containing Hello, Astrid! followed by a newline");
        assert_eq!(text, format!("{}\n", greeting(" Astrid ")));
    }
}
