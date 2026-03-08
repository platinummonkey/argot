//! Central command registry with lookup and search operations.
//!
//! [`Registry`] is the primary store for the command tree in an argot
//! application. It owns a `Vec<Command>` and exposes several query methods:
//!
//! - **[`Registry::get_command`]** — exact lookup by canonical name.
//! - **[`Registry::get_subcommand`]** — walk a path of canonical names into
//!   the nested subcommand tree.
//! - **[`Registry::list_commands`]** — iterate all top-level commands.
//! - **[`Registry::search`]** — case-insensitive substring search across
//!   canonical name, summary, and description.
//! - **[`Registry::fuzzy_search`]** — fuzzy (skim) search returning results
//!   sorted by score (best match first).
//! - **[`Registry::to_json`]** — serialize the command tree to pretty-printed
//!   JSON (handler closures are excluded).
//!
//! Pass `registry.commands()` to [`crate::Parser::new`] to wire the registry
//! into the parsing pipeline.
//!
//! # Example
//!
//! ```
//! # use argot::{Command, Registry};
//! let registry = Registry::new(vec![
//!     Command::builder("list").summary("List all items").build().unwrap(),
//!     Command::builder("get").summary("Get a single item").build().unwrap(),
//! ]);
//!
//! assert!(registry.get_command("list").is_some());
//! assert_eq!(registry.search("item").len(), 2);
//! ```

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use thiserror::Error;

use crate::model::{Command, Example};

/// Errors produced by [`Registry`] methods.
#[derive(Debug, Error)]
pub enum QueryError {
    /// JSON serialization failed.
    ///
    /// Wraps the underlying [`serde_json::Error`].
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Owns the registered command tree and provides query/search operations.
///
/// Create a `Registry` with [`Registry::new`], passing the fully-built list of
/// top-level commands. The registry takes ownership of the command list and
/// makes it available through a variety of lookup and search methods.
///
/// # Examples
///
/// ```
/// # use argot::{Command, Registry};
/// let registry = Registry::new(vec![
///     Command::builder("deploy").summary("Deploy the app").build().unwrap(),
/// ]);
///
/// let cmd = registry.get_command("deploy").unwrap();
/// assert_eq!(cmd.summary, "Deploy the app");
/// ```
pub struct Registry {
    commands: Vec<Command>,
}

impl Registry {
    /// Create a new `Registry` owning the given command list.
    ///
    /// # Arguments
    ///
    /// - `commands` — The top-level command list. Subcommands are nested
    ///   inside the respective [`Command::subcommands`] fields.
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot::{Command, Registry};
    /// let registry = Registry::new(vec![
    ///     Command::builder("run").build().unwrap(),
    /// ]);
    /// assert_eq!(registry.list_commands().len(), 1);
    /// ```
    pub fn new(commands: Vec<Command>) -> Self {
        Self { commands }
    }

    /// Borrow the raw command slice (useful for constructing a [`Parser`][crate::parser::Parser]).
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot::{Command, Registry, Parser};
    /// let registry = Registry::new(vec![Command::builder("ping").build().unwrap()]);
    /// let parser = Parser::new(registry.commands());
    /// let parsed = parser.parse(&["ping"]).unwrap();
    /// assert_eq!(parsed.command.canonical, "ping");
    /// ```
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Return references to all top-level commands.
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot::{Command, Registry};
    /// let registry = Registry::new(vec![
    ///     Command::builder("a").build().unwrap(),
    ///     Command::builder("b").build().unwrap(),
    /// ]);
    /// assert_eq!(registry.list_commands().len(), 2);
    /// ```
    pub fn list_commands(&self) -> Vec<&Command> {
        self.commands.iter().collect()
    }

    /// Look up a top-level command by its exact canonical name.
    ///
    /// Returns `None` if no command with that canonical name exists. Does not
    /// match aliases or spellings — use [`crate::Resolver`] for fuzzy/prefix
    /// matching.
    ///
    /// # Arguments
    ///
    /// - `canonical` — The exact canonical name to look up.
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot::{Command, Registry};
    /// let registry = Registry::new(vec![
    ///     Command::builder("deploy").alias("d").build().unwrap(),
    /// ]);
    ///
    /// assert!(registry.get_command("deploy").is_some());
    /// assert!(registry.get_command("d").is_none()); // alias, not canonical
    /// ```
    pub fn get_command(&self, canonical: &str) -> Option<&Command> {
        self.commands.iter().find(|c| c.canonical == canonical)
    }

    /// Walk a path of canonical names into the subcommand tree.
    ///
    /// `path = &["remote", "add"]` returns the `add` subcommand of `remote`.
    /// Each path segment must be an *exact canonical* name at that level of
    /// the tree.
    ///
    /// Returns `None` if any segment fails to match or if `path` is empty.
    ///
    /// # Arguments
    ///
    /// - `path` — Ordered slice of canonical command names from top-level down.
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot::{Command, Registry};
    /// let registry = Registry::new(vec![
    ///     Command::builder("remote")
    ///         .subcommand(Command::builder("add").build().unwrap())
    ///         .build()
    ///         .unwrap(),
    /// ]);
    ///
    /// let sub = registry.get_subcommand(&["remote", "add"]).unwrap();
    /// assert_eq!(sub.canonical, "add");
    ///
    /// assert!(registry.get_subcommand(&[]).is_none());
    /// assert!(registry.get_subcommand(&["remote", "nope"]).is_none());
    /// ```
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

