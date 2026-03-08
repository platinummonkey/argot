mod tokenizer;

use std::collections::HashMap;

use thiserror::Error;

use crate::model::{Command, ParsedCommand};
use crate::resolver::{ResolveError, Resolver};

use tokenizer::{tokenize, Token};

/// Errors produced by [`Parser::parse`].
#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("no command provided")]
    NoCommand,
    #[error(transparent)]
    Resolve(#[from] ResolveError),
    #[error("missing required argument: {0}")]
    MissingArgument(String),
    #[error("unexpected argument: {0}")]
    UnexpectedArgument(String),
    #[error("missing required flag: --{0}")]
    MissingFlag(String),
    #[error("flag --{name} requires a value")]
    FlagMissingValue { name: String },
    #[error("unknown flag: {0}")]
    UnknownFlag(String),
}

/// Parses raw argument slices against a slice of registered [`Command`]s.
pub struct Parser<'a> {
    commands: &'a [Command],
}

impl<'a> Parser<'a> {
    pub fn new(commands: &'a [Command]) -> Self {
        Self { commands }
    }

    /// Parse `argv` (the full argument list including the command name) into a
    /// [`ParsedCommand`] that borrows from the registered command tree.
    pub fn parse(&self, argv: &[&str]) -> Result<ParsedCommand<'a>, ParseError> {
        let tokens = tokenize(argv);
        let mut pos = 0;

        // First token must be a Word naming the top-level command.
        let cmd_name = match tokens.get(pos) {
            Some(Token::Word(w)) => {
                pos += 1;
                w.clone()
            }
            _ => return Err(ParseError::NoCommand),
        };

        let resolver = Resolver::new(self.commands);
        let mut cmd: &'a Command = resolver.resolve(&cmd_name)?;

        // Walk the subcommand tree while the next token is a Word that resolves.
        loop {
            if cmd.subcommands.is_empty() {
                break;
            }
            match tokens.get(pos) {
                Some(Token::Word(w)) => {
                    let sub_resolver = Resolver::new(&cmd.subcommands);
                    match sub_resolver.resolve(w) {
                        Ok(sub) => {
                            cmd = sub;
                            pos += 1;
                        }
                        Err(_) => break, // treat as positional argument
                    }
                }
                _ => break,
            }
        }

        // Process remaining tokens: flags and positional arguments.
        let mut positionals: Vec<String> = Vec::new();
        let mut flags: HashMap<String, String> = HashMap::new();

        while pos < tokens.len() {
            match &tokens[pos] {
                Token::Separator => {
                    // Everything after -- is a positional word (tokenizer already
                    // converts post-separator args to Token::Word, so this is a
                    // no-op guard for the separator token itself).
                    pos += 1;
                }
                Token::Word(w) => {
                    positionals.push(w.clone());
                    pos += 1;
                }
                Token::LongFlag { name, value } => {
                    let flag_def = cmd
                        .flags
                        .iter()
                        .find(|f| &f.name == name)
                        .ok_or_else(|| ParseError::UnknownFlag(format!("--{}", name)))?;

                    let val = if flag_def.takes_value {
                        if let Some(v) = value {
                            v.clone()
                        } else {
                            pos += 1;
                            match tokens.get(pos) {
                                Some(Token::Word(w)) => w.clone(),
                                _ => {
                                    return Err(ParseError::FlagMissingValue {
                                        name: name.clone(),
                                    })
                                }
                            }
                        }
                    } else {
                        "true".to_string()
                    };

                    flags.insert(flag_def.name.clone(), val);
                    pos += 1;
                }
                Token::ShortFlag { name: c, value } => {
                    let flag_def = cmd
                        .flags
                        .iter()
                        .find(|f| f.short == Some(*c))
                        .ok_or_else(|| ParseError::UnknownFlag(format!("-{}", c)))?;

                    let val = if flag_def.takes_value {
                        if let Some(v) = value {
                            v.clone()
                        } else {
                            pos += 1;
                            match tokens.get(pos) {
                                Some(Token::Word(w)) => w.clone(),
                                _ => {
                                    return Err(ParseError::FlagMissingValue {
                                        name: flag_def.name.clone(),
                                    })
                                }
                            }
                        }
                    } else {
                        "true".to_string()
                    };

                    flags.insert(flag_def.name.clone(), val);
                    pos += 1;
                }
            }
        }

