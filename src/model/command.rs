use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{Argument, BuildError, Example, Flag};

/// A handler function that can be registered on a [`Command`].
///
/// Uses `Arc` so that `Command: Clone` (refcount bump, no deep copy).
/// HRTB (`for<'a>`) allows the handler to be called with a `ParsedCommand`
/// of any lifetime.
pub type HandlerFn = Arc<
    dyn for<'a> Fn(&ParsedCommand<'a>) -> Result<(), Box<dyn std::error::Error>> + Send + Sync,
>;

/// The result of successfully parsing an invocation against a [`Command`].
#[derive(Debug)]
pub struct ParsedCommand<'a> {
    pub command: &'a Command,
    pub args: HashMap<String, String>,
    pub flags: HashMap<String, String>,
}

/// A command in the registry, potentially with subcommands.
#[derive(Clone, Serialize, Deserialize)]
pub struct Command {
    pub canonical: String,
    pub aliases: Vec<String>,
    pub spellings: Vec<String>,
    pub summary: String,
    pub description: String,
    pub arguments: Vec<Argument>,
    pub flags: Vec<Flag>,
    pub examples: Vec<Example>,
    pub subcommands: Vec<Command>,
    pub best_practices: Vec<String>,
    pub anti_patterns: Vec<String>,
    #[serde(skip)]
    pub handler: Option<HandlerFn>,
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("canonical", &self.canonical)
            .field("aliases", &self.aliases)
            .field("spellings", &self.spellings)
            .field("summary", &self.summary)
            .field("description", &self.description)
            .field("arguments", &self.arguments)
            .field("flags", &self.flags)
            .field("examples", &self.examples)
            .field("subcommands", &self.subcommands)
            .field("best_practices", &self.best_practices)
            .field("anti_patterns", &self.anti_patterns)
            .field("handler", &self.handler.as_ref().map(|_| "<handler>"))
            .finish()
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.canonical == other.canonical
            && self.aliases == other.aliases
            && self.spellings == other.spellings
            && self.summary == other.summary
            && self.description == other.description
            && self.arguments == other.arguments
            && self.flags == other.flags
            && self.examples == other.examples
            && self.subcommands == other.subcommands
            && self.best_practices == other.best_practices
            && self.anti_patterns == other.anti_patterns
    }
}

impl Command {
    pub fn builder(canonical: impl Into<String>) -> CommandBuilder {
        CommandBuilder {
            canonical: canonical.into(),
            aliases: Vec::new(),
            spellings: Vec::new(),
            summary: String::new(),
            description: String::new(),
            arguments: Vec::new(),
            flags: Vec::new(),
            examples: Vec::new(),
            subcommands: Vec::new(),
            best_practices: Vec::new(),
            anti_patterns: Vec::new(),
            handler: None,
        }
    }

    /// All strings this command can be matched by (canonical + aliases + spellings), lowercased.
    pub(crate) fn matchable_strings(&self) -> Vec<String> {
        let mut v = vec![self.canonical.to_lowercase()];
        v.extend(self.aliases.iter().map(|s| s.to_lowercase()));
        v.extend(self.spellings.iter().map(|s| s.to_lowercase()));
        v
    }
}

/// Consuming builder for [`Command`].
pub struct CommandBuilder {
    canonical: String,
    aliases: Vec<String>,
    spellings: Vec<String>,
    summary: String,
    description: String,
    arguments: Vec<Argument>,
    flags: Vec<Flag>,
    examples: Vec<Example>,
    subcommands: Vec<Command>,
    best_practices: Vec<String>,
    anti_patterns: Vec<String>,
    handler: Option<HandlerFn>,
}

impl CommandBuilder {
    pub fn aliases(mut self, aliases: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.aliases = aliases.into_iter().map(Into::into).collect();
        self
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }

    pub fn spellings(mut self, spellings: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.spellings = spellings.into_iter().map(Into::into).collect();
        self
    }

    pub fn spelling(mut self, spelling: impl Into<String>) -> Self {
        self.spellings.push(spelling.into());
        self
    }

