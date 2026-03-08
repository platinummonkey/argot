use serde::{Deserialize, Serialize};

use super::BuildError;

/// A positional argument accepted by a command.
///
/// Arguments are bound in declaration order when the parser consumes tokens
/// after the command (and any subcommand) has been identified. Optional
/// arguments may carry a default value that is substituted when the argument
/// is absent.
///
/// Use [`Argument::builder`] to construct instances.
///
/// # Examples
///
/// ```
/// # use argot::Argument;
/// let arg = Argument::builder("target")
///     .description("Deployment target environment")
///     .required()
///     .build()
///     .unwrap();
///
/// assert_eq!(arg.name, "target");
/// assert!(arg.required);
/// assert!(arg.default.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Argument {
    /// The canonical name of this argument, used as the key in [`crate::ParsedCommand::args`].
    pub name: String,
    /// Human-readable description shown in help output.
    pub description: String,
    /// Whether the parser returns an error when this argument is absent.
    pub required: bool,
    /// Value substituted when the argument is not provided (optional arguments only).
    pub default: Option<String>,
    /// Whether this argument consumes all remaining tokens (must be the last argument).
    pub variadic: bool,
}

/// Consuming builder for [`Argument`].
///
/// Obtain via [`Argument::builder`]. Call [`ArgumentBuilder::build`] when done.
///
/// # Examples
///
/// ```
/// # use argot::Argument;
/// let arg = Argument::builder("format")
///     .description("Output format")
///     .default_value("text")
///     .build()
///     .unwrap();
///
/// assert_eq!(arg.default.as_deref(), Some("text"));
/// assert!(!arg.required);
/// ```
pub struct ArgumentBuilder {
    name: String,
    description: String,
    required: bool,
    default: Option<String>,
    variadic: bool,
}

impl Argument {
    /// Create a new [`ArgumentBuilder`] with the given name.
    ///
    /// # Arguments
    ///
    /// - `name` — The argument name. Must be non-empty after trimming
    ///   (enforced by [`ArgumentBuilder::build`]).
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot::Argument;
    /// let arg = Argument::builder("file").build().unwrap();
    /// assert_eq!(arg.name, "file");
    /// ```
    pub fn builder(name: impl Into<String>) -> ArgumentBuilder {
        ArgumentBuilder {
            name: name.into(),
            description: String::new(),
            required: false,
            default: None,
            variadic: false,
        }
    }
}

impl ArgumentBuilder {
    /// Set the human-readable description for this argument.
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }

    /// Mark this argument as required.
    ///
    /// The parser will return [`crate::ParseError::MissingArgument`] if the
    /// argument is absent from the invocation.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Set the default value used when this argument is not provided.
    ///
    /// A default value is only meaningful for optional (non-required) arguments.
    /// If the argument is required *and* has a default, the default is still
    /// stored but the parser will still require the argument to be supplied.
    pub fn default_value(mut self, d: impl Into<String>) -> Self {
        self.default = Some(d.into());
        self
    }

    /// Mark this argument as variadic (consumes all remaining tokens).
    ///
    /// A variadic argument must be the last argument defined on the command.
    /// [`crate::CommandBuilder::build`] enforces this constraint and returns
    /// [`crate::BuildError::VariadicNotLast`] if a variadic argument is
    /// followed by another argument.
    pub fn variadic(mut self) -> Self {
        self.variadic = true;
        self
    }

    /// Consume the builder and return an [`Argument`].
    ///
    /// # Errors
    ///
    /// Returns [`BuildError::EmptyCanonical`] if the argument name is empty or
    /// consists entirely of whitespace.
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot::{Argument, BuildError};
    /// assert!(Argument::builder("env").build().is_ok());
    /// assert_eq!(Argument::builder("").build().unwrap_err(), BuildError::EmptyCanonical);
    /// ```
    pub fn build(self) -> Result<Argument, BuildError> {
        if self.name.trim().is_empty() {
            return Err(BuildError::EmptyCanonical);
        }
        Ok(Argument {
            name: self.name,
            description: self.description,
            required: self.required,
            default: self.default,
            variadic: self.variadic,
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
            TestCase {
                name: "happy path",
                arg_name: "file",
                required: false,
                expect_err: false,
            },
            TestCase {
                name: "required",
                arg_name: "file",
                required: true,
                expect_err: false,
            },
            TestCase {
                name: "empty name",
                arg_name: "",
                required: false,
                expect_err: true,
            },
            TestCase {
                name: "whitespace name",
                arg_name: "   ",
                required: false,
                expect_err: true,
            },
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