        // Bind positional arguments to declared argument slots.
        let mut args: HashMap<String, String> = HashMap::new();
        for (i, arg_def) in cmd.arguments.iter().enumerate() {
            if let Some(val) = positionals.get(i) {
                args.insert(arg_def.name.clone(), val.clone());
            } else if arg_def.required {
                return Err(ParseError::MissingArgument(arg_def.name.clone()));
            } else if let Some(default) = &arg_def.default {
                args.insert(arg_def.name.clone(), default.clone());
            }
        }

        if positionals.len() > cmd.arguments.len() {
            return Err(ParseError::UnexpectedArgument(
                positionals[cmd.arguments.len()].clone(),
            ));
        }

        // Validate required flags; apply defaults.
        for flag_def in &cmd.flags {
            if flag_def.required && !flags.contains_key(&flag_def.name) {
                return Err(ParseError::MissingFlag(flag_def.name.clone()));
            }
            if !flags.contains_key(&flag_def.name) {
                if let Some(default) = &flag_def.default {
                    flags.insert(flag_def.name.clone(), default.clone());
                }
            }
        }

        Ok(ParsedCommand { command: cmd, args, flags })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Argument, Command, Example, Flag};

    fn build_commands() -> Vec<Command> {
        let remote_add = Command::builder("add")
            .summary("Add a remote")
            .argument(
                Argument::builder("name")
                    .description("remote name")
                    .required()
                    .build()
                    .unwrap(),
            )
            .argument(
                Argument::builder("url")
                    .description("remote url")
                    .required()
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        let remote_remove = Command::builder("remove")
            .alias("rm")
            .summary("Remove a remote")
            .argument(
                Argument::builder("name")
                    .description("remote name")
                    .required()
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        let remote = Command::builder("remote")
            .summary("Manage remotes")
            .subcommand(remote_add)
            .subcommand(remote_remove)
            .build()
            .unwrap();

        let list = Command::builder("list")
            .alias("ls")
            .summary("List items")
            .argument(
                Argument::builder("filter")
                    .description("optional filter")
                    .build()
                    .unwrap(),
            )
            .flag(
                Flag::builder("verbose")
                    .short('v')
                    .description("verbose output")
                    .build()
                    .unwrap(),
            )
            .flag(
                Flag::builder("output")
                    .short('o')
                    .description("output format")
                    .takes_value()
                    .default_value("text")
                    .build()
                    .unwrap(),
            )
            .example(Example::new("list all", "myapp list"))
            .build()
            .unwrap();

        let deploy = Command::builder("deploy")
            .summary("Deploy")
            .flag(
                Flag::builder("env")
                    .description("target environment")
                    .takes_value()
                    .required()
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        vec![list, remote, deploy]
    }

    struct TestCase {
        name: &'static str,
        argv: &'static [&'static str],
        expect_err: bool,
        expected_canonical: Option<&'static str>,
        expected_args: Vec<(&'static str, &'static str)>,
        expected_flags: Vec<(&'static str, &'static str)>,
    }

    #[test]
    fn test_parse() {
        let commands = build_commands();
        let parser = Parser::new(&commands);

        let cases = vec![
            TestCase {
                name: "flat command no args",
                argv: &["list"],
                expect_err: false,
                expected_canonical: Some("list"),
                expected_args: vec![],
                expected_flags: vec![("output", "text")],
            },
            TestCase {
                name: "flat command with positional",
                argv: &["list", "foo"],
                expect_err: false,
                expected_canonical: Some("list"),
                expected_args: vec![("filter", "foo")],
                expected_flags: vec![("output", "text")],
            },
            TestCase {
                name: "alias resolved",
                argv: &["ls"],
                expect_err: false,
                expected_canonical: Some("list"),
                expected_args: vec![],
                expected_flags: vec![("output", "text")],
            },
            TestCase {
                name: "boolean flag short",
                argv: &["list", "-v"],
                expect_err: false,
                expected_canonical: Some("list"),
                expected_args: vec![],
                expected_flags: vec![("verbose", "true"), ("output", "text")],
            },
            TestCase {
                name: "long flag equals",
                argv: &["list", "--output=json"],
                expect_err: false,
                expected_canonical: Some("list"),
                expected_args: vec![],
                expected_flags: vec![("output", "json")],
            },
            TestCase {
                name: "long flag space value",
                argv: &["list", "--output", "json"],
                expect_err: false,
                expected_canonical: Some("list"),
                expected_args: vec![],
                expected_flags: vec![("output", "json")],
            },
            TestCase {
                name: "short flag space value",
                argv: &["list", "-o", "json"],
                expect_err: false,
                expected_canonical: Some("list"),
                expected_args: vec![],
                expected_flags: vec![("output", "json")],
            },
            TestCase {
                name: "two-level subcommand",
                argv: &["remote", "add", "origin", "https://example.com"],
                expect_err: false,
                expected_canonical: Some("add"),
                expected_args: vec![("name", "origin"), ("url", "https://example.com")],
                expected_flags: vec![],
            },
            TestCase {
                name: "subcommand alias",
                argv: &["remote", "rm", "origin"],
                expect_err: false,
                expected_canonical: Some("remove"),
                expected_args: vec![("name", "origin")],
                expected_flags: vec![],
            },
            TestCase {
                name: "no command",
                argv: &[],
                expect_err: true,
                expected_canonical: None,
                expected_args: vec![],
                expected_flags: vec![],
            },
            TestCase {
                name: "unknown command",
                argv: &["unknown"],
                expect_err: true,
                expected_canonical: None,
                expected_args: vec![],
                expected_flags: vec![],
            },
            TestCase {
                name: "unknown flag",
                argv: &["list", "--nope"],
                expect_err: true,
                expected_canonical: None,
                expected_args: vec![],
                expected_flags: vec![],
            },
            TestCase {
                name: "missing required flag",
                argv: &["deploy"],
                expect_err: true,
                expected_canonical: None,
                expected_args: vec![],
                expected_flags: vec![],
            },
            TestCase {
                name: "unexpected positional",
                argv: &["list", "one", "two"],
                expect_err: true,
                expected_canonical: None,
                expected_args: vec![],
                expected_flags: vec![],
            },
        ];

        for tc in &cases {
            let result = parser.parse(tc.argv);
            if tc.expect_err {
                assert!(result.is_err(), "case '{}': expected error", tc.name);
            } else {
                let parsed = result.unwrap_or_else(|e| panic!("case '{}': unexpected error: {}", tc.name, e));
                assert_eq!(
                    parsed.command.canonical,
                    tc.expected_canonical.unwrap(),
                    "case '{}'",
                    tc.name
                );
                for (k, v) in &tc.expected_args {
                    assert_eq!(
                        parsed.args.get(*k).map(String::as_str),
                        Some(*v),
                        "case '{}': arg {}",
                        tc.name,
                        k
                    );
                }
                for (k, v) in &tc.expected_flags {
                    assert_eq!(
                        parsed.flags.get(*k).map(String::as_str),
                        Some(*v),
                        "case '{}': flag {}",
                        tc.name,
                        k
                    );
                }
            }
        }
    }

    #[test]
    fn test_double_dash_separator() {
        let cmds = vec![Command::builder("run")
            .argument(
                Argument::builder("script")
                    .description("script to run")
                    .required()
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()];
        let parser = Parser::new(&cmds);
        // "--" separator should make "--not-a-flag" treated as a positional word.
        // But our command only has one argument, so the second word would be unexpected.
        // Let's just verify `--` itself doesn't cause a parse error on the separator.
        let result = parser.parse(&["run", "--", "myscript"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().args["script"], "myscript");
    }

    #[test]
    fn test_missing_required_argument() {
        let cmds = vec![Command::builder("get")
            .argument(
                Argument::builder("id")
                    .description("item id")
                    .required()
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()];
        let parser = Parser::new(&cmds);
        assert!(matches!(
            parser.parse(&["get"]),
            Err(ParseError::MissingArgument(ref s)) if s == "id"
        ));
    }

    #[test]
    fn test_flag_missing_value() {
        let cmds = vec![Command::builder("build")
            .flag(
                Flag::builder("target")
                    .takes_value()
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()];
        let parser = Parser::new(&cmds);
        assert!(matches!(
            parser.parse(&["build", "--target"]),
            Err(ParseError::FlagMissingValue { .. })
        ));
    }
}
