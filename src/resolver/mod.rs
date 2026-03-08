use thiserror::Error;

use crate::model::Command;

/// Errors produced by [`Resolver::resolve`].
#[derive(Debug, Error, PartialEq)]
pub enum ResolveError {
    #[error("unknown command: {0}")]
    Unknown(String),
    #[error("ambiguous command \"{input}\": could match {candidates:?}")]
    Ambiguous {
        input: String,
        candidates: Vec<String>,
    },
}

/// Resolves a string token to a [`Command`] in a slice, supporting aliases,
/// spellings, and unambiguous prefix matching.
pub struct Resolver<'a> {
    commands: &'a [Command],
}

impl<'a> Resolver<'a> {
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
    pub fn resolve(&self, input: &str) -> Result<&'a Command, ResolveError> {
        let normalized = input.trim().to_lowercase();

        if normalized.is_empty() {
            return Err(ResolveError::Unknown(input.to_string()));
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
            0 => Err(ResolveError::Unknown(input.to_string())),
            1 => Ok(matches[0]),
            _ => Err(ResolveError::Ambiguous {
                input: input.to_string(),
                candidates: matches.iter().map(|c| c.canonical.clone()).collect(),
            }),
        }
    }
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
                Err(ResolveError::Unknown(_)) => {
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
}
