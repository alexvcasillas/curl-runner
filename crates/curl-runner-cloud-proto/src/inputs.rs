//! Validation of a collection's declared inputs.

use std::collections::HashSet;
use std::fmt;

use crate::wire::Input;

pub const MAX_INPUT_NAME_CHARS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    EmptyName,
    NameTooLong(String),
    BadName(String),
    DuplicateName(String),
    /// A secret input with a default, which would publish the secret.
    SecretDefault(String),
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => f.write_str("an input has an empty name"),
            Self::NameTooLong(n) => write!(
                f,
                "input {n} has a name longer than {MAX_INPUT_NAME_CHARS} characters"
            ),
            Self::BadName(n) => write!(
                f,
                "input {n:?} must use letters, digits and underscores, and not start with a digit"
            ),
            Self::DuplicateName(n) => write!(f, "input {n} is declared more than once"),
            Self::SecretDefault(n) => write!(
                f,
                "input {n} is secret, so it cannot have a default. Set it where the collection runs"
            ),
        }
    }
}

impl std::error::Error for InputError {}

/// `^[A-Za-z_][A-Za-z0-9_]*$`
fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn validate_inputs(inputs: &[Input]) -> Result<(), InputError> {
    let mut seen = HashSet::with_capacity(inputs.len());
    for i in inputs {
        if i.name.is_empty() {
            return Err(InputError::EmptyName);
        }
        if i.name.chars().count() > MAX_INPUT_NAME_CHARS {
            return Err(InputError::NameTooLong(i.name.clone()));
        }
        if !valid_name(&i.name) {
            return Err(InputError::BadName(i.name.clone()));
        }
        if !seen.insert(i.name.as_str()) {
            return Err(InputError::DuplicateName(i.name.clone()));
        }
        if i.secret && i.default.is_some() {
            return Err(InputError::SecretDefault(i.name.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str) -> Input {
        Input {
            name: name.into(),
            description: None,
            secret: false,
            required: false,
            locked: false,
            default: None,
        }
    }

    #[test]
    fn accepts_valid_inputs() {
        let mut secret = input("API_KEY");
        secret.secret = true;
        let mut plain = input("base_url");
        plain.default = Some("https://api.example.com".into());
        assert_eq!(
            validate_inputs(&[secret, plain, input("_x1"), input(&"A".repeat(128))]),
            Ok(())
        );
    }

    #[test]
    fn rejects_bad_inputs() {
        let mut secret = input("API_KEY");
        secret.secret = true;
        secret.default = Some("sk_live_123".into());
        let long = "A".repeat(129);
        let cases = [
            (vec![input("")], InputError::EmptyName),
            (vec![input(&long)], InputError::NameTooLong(long.clone())),
            (vec![input("1ABC")], InputError::BadName("1ABC".into())),
            (vec![input("A-B")], InputError::BadName("A-B".into())),
            (vec![input("A B")], InputError::BadName("A B".into())),
            (vec![input("ÄPI")], InputError::BadName("ÄPI".into())),
            (
                vec![input("TOKEN"), input("TOKEN")],
                InputError::DuplicateName("TOKEN".into()),
            ),
            (vec![secret], InputError::SecretDefault("API_KEY".into())),
        ];
        for (inputs, want) in cases {
            assert_eq!(validate_inputs(&inputs), Err(want));
        }
    }
}
