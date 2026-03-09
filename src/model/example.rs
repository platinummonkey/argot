use serde::{Deserialize, Serialize};

/// A usage example for a command.
///
/// Examples are rendered in help output and Markdown documentation pages. Each
/// example has a short human-readable description, the command string as it
/// would be typed, and an optional expected output snippet.
///
/// # Examples
///
/// ```
/// # use argot_cmd::Example;
/// let ex = Example::new("list all items", "myapp list")
///     .with_output("item1\nitem2");
///
/// assert_eq!(ex.description, "list all items");
/// assert_eq!(ex.command, "myapp list");
/// assert_eq!(ex.output.as_deref(), Some("item1\nitem2"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Example {
    /// Short description of what the example demonstrates.
    pub description: String,
    /// The full command string as it would be typed by the user.
    pub command: String,
    /// Optional expected output shown below the command in help and documentation.
    pub output: Option<String>,
}

impl Example {
    /// Create a new [`Example`] with a description and command string.
    ///
    /// The `output` field is `None` by default; use [`Example::with_output`]
    /// to attach expected output.
    ///
    /// # Arguments
    ///
    /// - `description` — Short explanation of what the example shows.
    /// - `command` — The command string as typed (including the program name).
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot_cmd::Example;
    /// let ex = Example::new("deploy to staging", "myapp deploy staging");
    /// assert_eq!(ex.command, "myapp deploy staging");
    /// assert!(ex.output.is_none());
    /// ```
    pub fn new(description: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            command: command.into(),
            output: None,
        }
    }

    /// Attach an expected output snippet to this example.
    ///
    /// The output is rendered as a comment in plain-text help and as a code
    /// block annotation in Markdown documentation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot_cmd::Example;
    /// let ex = Example::new("check version", "myapp --version")
    ///     .with_output("myapp 1.0.0");
    ///
    /// assert_eq!(ex.output.as_deref(), Some("myapp 1.0.0"));
    /// ```
    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output = Some(output.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_new() {
        let ex = Example::new("run it", "mycmd run");
        assert_eq!(ex.description, "run it");
        assert_eq!(ex.command, "mycmd run");
        assert!(ex.output.is_none());
    }

    #[test]
    fn test_example_with_output() {
        let ex = Example::new("run it", "mycmd run").with_output("done");
        assert_eq!(ex.output.as_deref(), Some("done"));
    }

    #[test]
    fn test_example_serde_round_trip() {
        let ex = Example::new("desc", "cmd").with_output("out");
        let json = serde_json::to_string(&ex).unwrap();
        let de: Example = serde_json::from_str(&json).unwrap();
        assert_eq!(ex, de);
    }
}
