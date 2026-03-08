use thiserror::Error;

use crate::model::{Command, Example};

/// Errors produced by [`Registry`] methods.
#[derive(Debug, Error)]
pub enum QueryError {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Owns the registered command tree and provides query/search operations.
pub struct Registry {
    commands: Vec<Command>,
}

impl Registry {
    pub fn new(commands: Vec<Command>) -> Self {
        Self { commands }
    }

    /// Borrow the raw command slice (useful for constructing a [`Parser`][crate::parser::Parser]).
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    pub fn list_commands(&self) -> Vec<&Command> {
        self.commands.iter().collect()
    }

    pub fn get_command(&self, canonical: &str) -> Option<&Command> {
        self.commands.iter().find(|c| c.canonical == canonical)
    }

    /// Walk a dot-separated path of canonical names.
    /// `path = &["remote", "add"]` returns the `add` subcommand of `remote`.
    pub fn get_subcommand(&self, path: &[&str]) -> Option<&Command> {
        if path.is_empty() {
            return None;
        }
        let mut current = self.get_command(path[0])?;
        for &segment in &path[1..] {
            current = current
                .subcommands
                .iter()
                .find(|c| c.canonical == segment)?;
        }
        Some(current)
    }

    pub fn get_examples(&self, canonical: &str) -> Option<&[Example]> {
        self.get_command(canonical).map(|c| c.examples.as_slice())
    }

    /// Substring search across canonical name, summary, and description.
    pub fn search(&self, query: &str) -> Vec<&Command> {
        let q = query.to_lowercase();
        self.commands
            .iter()
            .filter(|c| {
                c.canonical.to_lowercase().contains(&q)
                    || c.summary.to_lowercase().contains(&q)
                    || c.description.to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn to_json(&self) -> Result<String, QueryError> {
        serde_json::to_string_pretty(&self.commands).map_err(QueryError::Serialization)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Command;

    fn registry() -> Registry {
        let sub = Command::builder("push")
            .summary("Push changes")
            .build()
            .unwrap();
        let remote = Command::builder("remote")
            .summary("Manage remotes")
            .subcommand(sub)
            .build()
            .unwrap();
        let list = Command::builder("list")
            .summary("List all items in the store")
            .build()
            .unwrap();
        Registry::new(vec![remote, list])
    }

    #[test]
    fn test_list_commands() {
        let r = registry();
        let cmds = r.list_commands();
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn test_get_command() {
        let r = registry();
        assert!(r.get_command("remote").is_some());
        assert!(r.get_command("missing").is_none());
    }

    #[test]
    fn test_get_subcommand() {
        let r = registry();
        assert_eq!(
            r.get_subcommand(&["remote", "push"]).unwrap().canonical,
            "push"
        );
        assert!(r.get_subcommand(&["remote", "nope"]).is_none());
        assert!(r.get_subcommand(&[]).is_none());
    }

    #[test]
    fn test_get_examples_empty() {
        let r = registry();
        assert_eq!(r.get_examples("list"), Some([].as_slice()));
    }

    #[test]
    fn test_search_match() {
        let r = registry();
        let results = r.search("store");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].canonical, "list");
    }

    #[test]
    fn test_search_no_match() {
        let r = registry();
        assert!(r.search("zzz").is_empty());
    }

    #[test]
    fn test_to_json() {
        let r = registry();
        let json = r.to_json().unwrap();
        assert!(json.contains("remote"));
        assert!(json.contains("list"));
        // Re-parse to verify valid JSON
        let _: serde_json::Value = serde_json::from_str(&json).unwrap();
    }
}
