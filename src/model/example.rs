use serde::{Deserialize, Serialize};

/// A usage example for a command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Example {
    pub description: String,
    pub command: String,
    pub output: Option<String>,
}

impl Example {
    pub fn new(description: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            command: command.into(),
            output: None,
        }
    }

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
