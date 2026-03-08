use serde::{Deserialize, Serialize};

use super::BuildError;

/// A positional argument accepted by a command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Argument {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

/// Consuming builder for [`Argument`].
pub struct ArgumentBuilder {
    name: String,
    description: String,
    required: bool,
    default: Option<String>,
}

impl Argument {
    pub fn builder(name: impl Into<String>) -> ArgumentBuilder {
        ArgumentBuilder {
            name: name.into(),
            description: String::new(),
            required: false,
            default: None,
        }
    }
}

impl ArgumentBuilder {
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn default_value(mut self, d: impl Into<String>) -> Self {
        self.default = Some(d.into());
        self
    }

    pub fn build(self) -> Result<Argument, BuildError> {
        if self.name.trim().is_empty() {
            return Err(BuildError::EmptyCanonical);
        }
        Ok(Argument {
            name: self.name,
            description: self.description,
            required: self.required,
            default: self.default,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCase {
        name: &'static str,
        arg_name: &'static str,
        required: bool,
        expect_err: bool,
    }

    #[test]
    fn test_builder() {
        let cases = vec![
            TestCase { name: "happy path", arg_name: "file", required: false, expect_err: false },
            TestCase { name: "required", arg_name: "file", required: true, expect_err: false },
            TestCase { name: "empty name", arg_name: "", required: false, expect_err: true },
            TestCase { name: "whitespace name", arg_name: "   ", required: false, expect_err: true },
        ];

        for tc in cases {
            let mut b = Argument::builder(tc.arg_name).description("a file");
            if tc.required {
                b = b.required();
            }
            let result = b.build();
            assert_eq!(result.is_err(), tc.expect_err, "case: {}", tc.name);
        }
    }

    #[test]
    fn test_serde_round_trip() {
        let arg = Argument::builder("path")
            .description("target path")
            .required()
            .build()
            .unwrap();
        let json = serde_json::to_string(&arg).unwrap();
        let de: Argument = serde_json::from_str(&json).unwrap();
        assert_eq!(arg, de);
    }
}
