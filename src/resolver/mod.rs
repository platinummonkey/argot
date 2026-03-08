//! String-to-command resolution with prefix and ambiguity detection.
//!
//! The resolver implements a three-phase algorithm:
//!
//! 1. **Normalize** — trim whitespace and lowercase the input.
//! 2. **Exact match** — check the input against every command's canonical
//!    name, aliases, and spellings. Return immediately if exactly one matches.
//! 3. **Prefix match** — check which commands have at least one matchable
//!    string that *starts with* the normalized input. If exactly one command
//!    matches, return it. If more than one matches, return
//!    [`ResolveError::Ambiguous`]. If none match, return
//!    [`ResolveError::Unknown`].
//!
//! This algorithm allows users (and agents) to type unambiguous prefixes like
//! `dep` instead of `deploy` while still producing clear errors when a prefix
//! is shared by multiple commands.
//!
//! When a command cannot be found, the resolver also computes up to three
//! "did you mean?" suggestions based on Levenshtein edit distance (≤ 2) or
//! substring containment, and attaches them to the [`ResolveError::Unknown`]
//! variant.
//!
//! # Example
//!
//! ```
//! # use argot::{Command, Resolver, ResolveError};
//! let cmds = vec![
//!     Command::builder("list").alias("ls").build().unwrap(),
//!     Command::builder("log").build().unwrap(),
//! ];
//!
//! let resolver = Resolver::new(&cmds);
//!
//! // Exact canonical
//! assert_eq!(resolver.resolve("list").unwrap().canonical, "list");
//! // Exact alias
//! assert_eq!(resolver.resolve("ls").unwrap().canonical, "list");
//! // Unambiguous prefix
//! assert_eq!(resolver.resolve("lo").unwrap().canonical, "log");
//! // Ambiguous prefix — "l" matches both "list" and "log"
//! assert!(resolver.resolve("l").is_err());
//! // Near-miss — "lust" is one edit away from "list"
//! match resolver.resolve("lust") {
//!     Err(ResolveError::Unknown { suggestions, .. }) => {
//!         assert!(suggestions.contains(&"list".to_string()));
//!     }
//!     _ => unreachable!(),
//! }
//! ```

use thiserror::Error;

use crate::model::Command;

/// Errors produced by [`Resolver::resolve`].
#[derive(Debug, Error, PartialEq)]
pub enum ResolveError {
    /// The input did not match any registered command. `suggestions` contains
    /// up to three canonically close alternatives determined by edit distance.
    #[error("unknown command: `{input}`")]
    Unknown {
        /// The original (untrimmed) input string.
        input: String,
        /// Up to three canonical names that are close to `input`. May be empty.
        suggestions: Vec<String>,
    },
    /// The input matched more than one command as a prefix, making it
    /// ambiguous. The `candidates` field lists the canonical names of the
    /// matching commands.
    #[error("ambiguous command \"{input}\": could match {candidates:?}")]
    Ambiguous {
        /// The original (untrimmed) input string.
        input: String,
        /// Canonical names of all commands that matched the prefix.
        candidates: Vec<String>,
    },
}

/// Resolves a string token to a [`Command`] in a slice, supporting aliases,
/// spellings, and unambiguous prefix matching.
///
/// Create a resolver by passing a slice of commands to [`Resolver::new`], then
/// call [`Resolver::resolve`] with a raw string token. The returned reference
/// borrows from the original command slice (lifetime `'a`).
///
/// # Examples
///
/// ```
/// # use argot::{Command, Resolver};
/// let cmds = vec![
///     Command::builder("deploy").alias("d").build().unwrap(),
///     Command::builder("delete").build().unwrap(),
/// ];
/// let resolver = Resolver::new(&cmds);
///
/// // Exact match via alias
/// assert_eq!(resolver.resolve("d").unwrap().canonical, "deploy");
/// // Prefix "del" is unambiguous
/// assert_eq!(resolver.resolve("del").unwrap().canonical, "delete");
/// ```
pub struct Resolver<'a> {
    commands: &'a [Command],
}