    /// Return the examples slice for a top-level command, or `None` if the
    /// command does not exist.
    ///
    /// An empty examples list returns `Some(&[])`.
    ///
    /// # Arguments
    ///
    /// - `canonical` — The exact canonical name of the command.
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot::{Command, Example, Registry};
    /// let registry = Registry::new(vec![
    ///     Command::builder("run")
    ///         .example(Example::new("basic run", "myapp run"))
    ///         .build()
    ///         .unwrap(),
    /// ]);
    ///
    /// assert_eq!(registry.get_examples("run").unwrap().len(), 1);
    /// assert!(registry.get_examples("missing").is_none());
    /// ```
    pub fn get_examples(&self, canonical: &str) -> Option<&[Example]> {
        self.get_command(canonical).map(|c| c.examples.as_slice())
    }

    /// Substring search across canonical name, summary, and description.
    ///
    /// The search is case-insensitive. Returns all top-level commands for
    /// which the query appears in at least one of the three text fields.
    ///
    /// # Arguments
    ///
    /// - `query` — The substring to search for (case-insensitive).
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot::{Command, Registry};
    /// let registry = Registry::new(vec![
    ///     Command::builder("list").summary("List all records").build().unwrap(),
    ///     Command::builder("get").summary("Get a single record").build().unwrap(),
    /// ]);
    ///
    /// let results = registry.search("record");
    /// assert_eq!(results.len(), 2);
    /// assert!(registry.search("zzz").is_empty());
    /// ```
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

    /// Fuzzy search across canonical name, summary, and description.
    ///
    /// Uses the skim fuzzy-matching algorithm. Returns matches sorted
    /// descending by score (best match first). Commands that produce no fuzzy
    /// match are excluded.
    ///
    /// # Arguments
    ///
    /// - `query` — The fuzzy query string.
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot::{Command, Registry};
    /// let registry = Registry::new(vec![
    ///     Command::builder("list").summary("List all items").build().unwrap(),
    ///     Command::builder("get").summary("Get an item").build().unwrap(),
    /// ]);
    ///
    /// let results = registry.fuzzy_search("lst");
    /// assert!(!results.is_empty());
    /// // Results are sorted best-first; scores are non-negative i64 values.
    /// for window in results.windows(2) {
    ///     assert!(window[0].1 >= window[1].1);
    /// }
    /// ```
    pub fn fuzzy_search(&self, query: &str) -> Vec<(&Command, i64)> {
        let matcher = SkimMatcherV2::default();
        let mut results: Vec<(&Command, i64)> = self
            .commands
            .iter()
            .filter_map(|cmd| {
                let text = format!("{} {} {}", cmd.canonical, cmd.summary, cmd.description);
                matcher.fuzzy_match(&text, query).map(|score| (cmd, score))
            })
            .collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results
    }

    /// Serialize the entire command tree to a pretty-printed JSON string.
    ///
    /// Handler closures are excluded from the output (they are skipped by the
    /// `serde` configuration on [`Command`]).
    ///
    /// # Errors
    ///
    /// Returns [`QueryError::Serialization`] if `serde_json` fails (in
    /// practice this should not happen for well-formed command trees).
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot::{Command, Registry};
    /// let registry = Registry::new(vec![
    ///     Command::builder("deploy").summary("Deploy").build().unwrap(),
    /// ]);
    ///
    /// let json = registry.to_json().unwrap();
    /// assert!(json.contains("deploy"));
    /// ```
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
    fn test_fuzzy_search_match() {
        let r = registry();
        let results = r.fuzzy_search("lst");
        assert!(!results.is_empty());
        assert!(results.iter().any(|(cmd, _)| cmd.canonical == "list"));
    }

    #[test]
    fn test_fuzzy_search_no_match() {
        let r = registry();
        assert!(r.fuzzy_search("zzzzz").is_empty());
    }

    #[test]
    fn test_fuzzy_search_sorted_by_score() {
        let exact = Command::builder("list")
            .summary("List all items")
            .build()
            .unwrap();
        let weak = Command::builder("remote")
            .summary("Manage remotes")
            .build()
            .unwrap();
        let r = Registry::new(vec![weak, exact]);
        let results = r.fuzzy_search("list");
        assert!(!results.is_empty());
        assert_eq!(results[0].0.canonical, "list");
        for window in results.windows(2) {
            assert!(window[0].1 >= window[1].1);
        }
    }

    #[test]
    fn test_to_json() {
        let r = registry();
        let json = r.to_json().unwrap();
        assert!(json.contains("remote"));
        assert!(json.contains("list"));
        let _: serde_json::Value = serde_json::from_str(&json).unwrap();
    }
}
