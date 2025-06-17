use rand::seq::SliceRandom;
use rand::thread_rng;
use uuid::Uuid;

const ADJECTIVES: &[&str] = &[
    "blue", "red", "green", "happy", "fast", "clever", "silly", "witty", "brave", "shiny",
];

const NOUNS: &[&str] = &[
    "badger", "fox", "mountain", "river", "sky", "computer", "ocean", "unicorn", "dragon", "star",
];

pub fn generate_random_project_name() -> String {
    let mut rng = thread_rng();
    let adjective = ADJECTIVES.choose(&mut rng).unwrap_or(&"Default");
    let noun = NOUNS.choose(&mut rng).unwrap_or(&"Project");

    format!(
        "{}{}",
        capitalize_first_letter(adjective),
        capitalize_first_letter(noun)
    )
}

fn capitalize_first_letter(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub fn generate_random_token() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid; // For parsing the token back to Uuid

    #[test]
    fn test_generate_random_project_name() {
        let name = generate_random_project_name();
        assert!(!name.is_empty(), "Generated name should not be empty");
        assert!(
            name.chars().any(|c| c.is_uppercase()),
            "Generated name should contain an uppercase letter for PascalCase"
        );
        // A more robust check could be to see if it starts with an uppercase,
        // and if there's another uppercase in the middle if both ADJECTIVES and NOUNS are not empty.
        // For simplicity, the current check is fine.
    }

    #[test]
    fn test_generate_random_token() {
        let token = generate_random_token();
        assert!(!token.is_empty(), "Generated token should not be empty");
        assert_eq!(
            token.len(),
            36,
            "Generated token should be 36 characters long (UUID string format)"
        );
        assert!(
            Uuid::parse_str(&token).is_ok(),
            "Generated token should be a valid UUID"
        );
    }
}