impl<'a> Resolver<'a> {
    /// Create a new `Resolver` over the given command slice.
    ///
    /// # Arguments
    ///
    /// - `commands` — The slice of commands to resolve against. The lifetime
    ///   `'a` is propagated to the references returned by [`Resolver::resolve`].
    pub fn new(commands: &'a [Command]) -> Self {
        Self { commands }
    }

    /// Resolve `input` against the registered commands.
    ///
    /// Resolution order:
    /// 1. Normalize: trim + lowercase.
    /// 2. Exact match across canonical/aliases/spellings → return immediately.
    /// 3. Prefix match → return if exactly one command matches; else `Ambiguous`.
    /// 4. No match → `Unknown`.
    ///
    /// # Arguments
    ///
    /// - `input` — The raw string to resolve (trimming and lowercasing are
    ///   applied internally).
    ///
    /// # Errors
    ///
    /// - [`ResolveError::Unknown`] — no command matched `input` exactly or as
    ///   a prefix. The `suggestions` field contains up to three canonical names
    ///   whose edit distance from `input` is ≤ 2, or which contain `input` as
    ///   a substring. May be empty if no close matches exist.
    /// - [`ResolveError::Ambiguous`] — `input` is a prefix of more than one
    ///   command; the `candidates` field lists their canonical names.
    ///
    /// # Examples
    ///
    /// ```
    /// # use argot::{Command, Resolver, ResolveError};
    /// let cmds = vec![Command::builder("get").build().unwrap()];
    /// let resolver = Resolver::new(&cmds);
    ///
    /// assert_eq!(resolver.resolve("get").unwrap().canonical, "get");
    /// assert_eq!(resolver.resolve("GET").unwrap().canonical, "get"); // case-insensitive
    /// assert!(matches!(resolver.resolve("xyz"), Err(ResolveError::Unknown { .. })));
    /// ```
    pub fn resolve(&self, input: &str) -> Result<&'a Command, ResolveError> {
        let normalized = input.trim().to_lowercase();

        if normalized.is_empty() {
            return Err(ResolveError::Unknown {
                input: input.to_string(),
                suggestions: vec![],
            });
        }

        // 1. Exact match
        for cmd in self.commands {
            if cmd.matchable_strings().contains(&normalized) {
                return Ok(cmd);
            }
        }

        // 2. Prefix match
        let matches: Vec<&'a Command> = self
            .commands
            .iter()
            .filter(|cmd| {
                cmd.matchable_strings()
                    .iter()
                    .any(|s| s.starts_with(&normalized))
            })
            .collect();

        match matches.len() {
            0 => {
                // Compute "did you mean?" suggestions — canonical names within
                // edit distance 2 or containing the normalized input as a substring.
                let mut suggestions: Vec<(String, usize)> = self
                    .commands
                    .iter()
                    .filter_map(|cmd| {
                        let dist = edit_distance(&normalized, &cmd.canonical.to_lowercase());
                        if dist <= 2 || cmd.canonical.to_lowercase().contains(&normalized) {
                            Some((cmd.canonical.clone(), dist))
                        } else {
                            None
                        }
                    })
                    .collect();
                suggestions.sort_by_key(|(_, d)| *d);
                let suggestions: Vec<String> =
                    suggestions.into_iter().take(3).map(|(s, _)| s).collect();
                Err(ResolveError::Unknown {
                    input: input.to_string(),
                    suggestions,
                })
            }
            1 => Ok(matches[0]),
            _ => Err(ResolveError::Ambiguous {
                input: input.to_string(),
                candidates: matches.iter().map(|c| c.canonical.clone()).collect(),
            }),
        }
    }
}