    pub fn summary(mut self, s: impl Into<String>) -> Self {
        self.summary = s.into();
        self
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }

    pub fn argument(mut self, arg: Argument) -> Self {
        self.arguments.push(arg);
        self
    }

    pub fn flag(mut self, flag: Flag) -> Self {
        self.flags.push(flag);
        self
    }

    pub fn example(mut self, example: Example) -> Self {
        self.examples.push(example);
        self
    }

    pub fn subcommand(mut self, cmd: Command) -> Self {
        self.subcommands.push(cmd);
        self
    }

    pub fn best_practice(mut self, bp: impl Into<String>) -> Self {
        self.best_practices.push(bp.into());
        self
    }

    pub fn anti_pattern(mut self, ap: impl Into<String>) -> Self {
        self.anti_patterns.push(ap.into());
        self
    }

    pub fn handler(mut self, h: HandlerFn) -> Self {
        self.handler = Some(h);
        self
    }

    pub fn build(self) -> Result<Command, BuildError> {
        if self.canonical.trim().is_empty() {
            return Err(BuildError::EmptyCanonical);
        }
        Ok(Command {
            canonical: self.canonical,
            aliases: self.aliases,
            spellings: self.spellings,
            summary: self.summary,
            description: self.description,
            arguments: self.arguments,
            flags: self.flags,
            examples: self.examples,
            subcommands: self.subcommands,
            best_practices: self.best_practices,
            anti_patterns: self.anti_patterns,
            handler: self.handler,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Argument, Flag};

    fn make_simple_cmd() -> Command {
        Command::builder("run")
            .alias("r")
            .spelling("RUN")
            .summary("Run something")
            .description("Runs the thing.")
            .build()
            .unwrap()
    }

    #[test]
    fn test_builder_happy_path() {
        let cmd = make_simple_cmd();
        assert_eq!(cmd.canonical, "run");
        assert_eq!(cmd.aliases, vec!["r"]);
        assert_eq!(cmd.spellings, vec!["RUN"]);
    }

    #[test]
    fn test_builder_empty_canonical() {
        assert_eq!(
            Command::builder("").build().unwrap_err(),
            BuildError::EmptyCanonical
        );
        assert_eq!(
            Command::builder("   ").build().unwrap_err(),
            BuildError::EmptyCanonical
        );
    }

    #[test]
    fn test_partial_eq_ignores_handler() {
        let cmd1 = Command::builder("run").build().unwrap();
        let mut cmd2 = cmd1.clone();
        cmd2.handler = Some(Arc::new(|_| Ok(())));
        assert_eq!(cmd1, cmd2);
    }

    #[test]
    fn test_serde_round_trip_skips_handler() {
        let cmd = Command::builder("deploy")
            .summary("Deploy the app")
            .argument(
                Argument::builder("env")
                    .description("target env")
                    .required()
                    .build()
                    .unwrap(),
            )
            .flag(
                Flag::builder("dry-run")
                    .description("dry run mode")
                    .build()
                    .unwrap(),
            )
            .handler(Arc::new(|_| Ok(())))
            .build()
            .unwrap();

        let json = serde_json::to_string(&cmd).unwrap();
        let de: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, de);
        assert!(de.handler.is_none());
    }

    #[test]
    fn test_matchable_strings() {
        let cmd = Command::builder("Git")
            .alias("g")
            .spelling("GIT")
            .build()
            .unwrap();
        let matchables = cmd.matchable_strings();
        assert!(matchables.contains(&"git".to_string()));
        assert!(matchables.contains(&"g".to_string()));
        assert!(matchables.contains(&"git".to_string())); // spelling lowercased
    }

    #[test]
    fn test_clone_shares_handler() {
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();
        let cmd = Command::builder("x")
            .handler(Arc::new(move |_| {
                called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }))
            .build()
            .unwrap();
        let cmd2 = cmd.clone();
        assert!(std::sync::Arc::ptr_eq(
            cmd.handler.as_ref().unwrap(),
            cmd2.handler.as_ref().unwrap()
        ));
    }
}
