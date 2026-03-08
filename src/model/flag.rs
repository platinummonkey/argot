use serde::{Deserialize, Serialize};

use super::BuildError;

/// A named flag accepted by a command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flag {
    pub name: String,
    pub short: Option<char>,
    pub description: String,
    pub required: bool,
    pub takes_value: bool,
    pub default: Option<String>,
}

/// Consuming builder for [`Flag`].
pub struct FlagBuilder {
    name: String,
    short: Option<char>,
    description: String,
    required: bool,
    takes_value: bool,
    default: Option<String>,
}

impl Flag {
    pub fn builder(name: impl Into<String>) -> FlagBuilder {
        FlagBuilder {
            name: name.into(),
            short: None,
            description: String::new(),
            required: false,
            takes_value: false,
            default: None,
        }
    }
}

impl FlagBuilder {
    pub fn short(mut self, c: char) -> Self {
        self.short = Some(c);
        self
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn takes_value(mut self) -> Self {
        self.takes_value = true;
        self
    }

    pub fn default_value(mut self, d: impl Into<String>) -> Self {
        self.default = Some(d.into());
        self
    }

    pub fn build(self) -> Result<Flag, BuildError> {
        if self.name.trim().is_empty() {
            return Err(BuildError::EmptyCanonical);
        }
        Ok(Flag {
            name: self.name,
            short: self.short,
            description: self.description,
            required: self.required,
            takes_value: self.takes_value,
            default: self.default,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_happy_path() {
        let flag = Flag::builder("verbose")
            .short('v')
            .description("verbose output")
            .build()
            .unwrap();
        assert_eq!(flag.name, "verbose");
        assert_eq!(flag.short, Some('v'));
        assert!(!flag.required);
        assert!(!flag.takes_value);
    }

    #[test]
    fn test_builder_empty_name() {
        assert!(Flag::builder("").build().is_err());
        assert!(Flag::builder("  ").build().is_err());
    }

    #[test]
    fn test_takes_value_with_default() {
        let flag = Flag::builder("output")
            .takes_value()
            .default_value("stdout")
            .build()
            .unwrap();
        assert!(flag.takes_value);
        assert_eq!(flag.default.as_deref(), Some("stdout"));
    }

    #[test]
    fn test_serde_round_trip() {
        let flag = Flag::builder("format")
            .short('f')
            .takes_value()
            .required()
            .build()
            .unwrap();
        let json = serde_json::to_string(&flag).unwrap();
        let de: Flag = serde_json::from_str(&json).unwrap();
        assert_eq!(flag, de);
    }
}