/// Compute the Levenshtein edit distance between two strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (la, lb) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; lb + 1]; la + 1];
    for (i, row) in dp.iter_mut().enumerate().take(la + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(lb + 1) {
        *cell = j;
    }
    for i in 1..=la {
        for j in 1..=lb {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j - 1].min(dp[i - 1][j]).min(dp[i][j - 1])
            };
        }
    }
    dp[la][lb]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Command;

    fn cmds() -> Vec<Command> {
        vec![
            Command::builder("list")
                .alias("ls")
                .spelling("LIST")
                .build()
                .unwrap(),
            Command::builder("log").build().unwrap(),
            Command::builder("get").build().unwrap(),
        ]
    }

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected_canonical: Option<&'static str>,
        expect_ambiguous: bool,
        expect_unknown: bool,
    }

    #[test]
    fn test_resolve() {
        let commands = cmds();
        let resolver = Resolver::new(&commands);

        let cases = vec![
            TestCase {
                name: "exact canonical",
                input: "list",
                expected_canonical: Some("list"),
                expect_ambiguous: false,
                expect_unknown: false,
            },
            TestCase {
                name: "exact alias",
                input: "ls",
                expected_canonical: Some("list"),
                expect_ambiguous: false,
                expect_unknown: false,
            },
            TestCase {
                name: "exact spelling (uppercase normalized)",
                input: "LIST",
                expected_canonical: Some("list"),
                expect_ambiguous: false,
                expect_unknown: false,
            },
            TestCase {
                name: "case insensitive canonical",
                input: "GET",
                expected_canonical: Some("get"),
                expect_ambiguous: false,
                expect_unknown: false,
            },
            TestCase {
                name: "unambiguous prefix",
                input: "ge",
                expected_canonical: Some("get"),
                expect_ambiguous: false,
                expect_unknown: false,
            },
            TestCase {
                name: "ambiguous prefix (list + log share 'l')",
                input: "l",
                expected_canonical: None,
                expect_ambiguous: true,
                expect_unknown: false,
            },
            TestCase {
                name: "unknown",
                input: "xyz",
                expected_canonical: None,
                expect_ambiguous: false,
                expect_unknown: true,
            },
            TestCase {
                name: "empty input unknown",
                input: "",
                expected_canonical: None,
                expect_ambiguous: false,
                expect_unknown: true,
            },
        ];

        for tc in &cases {
            let result = resolver.resolve(tc.input);
            match result {
                Ok(cmd) => {
                    assert!(
                        tc.expected_canonical.is_some(),
                        "case '{}': expected error but got Ok({})",
                        tc.name,
                        cmd.canonical
                    );
                    assert_eq!(
                        cmd.canonical,
                        tc.expected_canonical.unwrap(),
                        "case '{}'",
                        tc.name
                    );
                }
                Err(ResolveError::Ambiguous { .. }) => {
                    assert!(
                        tc.expect_ambiguous,
                        "case '{}': unexpected Ambiguous",
                        tc.name
                    );
                }
                Err(ResolveError::Unknown { .. }) => {
                    assert!(tc.expect_unknown, "case '{}': unexpected Unknown", tc.name);
                }
            }
        }
    }

    #[test]
    fn test_ambiguous_candidates_are_canonicals() {
        let commands = cmds();
        let resolver = Resolver::new(&commands);
        match resolver.resolve("l") {
            Err(ResolveError::Ambiguous { candidates, .. }) => {
                assert!(candidates.contains(&"list".to_string()));
                assert!(candidates.contains(&"log".to_string()));
            }
            other => panic!("expected Ambiguous, got {:?}", other),
        }
    }

    #[test]
    fn test_unknown_with_suggestions() {
        let commands = cmds(); // list / log / get
        let resolver = Resolver::new(&commands);
        // "lust" is close to "list" (edit distance 1 after normalization)
        match resolver.resolve("lust") {
            Err(ResolveError::Unknown { suggestions, .. }) => {
                assert!(
                    suggestions.contains(&"list".to_string()),
                    "expected 'list' in suggestions, got {:?}",
                    suggestions
                );
            }
            other => panic!("expected Unknown, got {:?}", other),
        }
    }

    #[test]
    fn test_unknown_no_suggestions_for_gibberish() {
        let commands = cmds();
        let resolver = Resolver::new(&commands);
        match resolver.resolve("xyzzy") {
            Err(ResolveError::Unknown { suggestions, .. }) => {
                assert!(
                    suggestions.is_empty(),
                    "expected no suggestions for gibberish, got {:?}",
                    suggestions
                );
            }
            other => panic!("expected Unknown, got {:?}", other),
        }
    }
}
